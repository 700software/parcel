use std::path::Path;

use indexmap::IndexMap;

use crate::diagnostic::diagnostic_error::DiagnosticError;

use super::{
  partial_parcel_config::PartialParcelConfig,
  plugin::{is_match, PipelineMap, PluginNode},
};

#[derive(Debug, PartialEq)]
pub struct ParcelConfig {
  pub(crate) bundler: PluginNode,
  pub(crate) compressors: PipelineMap,
  pub(crate) namers: Vec<PluginNode>,
  pub(crate) optimizers: PipelineMap,
  pub(crate) packagers: IndexMap<String, PluginNode>,
  pub(crate) reporters: Vec<PluginNode>,
  pub(crate) resolvers: Vec<PluginNode>,
  pub(crate) runtimes: Vec<PluginNode>,
  pub(crate) transformers: PipelineMap,
  pub(crate) validators: PipelineMap,
}

impl TryFrom<PartialParcelConfig> for ParcelConfig {
  type Error = DiagnosticError;

  fn try_from(config: PartialParcelConfig) -> Result<Self, Self::Error> {
    // The final stage of merging filters out any ... extensions as they are a noop
    fn filter_out_extends(pipelines: Vec<PluginNode>) -> Vec<PluginNode> {
      pipelines
        .into_iter()
        .filter(|p| p.package_name != "...")
        .collect()
    }

    fn filter_out_extends_from_map(
      map: IndexMap<String, Vec<PluginNode>>,
    ) -> IndexMap<String, Vec<PluginNode>> {
      map
        .into_iter()
        .map(|(pattern, plugins)| (pattern, filter_out_extends(plugins)))
        .collect()
    }

    match config.bundler {
      None => Err(DiagnosticError::new(String::from("Missing bundler"))),
      Some(bundler) => Ok(ParcelConfig {
        bundler,
        compressors: PipelineMap::new(filter_out_extends_from_map(config.compressors)),
        namers: filter_out_extends(config.namers),
        optimizers: PipelineMap::new(filter_out_extends_from_map(config.optimizers)),
        packagers: config.packagers,
        reporters: filter_out_extends(config.reporters),
        resolvers: filter_out_extends(config.resolvers),
        runtimes: filter_out_extends(config.runtimes),
        transformers: PipelineMap::new(filter_out_extends_from_map(config.transformers)),
        validators: PipelineMap::new(filter_out_extends_from_map(config.validators)),
      }),
    }
  }
}

// TODO Remove validations later for anything that does not take in input, should be done in parcel_rc_config
impl ParcelConfig {
  pub fn validators(&self, path: &Path) -> Result<Vec<PluginNode>, DiagnosticError> {
    let pipeline: &Option<&str> = &None;
    let validators = self.validators.get(path, pipeline);

    Ok(validators)
  }

  pub fn transformers(
    &self,
    path: &Path,
    pipeline: &Option<impl AsRef<str>>,
    allow_empty: bool,
  ) -> Result<Vec<PluginNode>, DiagnosticError> {
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

  pub fn bundler<P: AsRef<str>>(&self) -> Result<PluginNode, DiagnosticError> {
    Ok(self.bundler.clone())
  }

  pub fn namers(&self) -> Result<Vec<PluginNode>, DiagnosticError> {
    if self.namers.is_empty() {
      return self.missing_plugin_error(String::from(
        "No namer plugins specified in .parcelrc config",
      ));
    }

    Ok(self.namers.clone())
  }

  pub fn runtimes(&self) -> Result<Vec<PluginNode>, DiagnosticError> {
    if self.runtimes.is_empty() {
      return Ok(Vec::new());
    }

    Ok(self.runtimes.clone())
  }

  pub fn packager(&self, path: &Path) -> Result<PluginNode, DiagnosticError> {
    let basename = path.file_name().unwrap().to_str().unwrap();
    let path = path.as_os_str().to_str().unwrap();
    let packager = self
      .packagers
      .iter()
      .find(|(pattern, _)| is_match(pattern, path, basename, ""));

    match packager {
      None => self.missing_plugin_error(format!("No packager found for {}", path)),
      Some((_, pkgr)) => Ok(pkgr.clone()),
    }
  }

  pub fn optimizers(
    &self,
    path: &Path,
    pipeline: &Option<impl AsRef<str>>,
  ) -> Result<Vec<PluginNode>, DiagnosticError> {
    let mut use_empty_pipeline = false;
    // If a pipeline is specified, but it doesn't exist in the optimizers config, ignore it.
    // Pipelines for bundles come from their entry assets, so the pipeline likely exists in transformers.
    if let Some(p) = pipeline {
      if !self.optimizers.contains_named_pipeline(p) {
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

  pub fn compressors(&self, path: &Path) -> Result<Vec<PluginNode>, DiagnosticError> {
    let pipeline: &Option<&str> = &None;
    let compressors = self.compressors.get(path, pipeline);
    if compressors.is_empty() {
      let path = path.as_os_str().to_str().unwrap();
      return self.missing_plugin_error(format!("No compressors found for {}", path));
    }

    Ok(compressors)
  }

  pub fn resolvers(&self) -> Result<Vec<PluginNode>, DiagnosticError> {
    if self.resolvers.is_empty() {
      return self.missing_plugin_error(String::from("No resolvers specified in .parcelrc config"));
    }

    Ok(self.resolvers.clone())
  }

  pub fn reporters(&self) -> Result<Vec<PluginNode>, DiagnosticError> {
    Ok(self.reporters.clone())
  }

  fn missing_plugin_error<T>(&self, msg: String) -> Result<T, DiagnosticError> {
    Err(DiagnosticError::new(msg))
  }
}

// TODO Testing
