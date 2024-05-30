use subprocess::{CaptureData, Exec, ExitStatus};

use crate::{env, evaluator::FlattenedCmdline};

pub enum BuiltinCheck {
    Yes(Builtin),
    No(FlattenedCmdline),
}

pub struct Builtin {

    cmd: FlattenedCmdline,
    next: Vec<Exec>,

}

impl Builtin {
    pub fn maybe_new(cmd: FlattenedCmdline) -> BuiltinCheck {
        todo!()
    }

    pub fn execute(self) -> ExitStatus {
        todo!()
    }

    pub fn capture(self) -> CaptureData {
        todo!()
    }

    pub fn pipe(self, into: Exec) -> Builtin {
        todo!()
    }
}

#[inline]
fn exit_with_error(code: u32, msg: String) -> CaptureData {
    CaptureData {
        stdout: Vec::new(),
        stderr: msg.into_bytes(),
        exit_status: ExitStatus::Exited(code),
    }
}

#[inline]
fn exit_quiet_success() -> CaptureData {
    CaptureData {
        stdout: Vec::new(),
        stderr: Vec::new(),
        exit_status: ExitStatus::Exited(0),
    }
}

fn builtin_cd(cmd: FlattenedCmdline) -> CaptureData {

    match cmd.arguments.len() {
        0 => {
            let home = env::get("HOME");
            if home.len() == 0 {
                return exit_with_error(1, "$HOME variable not set".to_owned());
            }
            if let Err(e) = std::env::set_current_dir(home) {
                return exit_with_error(1, format!("unable to cd to $HOME: {}", e));
            }
        },
        1 => {}
        _ => return exit_with_error(1, "Too many arguments for cd".to_owned()),
    };

    exit_quiet_success()
}
