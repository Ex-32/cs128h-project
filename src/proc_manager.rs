use std::{fs, io, string::FromUtf8Error};

use subprocess::{Exec, ExitStatus, PopenError};

use crate::{
    ast::{RedirectFd, RedirectOp},
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
}

#[derive(Debug)]
pub struct ProcManager {}

impl ProcManager {
    pub fn new() -> Self {
        Self {}
    }

    fn build_exec(&mut self, cmd: FlattenedCmdline) -> Result<Exec, ProcError> {
        let mut exec = subprocess::Exec::cmd(cmd.command)
            .args(cmd.arguments.as_slice())
            .env_extend(cmd.envs.as_slice());

        for (op, path) in cmd.redirects.into_iter() {
            match op.r#type {
                crate::ast::RedirectType::Out => match op.fd {
                    RedirectFd::All => {
                        let file = fs::File::options()
                            .create(true)
                            .write(true)
                            .append(false)
                            .open(&path)
                            .map_err(|e| ProcError::RedirectError {
                                path: path.clone(),
                                internal: e,
                            })?;

                        let file2 = file.try_clone().map_err(|e| ProcError::RedirectError {
                            path: path.clone(),
                            internal: e,
                        })?;

                        exec = exec.stdout(file).stderr(file2)
                    }
                    RedirectFd::Default | RedirectFd::Stdout => {
                        let file = fs::File::options()
                            .create(true)
                            .write(true)
                            .append(false)
                            .open(&path)
                            .map_err(|e| ProcError::RedirectError {
                                path: path.clone(),
                                internal: e,
                            })?;

                        exec = exec.stdout(file)
                    }
                    RedirectFd::Stdin => return Err(ProcError::InvalidRedirect { op }),
                    RedirectFd::Stderr => {
                        let file = fs::File::options()
                            .create(true)
                            .write(true)
                            .append(false)
                            .open(&path)
                            .map_err(|e| ProcError::RedirectError {
                                path: path.clone(),
                                internal: e,
                            })?;

                        exec = exec.stderr(file)
                    }
                },
                crate::ast::RedirectType::OutAppend => match op.fd {
                    RedirectFd::All => {
                        let file = fs::File::options()
                            .create(true)
                            .write(true)
                            .append(true)
                            .open(&path)
                            .map_err(|e| ProcError::RedirectError {
                                path: path.clone(),
                                internal: e,
                            })?;

                        let file2 = file.try_clone().map_err(|e| ProcError::RedirectError {
                            path: path.clone(),
                            internal: e,
                        })?;

                        exec = exec.stdout(file).stderr(file2)
                    }
                    RedirectFd::Default | RedirectFd::Stdout => {
                        let file = fs::File::options()
                            .create(true)
                            .write(true)
                            .append(true)
                            .open(&path)
                            .map_err(|e| ProcError::RedirectError {
                                path: path.clone(),
                                internal: e,
                            })?;

                        exec = exec.stdout(file)
                    }
                    RedirectFd::Stdin => return Err(ProcError::InvalidRedirect { op }),
                    RedirectFd::Stderr => {
                        let file = fs::File::options()
                            .create(true)
                            .write(true)
                            .append(true)
                            .open(&path)
                            .map_err(|e| ProcError::RedirectError {
                                path: path.clone(),
                                internal: e,
                            })?;

                        exec = exec.stderr(file)
                    }
                },
                crate::ast::RedirectType::In => match op.fd {
                    RedirectFd::All => return Err(ProcError::InvalidRedirect { op }),
                    RedirectFd::Default | RedirectFd::Stdin => {
                        let file = fs::File::options()
                            .read(true)
                            .write(false)
                            .open(&path)
                            .map_err(|e| ProcError::RedirectError {
                                path: path.clone(),
                                internal: e,
                            })?;

                        exec = exec.stdin(file)
                    }
                    RedirectFd::Stdout => return Err(ProcError::InvalidRedirect { op }),
                    RedirectFd::Stderr => return Err(ProcError::InvalidRedirect { op }),
                },
            };
        }

        if let Some(_) = cmd.next {
            log::error!("command chaining not implemented, ignoring");
        }

        return Ok(exec);
    }

    pub fn dispatch(&mut self, cmd: FlattenedCmdline) -> Result<ExitStatus, ProcError> {
        let exec = self.build_exec(cmd)?;
        exec.join()
            .map_err(|e| ProcError::SubprocessError { internal: e })
    }

    pub fn dispatch_sub(
        &mut self,
        cmd: FlattenedCmdline,
    ) -> Result<(ExitStatus, String), ProcError> {
        let exec = self.build_exec(cmd)?;
        let cap = exec
            .capture()
            .map_err(|e| ProcError::SubprocessError { internal: e })?;
        let output = String::from_utf8(cap.stdout)
            .map_err(|e| ProcError::SubstitutionError { internal: e })?;
        Ok((cap.exit_status, output))
    }
}
