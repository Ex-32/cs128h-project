use crate::{
    ast::AstError,
    frontend::{Frontend, ReadlineError},
};
use color_eyre::Result;
use log::{error, info};

mod ast;
mod frontend;
mod parser;

fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init_from_env(
        env_logger::Env::new()
            .filter("RS_SHELL_LOG")
            .write_style("RS_SHELL_LOG_STYLE"),
    );
    info!("global logger initalized");

    let mut frontend = Frontend::new()?;

    loop {
        let input = match frontend.readline() {
            Ok(x) => x,
            Err(e) => match e {
                ReadlineError::Eof => break,
                _ => return Err(e.into()),
            },
        };
        let ast = match ast::generate_ast(&input) {
            Ok(x) => x,
            Err(e) => {
                match e {
                    AstError::ParseError { parse_failure, .. } => {
                        eprintln!("{}", *parse_failure);
                    }
                    AstError::RuleMismatch { .. } => {
                        error!("{}", e);
                    }
                    AstError::FdSizeOverflow { .. } => {
                        eprintln!("{}", e);
                    }
                };
                continue;
            }
        };
    }

    Ok(())
}
