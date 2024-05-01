use std::path::Path;

use crate::diagnostic::diagnostic_error::DiagnosticError;

use super::{PackageManager, Resolution};
use oxc_resolver::Resolver;

#[derive(Default)]
pub struct NodePackageManager {}

impl PackageManager for NodePackageManager {
  fn resolve(&self, specifier: &String, from: &Path) -> Result<Resolution, DiagnosticError> {
    Resolver::default()
      .resolve(from, &specifier)
      .map(|resolution| Resolution {
        resolved: resolution.full_path(),
      })
      .map_err(|source| DiagnosticError::new_source(source.to_string(), source))
  }
}
