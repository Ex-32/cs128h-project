use std::ffi::OsString;

use subprocess::ExitStatus;

use crate::{
    ast::*,
    env,
    proc_manager::{ProcError, ProcManager},
};

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum EvalError {
    #[error("enviroment variable {name} is not valud utf-8: {value}")]
    InvalidEnvValue {
        /// the name of the environment variable
        name: String,
        /// the result of calling `.to_string_lossy()` on the invalid value, should only be used
        /// for printing the malformed value as invalid sections are replaced with the unicode
        /// replacement character
        value: String,
    },

    #[error("evaluator recived error attempting to dispath command: {internal}")]
    DispatchError { internal: ProcError },
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Evaluator {
    proc_manager: ProcManager,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlattenedCmdline {
    pub envs: Vec<(OsString, OsString)>,
    pub command: OsString,
    pub arguments: Vec<OsString>,
    pub redirects: Vec<(RedirectOp, OsString)>,
    pub next: Option<(Separator, Box<FlattenedCmdline>)>,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            proc_manager: ProcManager::new(),
        }
    }

    pub fn eval(&mut self, ast: Main) -> Result<ExitStatus, EvalError> {
        let flattened = self.flatten_commandline(ast.0)?;
        match self.proc_manager.dispatch(flattened) {
            Ok(x) => Ok(x),
            Err(e) => Err(EvalError::DispatchError { internal: e }),
        }
    }

    fn flatten_commandline(&mut self, cmdline: CommandLine) -> Result<FlattenedCmdline, EvalError> {
        let envs = cmdline
            .envs
            .into_iter()
            .map(|x| self.flatten_command_env(x))
            .collect::<Result<Vec<_>, EvalError>>()?;
        let command = self.flatten_command(cmdline.command)?;
        let arguments = cmdline
            .arguments
            .into_iter()
            .map(|x| self.flatten_argument(x))
            .collect::<Result<Vec<_>, EvalError>>()?;
        let redirects = cmdline
            .redirects
            .into_iter()
            .map(|x| self.flatten_redirection(x))
            .collect::<Result<Vec<_>, EvalError>>()?;
        let next = match cmdline.next {
            Some((sep, next)) => Some((sep, Box::new(self.flatten_commandline(*next)?))),
            None => None,
        };

        Ok(FlattenedCmdline {
            envs,
            command,
            arguments,
            redirects,
            next,
        })
    }

    fn flatten_argument(&mut self, arg: Argument) -> Result<OsString, EvalError> {
        match arg {
            Argument::ShellSubstitution(x) => self.flatten_shell_substitution(x),
            Argument::StringLiteral(x) => self.flatten_string_literal(x),
            Argument::SingleQuoteString(x) => self.flatten_single_string(x),
            Argument::DoubleQuoteString(x) => self.flatten_double_string(x),
        }
    }

    fn flatten_command(&mut self, cmd: Command) -> Result<OsString, EvalError> {
        match cmd {
            Command::StringLiteral(x) => self.flatten_string_literal(x),
            Command::SingleQuoteString(x) => self.flatten_single_string(x),
            Command::DoubleQuoteString(x) => self.flatten_double_string(x),
        }
    }

    #[inline]
    fn flatten_redirection(&mut self, red: Redirection) -> Result<(RedirectOp, OsString), EvalError> {
        Ok((red.op, self.flatten_argument(red.arg)?))
    }

    #[inline]
    fn flatten_command_env(&mut self, env: CommandEnv) -> Result<(OsString, OsString), EvalError> {
        Ok((
            self.flatten_env_litteral(env.name)?,
            self.flatten_argument(env.value)?,
        ))
    }

    #[inline]
    fn flatten_env_litteral(&self, env: EnvLiteral) -> Result<OsString, EvalError> {
        Ok(env.0)
    }

    fn flatten_double_string(&mut self, string: DoubleQuoteString) -> Result<OsString, EvalError> {
        Ok(string
            .0
            .into_iter()
            .map(|x| self.flatten_double_string_component(x))
            .collect::<Result<Vec<_>, EvalError>>()?
            .into_iter()
            .fold(OsString::new(), |mut acc, x| {
                acc.push(&x);
                acc
            }))
    }

    #[inline]
    fn flatten_single_string(&self, string: SingleQuoteString) -> Result<OsString, EvalError> {
        Ok(string.0)
    }

    fn flatten_string_literal(&mut self, string: StringLiteral) -> Result<OsString, EvalError> {
        Ok(string
            .0
            .into_iter()
            .map(|x| self.flatten_string_linteral_component(x))
            .fold(OsString::new(), |mut acc, x| {
                acc.push(&x);
                acc
            }))
    }

    fn flatten_double_string_component(
        &mut self,
        component: DoubleQuoteComponent,
    ) -> Result<OsString, EvalError> {
        match component {
            DoubleQuoteComponent::Chars(x) => Ok(x.0),
            DoubleQuoteComponent::DollarEnv(x) => Ok(self.flatten_dollar_env(x)),
            DoubleQuoteComponent::DollarShell(x) => self.flatten_dollar_shell(x),
        }
    }

    fn flatten_string_linteral_component(
        &self,
        component: StringLiteralComponent,
    ) -> OsString {
        match component {
            StringLiteralComponent::RawChars(x) => x.0,
            StringLiteralComponent::DollarEnv(x) => self.flatten_dollar_env(x),
        }
    }

    #[inline]
    fn flatten_dollar_shell(&mut self, shell: DollarShell) -> Result<OsString, EvalError> {
        self.flatten_shell_substitution(ShellSubstitution(shell.0))
    }

    fn flatten_dollar_env(&self, env: DollarEnv) -> OsString {
        env::get(&env.0.0)
    }

    // TODO: implement this command
    fn flatten_shell_substitution(&mut self, sub: ShellSubstitution) -> Result<OsString, EvalError> {
        let flat = self.flatten_commandline(sub.0)?;
        Ok(self
            .proc_manager
            .dispatch_capture(flat)
            .map_err(|e| EvalError::DispatchError { internal: e })?
            .1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_ast_flatten() {
        let mut evaluator = Evaluator::new();
        let gen_ast = crate::ast::generate_ast("test 0 '1' \"2\"").unwrap();
        let gen_flatten = evaluator.flatten_commandline(gen_ast.0).unwrap();

        let manual_flatten = FlattenedCmdline {
            envs: Vec::new(),
            command: OsString::from("test"),
            arguments: vec!["0", "1", "2"]
                .into_iter()
                .map(|x| OsString::from(x))
                .collect(),
            redirects: Vec::new(),
            next: None,
        };

        assert_eq!(gen_flatten, manual_flatten);
    }
}
