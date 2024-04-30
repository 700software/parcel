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
    if self.source.is_none() {
      return write!(f, "{}", self.message);
    }

    return write!(
      f,
      "{}: {:?}",
      self.message,
      self.source.as_ref().unwrap().to_string()
    );
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
