use pest::{iterators::Pair, Parser};

use crate::parser::{Rule, ShellParser};

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AstError {
    #[error("parse error evaluating: '{line}'\n{parse_failure}")]
    ParseError {
        line: String,
        parse_failure: Box<pest::error::Error<Rule>>,
    },

    #[error("unable to generate AST node '{node_type}' from parser Rule '{pair_type:?}'")]
    RuleMismatch {
        node_type: &'static str,
        pair_type: Rule,
    },
}

trait FromPair {
    fn from_pair(pair: Pair<Rule>) -> Result<Self, AstError>
    where
        Self: Sized;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Main(pub CommandLine);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine {
    pub envs: Vec<CommandEnv>,
    pub command: Command,
    pub arguments: Vec<Argument>,
    pub redirects: Vec<Redirection>,
    pub next: Option<(Separator, Box<CommandLine>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    StringLiteral(StringLiteral),
    SingleQuoteString(SingleQuoteString),
    DoubleQuoteString(DoubleQuoteString),
    ShellSubstitution(ShellSubstitution),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    StringLiteral(StringLiteral),
    SingleQuoteString(SingleQuoteString),
    DoubleQuoteString(DoubleQuoteString),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirection {
    pub op: RedirectOp,
    pub arg: Argument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectOp {
    fd: RedirectFd,
    r#type: RedirectType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectFd {
    All,
    Stdin,
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectType {
    Out,
    OutAppend,
    In,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Separator {
    Semicolon,
    Pipe,
    Fork,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandEnv {
    pub name: EnvLiteral,
    pub value: Argument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvLiteral(pub String);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSubstitution(pub CommandLine);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoubleQuoteString(pub Vec<DoubleQuoteComponent>);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingleQuoteString(pub String);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLiteral(pub Vec<StringLiteralComponent>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoubleQuoteComponent {
    Chars(Chars),
    DollarEnv(DollarEnv),
    DollarShell(DollarShell),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringLiteralComponent {
    RawChars(RawChars),
    DollarEnv(DollarEnv),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DollarEnv(pub EnvLiteral);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DollarShell(pub CommandLine);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chars(pub String);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawChars(pub String);

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

    let main =
        Main::from_pair(pairs.into_iter().next().ok_or_else(|| {
            unreachable!("result of parsing Rule::Main must contain an inner pair")
        })?)?;
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
                .ok_or_else(|| unreachable!("Main Pair must contain CommandLine"))?,
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
                Box::new(next_cmd.ok_or_else(|| {
                    unreachable!("if CommandLine has Seperator it must also have child CommandLine")
                })?),
            ))
        } else {
            None
        };

        Ok(Self {
            envs,
            command: command.ok_or_else(|| unreachable!("Commandline must contain Command"))?,
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
            .ok_or_else(|| unreachable!("Argument must contain inner pair"))?;
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
            .ok_or_else(|| unreachable!("Command must contain inner pair"))?;
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
        let unreachable = || unreachable!("Redirection must contain at least two inner pairs");
        let mut inner = pair.into_inner();
        Ok(Self {
            op: RedirectOp::from_pair(inner.next().ok_or_else(unreachable)?)?,
            arg: Argument::from_pair(inner.next().ok_or_else(unreachable)?)?,
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
        let mut fd = RedirectFd::Stdout;

        let mut inner = pair.into_inner();
        let mut next = inner
            .next()
            .ok_or_else(|| unreachable!("RedirectOp must contain inner pair"))?;
        if let Rule::RedirectFd = next.as_rule() {
            fd = RedirectFd::from_pair(next)?;
            next = inner
                .next()
                .ok_or_else(|| unreachable!("RedirectOp must contain a RedirectType"))?;
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
        let unreachable = || unreachable!("CommandEnv must contain at least two inner pairs");
        let mut inner = pair.into_inner();
        Ok(Self {
            name: EnvLiteral::from_pair(inner.next().ok_or_else(unreachable)?)?,
            value: Argument::from_pair(inner.next().ok_or_else(unreachable)?)?,
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
        Ok(Self(pair.as_str().to_owned()))
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
                .ok_or_else(|| unreachable!("ShellSubstitution must contain inner pair"))?,
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
                .ok_or_else(|| unreachable!("SingleQuoteString must contain inner pair"))?
                .as_str()
                .to_owned(),
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
            .ok_or_else(|| unreachable!("DoubleQuoteComponent must contain inner pair"))?;
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
            .ok_or_else(|| unreachable!("StringLiteralComponent must contain inner pair"))?;
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
                .ok_or_else(|| unreachable!("DollarEnv Pair must contain inner pair"))?,
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
                .ok_or_else(|| unreachable!("DollarShell Pair must contain inner pair"))?,
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
        Ok(Chars(pair.as_str().to_owned()))
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
        Ok(RawChars(pair.as_str().to_owned()))
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
                RawChars("test".to_owned()),
            )])),
            arguments: vec![
                Argument::StringLiteral(StringLiteral(vec![StringLiteralComponent::RawChars(
                    RawChars("0".to_owned()),
                )])),
                Argument::SingleQuoteString(SingleQuoteString("1".to_owned())),
                Argument::DoubleQuoteString(DoubleQuoteString(vec![DoubleQuoteComponent::Chars(
                    Chars("2".to_owned()),
                )])),
            ],
            redirects: vec![],
            next: None,
        });

        let gen_ast = generate_ast("test 0 '1' \"2\"").unwrap();

        assert_eq!(gen_ast, manual_ast);
    }
}
