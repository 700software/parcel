use std::path::{Path, PathBuf};

use crate::diagnostic::diagnostic_error::DiagnosticError;

pub trait FileSystem {
  fn cwd(&self) -> PathBuf;
  fn find_ancestor_file(
    &self,
    files: Vec<String>,
    from: impl AsRef<Path>,
    root: impl AsRef<Path>,
  ) -> Option<PathBuf>;
  fn read_file(&self, path: impl AsRef<Path>) -> Result<String, DiagnosticError>;
}
