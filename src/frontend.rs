use std::path::PathBuf;

use color_eyre::Result;
use log::warn;
pub use rustyline::error::ReadlineError;
use rustyline::{history::FileHistory, Config, Editor};

#[non_exhaustive]
#[derive(Debug)]
pub struct Frontend {
    histfile: PathBuf,
    editor: Editor<(), FileHistory>,
}

impl Frontend {
    pub fn new() -> Result<Self> {
        let histfile = match dirs_next::data_dir() {
            Some(mut x) => {
                x.push("rs-shell/shell_history");
                x
            }
            None => {
                warn!("Unable to locate user data directory, history disabled");
                PathBuf::from(r"/dev/null")
            }
        };
        let editor = Editor::with_history(Config::default(), FileHistory::new())?;

        Ok(Self { histfile, editor })
    }

    pub fn readline(&mut self) -> Result<String, ReadlineError> {
        if let Err(e) = self.editor.load_history(&self.histfile) {
            warn!("failed to load editor history: {}", e);
        }

        let mut value = self.editor.readline("rs-shell $ ")?;

        if value.is_empty() {
            return self.readline();
        }

        while value.chars().last().unwrap() == '\\' {
            value.pop();
            value.push_str(&self.editor.readline(">> ")?);
        }

        if let Err(e) = self.editor.append_history(&self.histfile) {
            warn!("failed to save editor history: {}", e);
        }

        Ok(value)
    }
}
