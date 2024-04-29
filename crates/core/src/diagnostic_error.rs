// TODO Move this to another crate at some point

use std::{error::Error, fmt};

#[derive(Debug)]
pub struct DiagnosticError {
  message: String,
  source: Option<Box<dyn Error>>,
}

impl DiagnosticError {
  pub fn new(message: String) -> Self {
    Self {
      message,
      source: None,
    }
  }

  pub fn new_source<E: Error + 'static>(message: String, source: E) -> Self {
    Self {
      message,
      source: Some(Box::new(source)),
    }
  }
}

impl fmt::Display for DiagnosticError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl Error for DiagnosticError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    self.source.as_deref()
  }

  fn description(&self) -> &str {
    "description() is deprecated; use Display"
  }

  fn cause(&self) -> Option<&dyn Error> {
    self.source()
  }
}
