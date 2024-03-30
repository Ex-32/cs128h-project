use crate::{
    evaluator::Evaluator,
    frontend::{Frontend, ReadlineError},
};
use color_eyre::Result;
use log::{debug, error, info};

mod ast;
mod evaluator;
mod frontend;
mod parser;

static LOG_LEVEL_ENV: &'static str = "RS_SHELL_LOG";
static LOG_STYLE_ENV: &'static str = "RS_SHELL_LOG_STYLE";

fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init_from_env(
        env_logger::Env::new()
            .filter_or(LOG_LEVEL_ENV, "warn")
            .write_style(LOG_STYLE_ENV),
    );
    info!("global logger initalized");

    let mut frontend = Frontend::new()?;
    debug!("constructed frontend singleton");
    let mut evaluator = Evaluator::new();
    debug!("constructed evaluator singleton");

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
        evaluator.dispatch(ast)?;
    }
    info!("REPL loop exited without error, exiting");
    Ok(())
}
