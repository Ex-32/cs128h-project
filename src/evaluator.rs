use crate::ast::*;

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
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
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Evaluator {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FlattenedCmdline {
    pub envs: Vec<(String, String)>,
    pub command: String,
    pub arguments: Vec<String>,
    pub redirects: Vec<(RedirectOp, String)>,
    pub next: Option<(Separator, Box<FlattenedCmdline>)>,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {}
    }

    pub fn eval(&mut self, ast: Main) -> Result<u8, EvalError> {
        let flattened = dbg!(self.flatten_commandline(ast.0)?);

        todo!()
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

    fn flatten_argument(&mut self, arg: Argument) -> Result<String, EvalError> {
        match arg {
            Argument::ShellSubstitution(x) => self.flatten_shell_substitution(x),
            Argument::StringLiteral(x) => self.flatten_string_literal(x),
            Argument::SingleQuoteString(x) => self.flatten_single_string(x),
            Argument::DoubleQuoteString(x) => self.flatten_double_string(x),
        }
    }

    fn flatten_command(&mut self, cmd: Command) -> Result<String, EvalError> {
        match cmd {
            Command::StringLiteral(x) => self.flatten_string_literal(x),
            Command::SingleQuoteString(x) => self.flatten_single_string(x),
            Command::DoubleQuoteString(x) => self.flatten_double_string(x),
        }
    }

    #[inline]
    fn flatten_redirection(&mut self, red: Redirection) -> Result<(RedirectOp, String), EvalError> {
        Ok((red.op, self.flatten_argument(red.arg)?))
    }

    #[inline]
    fn flatten_command_env(&mut self, env: CommandEnv) -> Result<(String, String), EvalError> {
        Ok((
            self.flatten_env_litteral(env.name)?,
            self.flatten_argument(env.value)?,
        ))
    }

    #[inline]
    fn flatten_env_litteral(&self, env: EnvLiteral) -> Result<String, EvalError> {
        Ok(env.0)
    }

    fn flatten_double_string(&mut self, string: DoubleQuoteString) -> Result<String, EvalError> {
        Ok(string
            .0
            .into_iter()
            .map(|x| self.flatten_double_string_component(x))
            .collect::<Result<Vec<_>, EvalError>>()?
            .into_iter()
            .fold(String::new(), |mut acc, x| {
                acc.push_str(&x);
                acc
            }))
    }

    #[inline]
    fn flatten_single_string(&self, string: SingleQuoteString) -> Result<String, EvalError> {
        Ok(string.0)
    }

    fn flatten_string_literal(&mut self, string: StringLiteral) -> Result<String, EvalError> {
        Ok(string
            .0
            .into_iter()
            .map(|x| self.flatten_string_linteral_component(x))
            .collect::<Result<Vec<_>, EvalError>>()?
            .into_iter()
            .fold(String::new(), |mut acc, x| {
                acc.push_str(&x);
                acc
            }))
    }

    fn flatten_double_string_component(
        &mut self,
        component: DoubleQuoteComponent,
    ) -> Result<String, EvalError> {
        match component {
            DoubleQuoteComponent::Chars(x) => Ok(x.0),
            DoubleQuoteComponent::DollarEnv(x) => self.flatten_dollar_env(x),
            DoubleQuoteComponent::DollarShell(x) => self.flatten_dollar_shell(x),
        }
    }

    fn flatten_string_linteral_component(
        &self,
        component: StringLiteralComponent,
    ) -> Result<String, EvalError> {
        match component {
            StringLiteralComponent::RawChars(x) => Ok(x.0),
            StringLiteralComponent::DollarEnv(x) => self.flatten_dollar_env(x),
        }
    }

    #[inline]
    fn flatten_dollar_shell(&mut self, shell: DollarShell) -> Result<String, EvalError> {
        self.flatten_shell_substitution(ShellSubstitution(shell.0))
    }

    fn flatten_dollar_env(&self, env: DollarEnv) -> Result<String, EvalError> {
        match std::env::var(&env.0 .0) {
            Ok(x) => Ok(x),
            Err(std::env::VarError::NotPresent) => Ok(String::new()),
            Err(std::env::VarError::NotUnicode(x)) => Err(EvalError::InvalidEnvValue {
                name: env.0 .0,
                value: x.to_string_lossy().to_string(),
            }),
        }
    }

    // TODO: implement this command
    fn flatten_shell_substitution(&mut self, sub: ShellSubstitution) -> Result<String, EvalError> {
        let flat = self.flatten_commandline(sub.0)?;
        todo!("shell substitution not yet implemented :(");
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
            command: "test".to_owned(),
            arguments: vec!["0", "1", "2"]
                .into_iter()
                .map(|x| String::from(x))
                .collect(),
            redirects: Vec::new(),
            next: None,
        };

        assert_eq!(gen_flatten, manual_flatten);
    }
}
