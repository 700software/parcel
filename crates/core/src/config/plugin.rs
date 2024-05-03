use glob_match::glob_match;
use indexmap::IndexMap;
use std::{
  path::{Path, PathBuf},
  rc::Rc,
};

#[derive(Clone, Debug, PartialEq)]
pub struct PluginNode {
  pub package_name: String,
  pub resolve_from: Rc<PathBuf>,
}

#[derive(Debug, PartialEq)]
pub struct PipelineMap {
  map: IndexMap<String, Vec<PluginNode>>,
}

impl PipelineMap {
  pub fn new(map: IndexMap<String, Vec<PluginNode>>) -> Self {
    Self { map }
  }

  pub fn get(&self, path: &Path, pipeline: &Option<impl AsRef<str>>) -> Vec<PluginNode> {
    let basename = path.file_name().unwrap().to_str().unwrap();
    let path = path.as_os_str().to_str().unwrap();

    let mut matches = Vec::new();
    if let Some(pipeline) = pipeline {
      let exact_match = self
        .map
        .iter()
        .find(|(pattern, _)| is_match(pattern, path, basename, pipeline.as_ref()));

      if let Some((_, m)) = exact_match {
        matches.push(m);
      } else {
        return Vec::new();
      }
    }

    for (pattern, pipeline) in self.map.iter() {
      if is_match(pattern, path, basename, "") {
        matches.push(pipeline);
      }
    }

    if matches.is_empty() {
      return Vec::new();
    }

    fn flatten(matches: &mut Vec<&Vec<PluginNode>>) -> Vec<PluginNode> {
      matches
        .remove(0)
        .into_iter()
        .flat_map(|plugin| vec![plugin.clone()])
        .collect()
    }

    flatten(&mut matches)
  }

  pub fn contains_named_pipeline(&self, pipeline: impl AsRef<str>) -> bool {
    let named_pipeline = format!("{}:", pipeline.as_ref());

    self
      .map
      .keys()
      .any(|glob| glob.starts_with(&named_pipeline))
  }

  pub fn named_pipelines(&self) -> Vec<&str> {
    self
      .map
      .keys()
      .filter_map(|glob| glob.split_once(':').map(|g| g.0))
      .collect()
  }
}

pub(crate) fn is_match(pattern: &str, path: &str, basename: &str, pipeline: &str) -> bool {
  let (pattern_pipeline, glob) = pattern.split_once(':').unwrap_or(("", pattern));
  if pipeline.is_empty() && pattern_pipeline.is_empty() {
    return false;
  }

  if !pipeline.is_empty() && pipeline != pattern_pipeline {
    return false;
  }

  return glob_match(glob, basename) || glob_match(glob, path);
}
