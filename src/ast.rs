use std::ffi::OsString;

use pest::{iterators::Pair, Parser};

use crate::parser::{Rule, ShellParser};

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum AstError {
    /// this error indicates that [`ShellParser`] failed to parse the string into pair data,
    /// probably because the input was malformed
    #[error("parser error evaluating: '{line}'\n{parse_failure}")]
    ParseError {
        /// the line that the parser tried (and failed) to parse
        line: String,
        /// the internal error thrown by the parser itself
        parse_failure: Box<pest::error::Error<Rule>>,
    },

    /// this error should never be produced, if this error is produced then there is a logic error
    /// in the AST generation code that should be reported as a bug
    #[error("unable to generate AST node '{node_type}' from parser Rule '{pair_type:?}'")]
    RuleMismatch {
        /// the name of AST node that this implementation of [`FromPair`] was trying to creates
        node_type: &'static str,
        /// the parser pair it was actually passed
        pair_type: Rule,
    },
}

trait FromPair {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError>
    where
        Self: Sized;
}

/// top-level component of an AST, it contains a single [`CommandLine`] and enforces the
/// requirement that the parser evaluate the entire input string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Main(pub CommandLine);

/// high-level AST component that describes an entire command including its arguments, environment
/// variables, etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine {
    /// one-shot environment variables to run the command with
    pub envs: Vec<CommandEnv>,
    /// the actual command itself
    pub command: Command,
    /// the arguments passed to the command
    pub arguments: Vec<Argument>,
    /// stdio redirections
    pub redirects: Vec<Redirection>,
    /// a possible second command chained with this one using syntax like `;` or `|`
    pub next: Option<(Separator, Box<CommandLine>)>,
}

/// mid-level AST component that describes an argument to a command
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    StringLiteral(StringLiteral),
    SingleQuoteString(SingleQuoteString),
    DoubleQuoteString(DoubleQuoteString),
    ShellSubstitution(ShellSubstitution),
}

/// mid-level AST component that describes a command, that is, the name or path of an executable or
/// shell builtin
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    StringLiteral(StringLiteral),
    SingleQuoteString(SingleQuoteString),
    DoubleQuoteString(DoubleQuoteString),
}

/// mid-level AST component that describes a redirection of a stdio fd to another file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirection {
    pub op: RedirectOp,
    pub arg: Argument,
}

/// low-level AST component that defines a redirection operation, that is, the specific stdio fd
/// that is being redirected and the type of redirection
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectOp {
    pub fd: RedirectFd,
    pub r#type: RedirectType,
}

/// low-level AST component that defines the file descriptor to be redirected in a redirection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectFd {
    /// the All variant refers to either stdout and stderr if used with [`RedirectType::Out`] or
    /// [`RedirectType::OutAppend`] and to stdin, if used with [`RedirectType::In`]
    All,
    /// default is stdout for output redirects and stdin for input redirects
    Default,
    Stdin,
    Stdout,
    Stderr,
}

/// low-level AST component that defines the type of redirection to be performed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectType {
    Out,
    OutAppend,
    In,
}

/// low-level AST component that defines how multiple [`CommandLine`]s should be chained together
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Separator {
    /// run the first command, wait for it to finish, then run the second
    Semicolon,
    /// run the first command, and then immediately run the second, piping the stdout of the first
    /// to the stdin of the second
    Pipe,
    /// run the first, and then immediately run the second
    Fork,
}

/// mid-level AST component that defines a one-shot environment variable to be set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandEnv {
    /// the name of the environment variable
    pub name: EnvLiteral,
    /// the value of the environment variable
    pub value: Argument,
}

/// low-level AST component that defines the name of an environment variable
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvLiteral(pub OsString);

/// mid-level AST component that defines a shell substitution
///
/// effectively an entire child AST the output of evaluating & executing this inner AST becomes the
/// value of the [`ShellSubstitution`] during when evaluated
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSubstitution(pub CommandLine);

/// mid-level AST component that defines a string enclosed in double quotes
///
/// because double quoted strings can contain complex paces like variable and shell substitution,
/// they're represented as a vector of [`DoubleQuoteComponent`]s each of which is evaluated
/// differently and then concatenated together during evaluation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoubleQuoteString(pub Vec<DoubleQuoteComponent>);

/// low-level AST component that defines a string enclosed in single quotes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingleQuoteString(pub OsString);

/// mid-level AST component that defines a string not enclosed in quotes
///
/// because unquoted strings can contain variable substitutions they're represented as a vector of
/// [`StringLiteralComponent`]s that are evaluated separately and then concatenated together during
/// evaluation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLiteral(pub Vec<StringLiteralComponent>);

/// low-level AST component that defines part of a [`DoubleQuoteString`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoubleQuoteComponent {
    /// literal characters
    Chars(Chars),
    /// environment variable substitution
    DollarEnv(DollarEnv),
    /// shell substitution
    DollarShell(DollarShell),
}

/// low-level AST component that defines part of a [`StringLiteral`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringLiteralComponent {
    /// literal characters
    RawChars(RawChars),
    /// environment variable substitution
    DollarEnv(DollarEnv),
}

/// low-level AST component that defines a environment variable substitution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DollarEnv(pub EnvLiteral);

/// low-level AST component that defines a shell substitution inside a string using the `$()`
/// syntax
///
/// like [`ShellSubstitution`] this effectively contains an entire child AST that is evaluated and
/// run to produce the final string value of this component
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DollarShell(pub CommandLine);

/// low-level AST component that defines literal characters that are inside a double quoted string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chars(pub OsString);

/// low-level AST component that defines literal characters that aren't quoted
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawChars(pub OsString);

/// Parses a string into an AST acording to [`ShellParser`]
///
/// returns a [`Main`] struct, the top-level struct of an AST.
///
/// this function, and the implementations of [`FromPair`] that it relies on contain a large amount
/// of [`unreachable!`] statements based on the parsing expression grammar defined in
/// `src/grammar/shell.pest`, modify with caution
pub fn generate_ast(expr: &str) -> Result<Main, AstError> {
    let pairs = match ShellParser::parse(Rule::Main, expr) {
        Ok(x) => x,
        Err(e) => {
            return Err(AstError::ParseError {
                line: expr.to_owned(),
                parse_failure: Box::new(e),
            })
        }
    };

    let main = Main::from_pair(
        pairs
            .into_iter()
            .next()
            .expect("result of parsing Rule::Main must contain an inner pair"),
    )?;
    Ok(main)
}

impl FromPair for Main {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::Main {
            return Err(AstError::RuleMismatch {
                node_type: "Main",
                pair_type: pair.as_rule(),
            });
        }
        Ok(Self(CommandLine::from_pair(
            pair.into_inner()
                .next()
                .expect("Main Pair must contain CommandLine"),
        )?))
    }
}

impl FromPair for CommandLine {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        let mut envs = Vec::new();
        let mut command = None;
        let mut arguments = Vec::new();
        let mut redirects = Vec::new();
        let mut next_sep = None;
        let mut next_cmd = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::CommandEnv => envs.push(CommandEnv::from_pair(inner)?),
                Rule::Command => command = Some(Command::from_pair(inner)?),
                Rule::Argument => arguments.push(Argument::from_pair(inner)?),
                Rule::Redirection => redirects.push(Redirection::from_pair(inner)?),
                Rule::Separator => next_sep = Some(Separator::from_pair(inner)?),
                Rule::CommandLine => next_cmd = Some(CommandLine::from_pair(inner)?),
                _ => unreachable!("CommandLine can only contain CommandEnv, Command, Argument, Redirection, Separator, or CommandLine"),
            }
        }

        let next = if let Some(sep) = next_sep {
            Some((
                sep,
                Box::new(
                    next_cmd
                        .expect("if CommandLine has Seperator it must also have child CommandLine"),
                ),
            ))
        } else {
            None
        };

        Ok(Self {
            envs,
            command: command.expect("Commandline must contain Command"),
            arguments,
            redirects,
            next,
        })
    }
}

impl FromPair for Argument {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::Argument {
            return Err(AstError::RuleMismatch {
                node_type: "Argument",
                pair_type: pair.as_rule(),
            });
        }
        let inner = pair
            .into_inner()
            .next()
            .expect("Argument must contain inner pair");
        Ok(match inner.as_rule() {
            Rule::ShellSubstitution => Self::ShellSubstitution(ShellSubstitution::from_pair(inner)?),
            Rule::SingleQuoteString => Self::SingleQuoteString(SingleQuoteString::from_pair(inner)?),
            Rule::DoubleQuoteString => Self::DoubleQuoteString(DoubleQuoteString::from_pair(inner)?),
            Rule::StringLiteral => Self::StringLiteral(StringLiteral::from_pair(inner)?),
            _ => unreachable!(
                "Argument can only contain ShellSubstitution, SingleQuoteString, DoubleQuoteString, or StringLiteral"
            ),
        })
    }
}

impl FromPair for Command {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::Command {
            return Err(AstError::RuleMismatch {
                node_type: "Command",
                pair_type: pair.as_rule(),
            });
        }
        let inner = pair
            .into_inner()
            .next()
            .expect("Command must contain inner pair");
        Ok(match inner.as_rule() {
            Rule::SingleQuoteString => {
                Self::SingleQuoteString(SingleQuoteString::from_pair(inner)?)
            }
            Rule::DoubleQuoteString => {
                Self::DoubleQuoteString(DoubleQuoteString::from_pair(inner)?)
            }
            Rule::StringLiteral => Self::StringLiteral(StringLiteral::from_pair(inner)?),
            _ => unreachable!(
                "Command can only contain SingleQuoteString, DoubleQuoteString, or StringLiteral"
            ),
        })
    }
}

impl FromPair for Redirection {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::Redirection {
            return Err(AstError::RuleMismatch {
                node_type: "Redirection",
                pair_type: pair.as_rule(),
            });
        }
        const ERR_MSG: &str = "Redirection must contain at least two inner pairs";
        let mut inner = pair.into_inner();
        Ok(Self {
            op: RedirectOp::from_pair(inner.next().expect(ERR_MSG))?,
            arg: Argument::from_pair(inner.next().expect(ERR_MSG))?,
        })
    }
}

impl FromPair for RedirectOp {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::RedirectOp {
            return Err(AstError::RuleMismatch {
                node_type: "RedirectOp",
                pair_type: pair.as_rule(),
            });
        }
        let mut fd = RedirectFd::Default;

        let mut inner = pair.into_inner();
        let mut next = inner.next().expect("RedirectOp must contain inner pair");
        if let Rule::RedirectFd = next.as_rule() {
            fd = RedirectFd::from_pair(next)?;
            next = inner
                .next()
                .expect("RedirectOp must contain a RedirectType");
        }

        Ok(Self {
            fd,
            r#type: RedirectType::from_pair(next)?,
        })
    }
}

impl FromPair for RedirectFd {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::RedirectFd {
            return Err(AstError::RuleMismatch {
                node_type: "RedirectFd",
                pair_type: pair.as_rule(),
            });
        }
        Ok(match pair.as_str() {
            "&" => RedirectFd::All,
            "0" => RedirectFd::Stdin,
            "1" => RedirectFd::Stdout,
            "2" => RedirectFd::Stderr,
            _ => unreachable!("only '&', '0', '1', & '2' are valid redirect fds"),
        })
    }
}

impl FromPair for RedirectType {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::RedirectType {
            return Err(AstError::RuleMismatch {
                node_type: "RedirectType",
                pair_type: pair.as_rule(),
            });
        }
        Ok(match pair.as_str() {
            ">>" => RedirectType::OutAppend,
            ">" => RedirectType::Out,
            "<" => RedirectType::In,
            _ => unreachable!("Separator can only be '>>', '>', or '<'"),
        })
    }
}

impl FromPair for Separator {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::Separator {
            return Err(AstError::RuleMismatch {
                node_type: "Separator",
                pair_type: pair.as_rule(),
            });
        }
        Ok(match pair.as_str() {
            ";" => Separator::Semicolon,
            "|" => Separator::Pipe,
            "&" => Separator::Fork,
            _ => unreachable!("Separator can only be ';', '|', or '&'"),
        })
    }
}

impl FromPair for CommandEnv {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::CommandEnv {
            return Err(AstError::RuleMismatch {
                node_type: "CommandEnv",
                pair_type: pair.as_rule(),
            });
        }
        const ERR_MSG: &str = "CommandEnv must contain at least two inner pairs";
        let mut inner = pair.into_inner();
        Ok(Self {
            name: EnvLiteral::from_pair(inner.next().expect(ERR_MSG))?,
            value: Argument::from_pair(inner.next().expect(ERR_MSG))?,
        })
    }
}

impl FromPair for EnvLiteral {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::EnvLiteral {
            return Err(AstError::RuleMismatch {
                node_type: "EnvLiteral",
                pair_type: pair.as_rule(),
            });
        }
        Ok(Self(pair.as_str().into()))
    }
}

impl FromPair for ShellSubstitution {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::ShellSubstitution {
            return Err(AstError::RuleMismatch {
                node_type: "ShellSubstitution",
                pair_type: pair.as_rule(),
            });
        }
        Ok(Self(CommandLine::from_pair(
            pair.into_inner()
                .next()
                .expect("ShellSubstitution must contain inner pair"),
        )?))
    }
}

impl FromPair for DoubleQuoteString {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::DoubleQuoteString {
            return Err(AstError::RuleMismatch {
                node_type: "DoubleQuoteString",
                pair_type: pair.as_rule(),
            });
        }
        Ok(Self(
            pair.into_inner()
                .map(|inner| DoubleQuoteComponent::from_pair(inner))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

impl FromPair for SingleQuoteString {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::SingleQuoteString {
            return Err(AstError::RuleMismatch {
                node_type: "SingleQuoteString",
                pair_type: pair.as_rule(),
            });
        }
        Ok(Self(
            pair.into_inner()
                .next()
                .expect("SingleQuoteString must contain inner pair")
                .as_str()
                .into(),
        ))
    }
}

impl FromPair for StringLiteral {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::StringLiteral {
            return Err(AstError::RuleMismatch {
                node_type: "StringLiteral",
                pair_type: pair.as_rule(),
            });
        }
        Ok(Self(
            pair.into_inner()
                .map(|inner| StringLiteralComponent::from_pair(inner))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

impl FromPair for DoubleQuoteComponent {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::DoubleQuoteComponent {
            return Err(AstError::RuleMismatch {
                node_type: "DoubleQuoteComponent",
                pair_type: pair.as_rule(),
            });
        }
        let inner = pair
            .into_inner()
            .next()
            .expect("DoubleQuoteComponent must contain inner pair");
        Ok(match inner.as_rule() {
            Rule::Chars => Self::Chars(Chars::from_pair(inner)?),
            Rule::DollarEnv => Self::DollarEnv(DollarEnv::from_pair(inner)?),
            Rule::DollarShell => Self::DollarShell(DollarShell::from_pair(inner)?),
            _ => unreachable!(
                "DoubleQuoteComponent can only contain Chars, DollarEnv, or DollarShell"
            ),
        })
    }
}

impl FromPair for StringLiteralComponent {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::StringLiteralComponent {
            return Err(AstError::RuleMismatch {
                node_type: "StringLiteralComponent",
                pair_type: pair.as_rule(),
            });
        }
        let inner = pair
            .into_inner()
            .next()
            .expect("StringLiteralComponent must contain inner pair");
        Ok(match inner.as_rule() {
            Rule::RawChars => Self::RawChars(RawChars::from_pair(inner)?),
            Rule::DollarEnv => Self::DollarEnv(DollarEnv::from_pair(inner)?),
            _ => unreachable!("StringLiteralComponent can only contain RawChars or DollarEnv"),
        })
    }
}

impl FromPair for DollarEnv {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::DollarEnv {
            return Err(AstError::RuleMismatch {
                node_type: "DollarEnv",
                pair_type: pair.as_rule(),
            });
        }
        Ok(Self(EnvLiteral::from_pair(
            pair.into_inner()
                .next()
                .expect("DollarEnv Pair must contain inner pair"),
        )?))
    }
}

impl FromPair for DollarShell {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::DollarShell {
            return Err(AstError::RuleMismatch {
                node_type: "DollarShell",
                pair_type: pair.as_rule(),
            });
        }
        Ok(Self(CommandLine::from_pair(
            pair.into_inner()
                .next()
                .expect("DollarShell Pair must contain inner pair"),
        )?))
    }
}

impl FromPair for Chars {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::Chars {
            return Err(AstError::RuleMismatch {
                node_type: "Chars",
                pair_type: pair.as_rule(),
            });
        }
        Ok(Chars(pair.as_str().into()))
    }
}

impl FromPair for RawChars {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError> {
        if pair.as_rule() != Rule::RawChars {
            return Err(AstError::RuleMismatch {
                node_type: "RawChars",
                pair_type: pair.as_rule(),
            });
        }
        Ok(RawChars(pair.as_str().into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_ast_gen() {
        let manual_ast = Main(CommandLine {
            envs: Vec::new(),
            command: Command::StringLiteral(StringLiteral(vec![StringLiteralComponent::RawChars(
                RawChars("test".into()),
            )])),
            arguments: vec![
                Argument::StringLiteral(StringLiteral(vec![StringLiteralComponent::RawChars(
                    RawChars("0".into()),
                )])),
                Argument::SingleQuoteString(SingleQuoteString("1".into())),
                Argument::DoubleQuoteString(DoubleQuoteString(vec![DoubleQuoteComponent::Chars(
                    Chars("2".into()),
                )])),
            ],
            redirects: vec![],
            next: None,
        });

        let gen_ast = generate_ast("test 0 '1' \"2\"").unwrap();

        assert_eq!(gen_ast, manual_ast);
    }
}
