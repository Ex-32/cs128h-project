use std::{
    collections::VecDeque, ffi::{OsStr, OsString}, fs::{self, File}, io, string::FromUtf8Error
};

use subprocess::{CaptureData, Exec, ExitStatus, Pipeline, PopenError};

use crate::{
    ast::{RedirectFd, RedirectOp, Separator},
    builtins::Builtin,
    env,
    evaluator::FlattenedCmdline,
};

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum ProcError {
    /// this error indicates that a redirect could not be successfully created this could be
    /// because you're trying to do a stdin error from a file that does not exist, but it could
    /// also indicate that an output redirect file is not writable or could not be created.
    #[error("unable to redirecd '{path}': {internal}")]
    RedirectError { path: String, internal: io::Error },

    #[error("invalid mixture of type and file descriptors")]
    InvalidRedirect { op: RedirectOp },

    #[error("error while creating subprocess: {internal}")]
    SubprocessError { internal: PopenError },

    #[error("text output of shell substitution not valid UTF-8")]
    SubstitutionError { internal: FromUtf8Error },

    #[error("final command in seqence had dangling pipe")]
    DanglingPipe,

    #[error("feature '{feature}' has not yet been implemented")]
    NotImplemented { feature: &'static str },
}

#[derive(Debug)]
pub struct ProcManager {}

impl ProcManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn dispatch(&mut self, cmd: FlattenedCmdline) -> Result<ExitStatus, ProcError> {
        let execs = self.build_exec(cmd)?;
        let mut exit = ExitStatus::Undetermined;

        for exec in execs {
            exit = exec.join()?;
        }

        Ok(exit)
    }

    pub fn dispatch_capture(
        &mut self,
        cmd: FlattenedCmdline,
    ) -> Result<(ExitStatus, OsString), ProcError> {
        let execs = self.build_exec(cmd)?;
        let mut buf = Vec::new();
        let mut exit = ExitStatus::Undetermined;

        for exec in execs {
            let cap = exec.capture()?;
            exit = cap.exit_status;
            buf.extend(cap.stdout);
        }

        let output =
            String::from_utf8(buf).map_err(|e| ProcError::SubstitutionError { internal: e })?;
        Ok((exit, output.into()))
    }

    fn build_exec(&self, cmd: FlattenedCmdline) -> Result<Vec<Execable>, ProcError> {
        let execs = self.build_exec_internal(cmd)?;

        let mut ret: Vec<Execable> = Vec::new();

        for (sep, exec) in execs {
            match sep {
                Separator::Semicolon => ret.push(exec.into()),
                Separator::Pipe => match ret.pop() {
                    Some(x) => ret.push(x.pipe(exec)),
                    None => return Err(ProcError::DanglingPipe),
                },
                Separator::Fork => return Err(ProcError::NotImplemented { feature: "fork" }),
            }
        }

        Ok(ret)
    }

    fn build_exec_internal(
        &self,
        cmd: FlattenedCmdline,
    ) -> Result<VecDeque<(Separator, Exec)>, ProcError> {
        let mut exec = subprocess::Exec::cmd(cmd.command)
            .args(cmd.arguments.as_slice())
            .env_extend(env::pairs().as_slice())
            .env_extend(cmd.envs.as_slice());

        for (op, path) in cmd.redirects.into_iter() {
            match op.r#type {
                crate::ast::RedirectType::Out => match op.fd {
                    RedirectFd::All => {
                        let file = file_write(&path)?;
                        let file2 = file.try_clone().map_err(|e| ProcError::RedirectError {
                            path: path.to_string_lossy().to_string(),
                            internal: e,
                        })?;

                        exec = exec.stdout(file).stderr(file2)
                    }
                    RedirectFd::Default | RedirectFd::Stdout => {
                        let file = file_write(&path)?;

                        exec = exec.stdout(file)
                    }
                    RedirectFd::Stdin => return Err(ProcError::InvalidRedirect { op }),
                    RedirectFd::Stderr => {
                        let file = file_write(&path)?;

                        exec = exec.stderr(file)
                    }
                },
                crate::ast::RedirectType::OutAppend => match op.fd {
                    RedirectFd::All => {
                        let file = file_append(&path)?;
                        let file2 = file.try_clone().map_err(|e| ProcError::RedirectError {
                            path: path.to_string_lossy().to_string(),
                            internal: e,
                        })?;

                        exec = exec.stdout(file).stderr(file2)
                    }
                    RedirectFd::Default | RedirectFd::Stdout => {
                        let file = file_append(&path)?;

                        exec = exec.stdout(file)
                    }
                    RedirectFd::Stdin => return Err(ProcError::InvalidRedirect { op }),
                    RedirectFd::Stderr => {
                        let file = file_append(&path)?;

                        exec = exec.stderr(file)
                    }
                },
                crate::ast::RedirectType::In => match op.fd {
                    RedirectFd::All => return Err(ProcError::InvalidRedirect { op }),
                    RedirectFd::Default | RedirectFd::Stdin => {
                        let file = file_read(&path)?;

                        exec = exec.stdin(file)
                    }
                    RedirectFd::Stdout => return Err(ProcError::InvalidRedirect { op }),
                    RedirectFd::Stderr => return Err(ProcError::InvalidRedirect { op }),
                },
            };
        }

        match cmd.next {
            Some((separator, next_cmd)) => {
                let mut seq = self.build_exec_internal(*next_cmd)?;
                seq.front_mut()
                    .expect("build_exec() must return non empty collection")
                    .0 = separator;
                seq.push_front((Separator::Semicolon, exec));
                Ok(seq)
            }
            None => Ok(VecDeque::from([(Separator::Semicolon, exec)])),
        }
    }
}

fn file_read(path: &OsStr) -> Result<File, ProcError> {
    fs::File::options()
        .read(true)
        .write(false)
        .open(&path)
        .map_err(|e| ProcError::RedirectError {
            path: path.to_string_lossy().to_string(),
            internal: e,
        })
}
fn file_write(path: &OsStr) -> Result<File, ProcError> {
    fs::File::options()
        .create(true)
        .write(true)
        .append(false)
        .open(&path)
        .map_err(|e| ProcError::RedirectError {
            path: path.to_string_lossy().to_string(),
            internal: e,
        })
}
fn file_append(path: &OsStr) -> Result<File, ProcError> {
    fs::File::options()
        .create(true)
        .write(true)
        .append(true)
        .open(&path)
        .map_err(|e| ProcError::RedirectError {
            path: path.to_string_lossy().to_string(),
            internal: e,
        })
}

pub enum Execable {
    Exec(Exec),
    Pipeline(Pipeline),
    Builtin(Builtin),
}

impl Execable {
    fn join(self) -> Result<ExitStatus, ProcError> {
        match self {
            Execable::Exec(x) => x
                .join()
                .map_err(|e| ProcError::SubprocessError { internal: e }),
            Execable::Pipeline(x) => x
                .join()
                .map_err(|e| ProcError::SubprocessError { internal: e }),
            Execable::Builtin(x) => Ok(x.execute()),
        }
    }

    fn capture(self) -> Result<CaptureData, ProcError> {
        match self {
            Execable::Exec(x) => x
                .capture()
                .map_err(|e| ProcError::SubprocessError { internal: e }),
            Execable::Pipeline(x) => x
                .capture()
                .map_err(|e| ProcError::SubprocessError { internal: e }),
            Execable::Builtin(x) => Ok(x.capture()),
        }
    }

    fn pipe(self, into: Exec) -> Execable {
        match self {
            Execable::Exec(x) => (x | into).into(),
            Execable::Pipeline(x) => (x | into).into(),
            Execable::Builtin(x) => x.pipe(into).into(),
        }
    }
}

impl From<Exec> for Execable {
    fn from(value: Exec) -> Self {
        Self::Exec(value)
    }
}

impl From<Pipeline> for Execable {
    fn from(value: Pipeline) -> Self {
        Self::Pipeline(value)
    }
}

impl From<Builtin> for Execable {
    fn from(value: Builtin) -> Self {
        Self::Builtin(value)
    }
}
