use std::{
  collections::HashMap,
  env,
  path::{Path, PathBuf},
};

use crate::diagnostic::diagnostic_error::DiagnosticError;

use super::file_system::FileSystem;

pub struct MemoryFileSystem {
  files: HashMap<PathBuf, String>,
}

impl MemoryFileSystem {
  pub fn new(files: HashMap<PathBuf, String>) -> Self {
    MemoryFileSystem { files }
  }
}

impl FileSystem for MemoryFileSystem {
  fn cwd(&self) -> PathBuf {
    env::current_dir().expect("Failed to load the current working directory")
  }

  fn find_ancestor_file(
    &self,
    filenames: Vec<String>,
    from: impl AsRef<Path>,
    root: impl AsRef<Path>,
  ) -> Option<PathBuf> {
    for dir in from.as_ref().ancestors() {
      // Break if we hit a node_modules directory
      if let Some(filename) = dir.file_name() {
        if filename == "node_modules" {
          break;
        }
      }

      for name in &filenames {
        let fullpath = dir.join(name);
        if fullpath.is_file() {
          return Some(fullpath);
        }
      }

      if dir == root.as_ref() {
        break;
      }
    }

    None
  }

  fn read_file(&self, file_path: impl AsRef<Path>) -> Result<String, DiagnosticError> {
    self
      .files
      .get(file_path.as_ref())
      .map(|s| String::from(s))
      .ok_or_else(|| DiagnosticError::new(format!("Failed to read file")))
  }
}
