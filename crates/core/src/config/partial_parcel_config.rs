use indexmap::IndexMap;
use std::{collections::HashSet, rc::Rc};

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

// TODO validate ...
impl PartialParcelConfig {
  fn merge_map(
    from_map: IndexMap<String, PluginNode>,
    to_map: IndexMap<String, PluginNode>,
  ) -> IndexMap<String, PluginNode> {
    if to_map.is_empty() {
      return from_map;
    }

    if from_map.is_empty() {
      return to_map;
    }

    // TODO
    from_map
  }

  fn merge_maps(
    from_map: IndexMap<String, Vec<PluginNode>>,
    to_map: IndexMap<String, Vec<PluginNode>>,
  ) -> IndexMap<String, Vec<PluginNode>> {
    if to_map.is_empty() {
      return from_map;
    }

    if from_map.is_empty() {
      return to_map;
    }

    let mut map = IndexMap::new();
    let mut used_patterns = HashSet::new();

    // Add the extension options first so they have higher precedence in the output glob map
    for (pattern, from_pipelines) in to_map {
      let to_pipelines = from_map.get(&pattern);
      if to_pipelines.is_some() {
        used_patterns.insert(pattern.clone());
        map.insert(
          pattern,
          PartialParcelConfig::merge_pipelines(from_pipelines, to_pipelines.unwrap().clone()),
        );
      }
    }

    // Add remaining pipelines
    for (pattern, pipelines) in from_map {
      if !used_patterns.contains(&pattern) {
        map.insert(String::from(pattern), pipelines);
      }
    }

    map
  }

  fn merge_pipelines(
    from_pipelines: Vec<PluginNode>,
    to_pipelines: Vec<PluginNode>,
  ) -> Vec<PluginNode> {
    let spread_index = from_pipelines
      .iter()
      .position(|plugin| plugin.package_name == "...");

    match spread_index {
      None => from_pipelines,
      Some(index) => [
        &from_pipelines[..index],
        to_pipelines.as_slice(),
        &from_pipelines[index..],
      ]
      .concat(),
    }
  }

  pub fn merge(from_config: PartialParcelConfig, extend_config: PartialParcelConfig) -> Self {
    PartialParcelConfig {
      bundler: from_config.bundler.or(extend_config.bundler),
      compressors: PartialParcelConfig::merge_maps(
        from_config.compressors,
        extend_config.compressors,
      ),
      namers: PartialParcelConfig::merge_pipelines(from_config.namers, extend_config.namers),
      optimizers: PartialParcelConfig::merge_maps(from_config.optimizers, extend_config.optimizers),
      packagers: PartialParcelConfig::merge_map(from_config.packagers, extend_config.packagers),
      reporters: PartialParcelConfig::merge_pipelines(
        from_config.reporters,
        extend_config.reporters,
      ),
      resolvers: PartialParcelConfig::merge_pipelines(
        from_config.resolvers,
        extend_config.resolvers,
      ),
      runtimes: PartialParcelConfig::merge_pipelines(from_config.runtimes, extend_config.runtimes),
      transformers: PartialParcelConfig::merge_maps(
        from_config.transformers,
        extend_config.transformers,
      ),
      validators: PartialParcelConfig::merge_maps(from_config.validators, extend_config.validators),
    }
  }
}
