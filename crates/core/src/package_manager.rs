use std::path::{Path, PathBuf};

use mockall::automock;

use crate::diagnostic_error::DiagnosticError;

pub struct Resolution {
  pub resolved: PathBuf,
}

#[automock]
pub trait PackageManager {
  fn resolve(&self, specifier: &String, from: &Path) -> Result<Resolution, DiagnosticError>;
}
