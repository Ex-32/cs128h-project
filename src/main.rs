use crate::{
    evaluator::Evaluator,
    frontend::{Frontend, ReadlineError},
};
use clap::Parser;
use color_eyre::Result;
use log::{debug, error, info};
use std::process::ExitCode;

mod ast;
mod evaluator;
mod frontend;
mod parser;

static LOG_LEVEL_ENV: &'static str = "RS_SHELL_LOG";
static LOG_STYLE_ENV: &'static str = "RS_SHELL_LOG_STYLE";

#[derive(Debug, Clone, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// evaluate given expression and then exit
    #[arg(short, long)]
    command: Option<String>,
}

fn main() -> Result<ExitCode> {
    color_eyre::install()?;
    env_logger::init_from_env(
        env_logger::Env::new()
            .filter_or(LOG_LEVEL_ENV, "warn")
            .write_style(LOG_STYLE_ENV),
    );
    info!("global logger initalized");

    let args = Args::parse();

    let mut evaluator = Evaluator::new();
    debug!("constructed evaluator singleton");

    if let Some(cmd) = args.command {
        let ast = ast::generate_ast(&cmd)?;
        return Ok(ExitCode::from(evaluator.eval(ast)?));
    }

    let mut frontend = Frontend::new()?;
    debug!("constructed frontend singleton");

    loop {
        let input = match frontend.readline() {
            Ok(x) => x,
            Err(e) => match e {
                ReadlineError::Eof => break,
                _ => return Err(e.into()),
            },
        };
        debug!("read line from user: '{}'", input);
        let ast = match ast::generate_ast(&input) {
            Ok(x) => x,
            Err(e) => {
                error!("{}", e);
                continue;
            }
        };
        debug!("successful AST generation");
        println!("{:#?}", ast);
        evaluator.eval(ast)?;
    }
    info!("REPL loop exited without error, exiting");
    Ok(ExitCode::SUCCESS)
}
