use indexmap::IndexMap;
use std::rc::Rc;

use super::{parcel_config::PluginNode, parcel_rc::ParcelRcFile};

/// An intermediate representation of the .parcelrc config
///
/// This data structure is used to perform configuration merging, to eventually create a compelete ParcelConfig.
///
#[derive(Debug, Default, PartialEq)]
pub struct PartialParcelConfig {
  pub bundler: Option<PluginNode>,
  pub compressors: IndexMap<String, Vec<PluginNode>>,
  pub namers: Vec<PluginNode>,
  pub optimizers: IndexMap<String, Vec<PluginNode>>,
  pub packagers: IndexMap<String, PluginNode>,
  pub reporters: Vec<PluginNode>,
  pub resolvers: Vec<PluginNode>,
  pub runtimes: Vec<PluginNode>,
  pub transformers: IndexMap<String, Vec<PluginNode>>,
  pub validators: IndexMap<String, Vec<PluginNode>>,
}

impl From<&ParcelRcFile> for PartialParcelConfig {
  fn from(parcel_rc: &ParcelRcFile) -> Self {
    let resolve_from = Rc::new(parcel_rc.path.display().to_string());

    let to_vec = |maybe_plugins: Option<&Vec<String>>| {
      maybe_plugins
        .map(|plugins| {
          plugins
            .iter()
            .map(|package_name| PluginNode {
              package_name: String::from(package_name),
              // resolve_from: Rc::clone(&resolve_from),
              resolve_from: resolve_from.clone(),
            })
            .collect()
        })
        .unwrap_or(Vec::new())
    };

    let to_pipelines = |map: Option<&IndexMap<String, Vec<String>>>| {
      map
        .map(|plugins| {
          plugins
            .iter()
            .map(|(pattern, plugins)| {
              (
                String::from(pattern),
                plugins
                  .iter()
                  .map(|package_name| PluginNode {
                    package_name: String::from(package_name),
                    resolve_from: Rc::clone(&resolve_from),
                  })
                  .collect(),
              )
            })
            .collect()
        })
        .unwrap_or(IndexMap::new())
    };

    let to_pipeline = |map: Option<&IndexMap<String, String>>| {
      map
        .map(|plugins| {
          plugins
            .iter()
            .map(|(pattern, package_name)| {
              (
                String::from(pattern),
                PluginNode {
                  package_name: String::from(package_name),
                  resolve_from: Rc::clone(&resolve_from),
                },
              )
            })
            .collect()
        })
        .unwrap_or(IndexMap::new())
    };

    PartialParcelConfig {
      bundler: parcel_rc
        .contents
        .bundler
        .as_ref()
        .map(|package_name| PluginNode {
          package_name: String::from(package_name),
          resolve_from: Rc::clone(&resolve_from),
        }),
      compressors: to_pipelines(parcel_rc.contents.compressors.as_ref()),
      namers: to_vec(parcel_rc.contents.namers.as_ref()),
      optimizers: to_pipelines(parcel_rc.contents.optimizers.as_ref()),
      packagers: to_pipeline(parcel_rc.contents.packagers.as_ref()),
      reporters: to_vec(parcel_rc.contents.reporters.as_ref()),
      resolvers: to_vec(parcel_rc.contents.resolvers.as_ref()),
      runtimes: to_vec(parcel_rc.contents.runtimes.as_ref()),
      transformers: to_pipelines(parcel_rc.contents.transformers.as_ref()),
      validators: to_pipelines(parcel_rc.contents.validators.as_ref()),
    }
  }
}

impl PartialParcelConfig {
  pub fn merge(base: &PartialParcelConfig, other: &PartialParcelConfig) -> Self {
    PartialParcelConfig::default()
  }
}
