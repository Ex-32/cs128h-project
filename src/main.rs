use crate::frontend::{Frontend, ReadlineError};
use color_eyre::Result;
use log::info;

mod frontend;

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
        println!("{}", input);
    }

    Ok(())
}
