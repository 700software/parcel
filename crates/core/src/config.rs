use glob_match::glob_match;
use indexmap::IndexMap;
use std::{
  borrow::Borrow,
  collections::HashMap,
  marker::PhantomData,
  path::{Path, PathBuf},
  rc::Rc,
};

use crate::parcel_config::ParcelRc;

#[derive(Debug, PartialEq)]
pub struct PipelineMap {
  map: IndexMap<String, Vec<PluginNode>>,
}

impl PipelineMap {
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
}

#[derive(Debug, PartialEq)]
pub struct Config {
  bundler: PluginNode,
  compressors: PipelineMap,
  namers: Vec<PluginNode>,
  optimizers: PipelineMap,
  packagers: PipelineMap,
  reporters: Vec<PluginNode>,
  resolvers: Vec<PluginNode>,
  runtimes: Vec<PluginNode>,
  transformers: PipelineMap,
  validators: PipelineMap,
}

#[derive(Debug, Default, PartialEq)]
pub struct PartialConfig {
  bundler: Option<PluginNode>,
  compressors: HashMap<String, Vec<PluginNode>>,
  namers: Vec<PluginNode>,
  optimizers: HashMap<String, Vec<PluginNode>>,
  packagers: HashMap<String, PluginNode>,
  reporters: Vec<PluginNode>,
  resolvers: Vec<PluginNode>,
  runtimes: Vec<PluginNode>,
  transformers: HashMap<String, Vec<PluginNode>>,
  validators: HashMap<String, Vec<PluginNode>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PluginNode {
  pub package_name: String,
  pub resolve_from: Rc<String>,
}

pub struct ProjectPath(String);

type Result<T> = std::result::Result<T, String>;

impl From<&ParcelRc> for PartialConfig {
  fn from(parcel_rc: &ParcelRc) -> Self {
    let resolve_from = Rc::new(parcel_rc.path.display().to_string());

    // TODO
    PartialConfig {
      bundler: parcel_rc
        .contents
        .bundler
        .as_ref()
        .map(|package_name| PluginNode {
          package_name: package_name.clone(),
          resolve_from: Rc::clone(&resolve_from),
        }),
      compressors: HashMap::new(),
      namers: parcel_rc
        .contents
        .namers
        .as_ref()
        .map(|namers| {
          namers
            .into_iter()
            .map(|package_name| PluginNode {
              package_name: package_name.clone(),
              resolve_from: Rc::clone(&resolve_from),
            })
            .collect()
        })
        .unwrap_or(Vec::new()),
      optimizers: HashMap::new(),
      packagers: HashMap::new(),
      reporters: parcel_rc
        .contents
        .reporters
        .as_ref()
        .map(|reporters| {
          reporters
            .into_iter()
            .map(|package_name| PluginNode {
              package_name: package_name.clone(),
              resolve_from: Rc::clone(&resolve_from),
            })
            .collect()
        })
        .unwrap_or(Vec::new()),
      resolvers: parcel_rc
        .contents
        .resolvers
        .as_ref()
        .map(|resolvers| {
          resolvers
            .into_iter()
            .map(|package_name| PluginNode {
              package_name: package_name.clone(),
              resolve_from: Rc::clone(&resolve_from),
            })
            .collect()
        })
        .unwrap_or(Vec::new()),
      runtimes: parcel_rc
        .contents
        .runtimes
        .as_ref()
        .map(|runtimes| {
          runtimes
            .into_iter()
            .map(|package_name| PluginNode {
              package_name: package_name.clone(),
              resolve_from: Rc::clone(&resolve_from),
            })
            .collect()
        })
        .unwrap_or(Vec::new()),
      transformers: HashMap::new(),
      validators: HashMap::new(),
    }
  }
}

impl Config {
  pub fn validators(&self, path: &Path) -> Result<Vec<PluginNode>> {
    let pipeline: &Option<&str> = &None;
    let validators = self.validators.get(path, pipeline);

    Ok(validators)
  }

  pub fn transformers(
    &self,
    path: &Path,
    pipeline: &Option<impl AsRef<str>>,
    allow_empty: bool,
  ) -> Result<Vec<PluginNode>> {
    let transformers = self.transformers.get(path, pipeline);

    if transformers.is_empty() {
      if allow_empty {
        return Ok(Vec::new());
      }

      let path = path.as_os_str().to_str().unwrap();

      return match pipeline {
        None => self.missing_plugin_error(format!("No transformers found for {}.", path)),
        Some(pipeline) => self.missing_plugin_error(format!(
          "No transformers found for {} with pipeline {:?}.",
          path,
          pipeline.as_ref()
        )),
      };
    }

    Ok(transformers)
  }

  pub fn bundler<P: AsRef<str>>(&self) -> Result<PluginNode> {
    Ok(self.bundler.clone())
    // match self.bundler.clone() {
    //   None => self.missing_plugin_error(String::from("No bundler specified in .parcelrc config")),
    //   Some(bundler) => Ok(bundler),
    // }
  }

  pub fn namers(&self) -> Result<Vec<PluginNode>> {
    if self.namers.is_empty() {
      return self.missing_plugin_error(String::from(
        "No namer plugins specified in .parcelrc config",
      ));
    }

    Ok(self.namers.clone())
  }

  pub fn runtimes(&self) -> Result<Vec<PluginNode>> {
    if self.runtimes.is_empty() {
      return Ok(Vec::new());
    }

    Ok(self.runtimes.clone())
  }

  pub fn packager(&self, path: &Path) -> Result<PluginNode> {
    let basename = path.file_name().unwrap().to_str().unwrap();
    let path = path.as_os_str().to_str().unwrap();
    let packager = self
      .packagers
      .map
      .iter()
      .find(|(pattern, _)| is_match(pattern, path, basename, ""));

    match packager {
      None => self.missing_plugin_error(format!("No packager found for {}", path)),
      Some((_, pkgr)) => Ok(pkgr.first().unwrap().clone()),
    }
  }

  pub fn optimizers(
    &self,
    path: &Path,
    pipeline: &Option<impl AsRef<str>>,
  ) -> Result<Vec<PluginNode>> {
    let mut use_empty_pipeline = false;
    // If a pipeline is specified, but it doesn't exist in the optimizers config, ignore it.
    // Pipelines for bundles come from their entry assets, so the pipeline likely exists in transformers.
    if let Some(p) = pipeline {
      let prefix = format!("{}:", p.as_ref());
      if !self
        .optimizers
        .map
        .keys()
        .any(|glob| glob.starts_with(&prefix))
      {
        use_empty_pipeline = true;
      }
    }

    let optimizers = self
      .optimizers
      .get(path, if use_empty_pipeline { &None } else { pipeline });
    if optimizers.is_empty() {
      return Ok(Vec::new());
    }

    Ok(optimizers)
  }

  pub fn compressors(&self, path: &Path) -> Result<Vec<PluginNode>> {
    let pipeline: &Option<&str> = &None;
    let compressors = self.compressors.get(path, pipeline);
    if compressors.is_empty() {
      let path = path.as_os_str().to_str().unwrap();
      return self.missing_plugin_error(format!("No compressors found for {}", path));
    }

    Ok(compressors)
  }

  pub fn resolvers(&self) -> Result<Vec<PluginNode>> {
    if self.resolvers.is_empty() {
      return self.missing_plugin_error(String::from("No resolvers specified in .parcelrc config"));
    }

    Ok(self.resolvers.clone())
  }

  pub fn reporters(&self) -> Result<Vec<PluginNode>> {
    Ok(self.reporters.clone())
  }

  fn missing_plugin_error<T>(&self, msg: String) -> Result<T> {
    Err(msg)
  }
}

fn is_match(pattern: &str, path: &str, basename: &str, pipeline: &str) -> bool {
  let (pattern_pipeline, glob) = pattern.split_once(':').unwrap_or(("", pattern));
  if pipeline.is_empty() && pattern_pipeline.is_empty() {
    return false;
  }

  if !pipeline.is_empty() && pipeline != pattern_pipeline {
    return false;
  }

  return glob_match(glob, basename) || glob_match(glob, path);
}
