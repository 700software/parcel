use std::{
  env, fs,
  path::{Path, PathBuf},
};

use crate::diagnostic_error::DiagnosticError;

pub trait Fs {
  fn cwd(&self) -> PathBuf;
  fn find_ancestor_file(
    &self,
    files: Vec<String>,
    from: impl AsRef<Path>,
    root: impl AsRef<Path>,
  ) -> Option<PathBuf>;
  fn read_file(&self, path: impl AsRef<Path>) -> Result<String, DiagnosticError>;
}

pub struct FileSystem {}

impl FileSystem {
  pub fn new() -> Self {
    FileSystem {}
  }
}

impl Fs for FileSystem {
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
    fs::read_to_string(file_path)
      .map_err(|source| DiagnosticError::new_source(format!("Failed to read file"), source))
  }
}
