use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Location {
  pub line: u32,
  pub column: u32,
}

/// Identifies a specific location in a source file
///
/// Source locations start at 1:1.
///
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceLocation {
  /// The file path associated with the source
  pub file_path: PathBuf,

  /// The starting position within the source code
  pub start: Location,

  /// The final location in the source code
  pub end: Location,
}
