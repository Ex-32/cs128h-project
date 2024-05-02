use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar/shell.pest"]
pub(crate) struct ShellParser;
