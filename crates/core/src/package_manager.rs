use std::{
  env, fs, io,
  path::{Path, PathBuf},
};

use crate::diagnostic_error::DiagnosticError;

pub trait PackageManager {
  fn resolve(
    &self,
    files: Vec<String>,
    from: impl AsRef<Path>,
    root: impl AsRef<Path>,
  ) -> Option<PathBuf>;
}

pub struct FileSystem {}

impl FileSystem {
  pub fn new() -> Self {
    FileSystem {}
  }
}

impl Fs for FileSystem {
  fn cwd(&self) -> PathBuf {
    env::current_dir().expect("Failed to retrieve current working directory")
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
    fs::read_to_string(file_path).map_err(|source| {
      DiagnosticError::new_source(format!("Failed to read file {}", file_path), source)
    })
  }
}
