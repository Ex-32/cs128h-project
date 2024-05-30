use color_eyre::Result;
pub use rustyline::error::ReadlineError;
use rustyline::{history::DefaultHistory, Editor};

#[non_exhaustive]
#[derive(Debug)]
pub struct Frontend {
    editor: Editor<(), DefaultHistory>,
}

impl Frontend {
    pub fn new() -> Result<Self> {
        let editor = Editor::new()?;
        Ok(Self { editor })
    }

    pub fn readline(&mut self) -> Result<String, ReadlineError> {
        let mut value = self.editor.readline("rs-shell $ ")?;

        if value.is_empty() {
            return self.readline();
        }

        while value.chars().last().unwrap() == '\\' {
            value.pop();
            value.push_str(&self.editor.readline(">> ")?);
        }

        Ok(value)
    }
}
