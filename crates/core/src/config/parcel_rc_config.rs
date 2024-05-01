use std::{
  fs::canonicalize,
  path::{Path, PathBuf},
};

use pathdiff::diff_paths;

use crate::fs::file_system::FileSystem;
use crate::{diagnostic::diagnostic_error::DiagnosticError, package_manager::PackageManager};

use super::{parcel_rc::ParcelRcFile, partial_parcel_config::PartialParcelConfig};

pub struct ParcelRcConfig<'a, T, U> {
  fs: &'a T,
  package_manager: &'a U,
}

impl<'a, T: FileSystem, U: PackageManager> ParcelRcConfig<'a, T, U> {
  pub fn new(fs: &'a T, package_manager: &'a U) -> Self {
    ParcelRcConfig {
      fs,
      package_manager,
    }
  }

  fn resolve_config(
    &self,
    project_root: &Path,
    path: &PathBuf,
  ) -> Result<PathBuf, DiagnosticError> {
    // TODO Add caching?

    let from = path.parent().unwrap_or(path);

    self
      .fs
      .find_ancestor_file(vec![String::from(".parcelrc")], from, project_root)
      .ok_or(DiagnosticError::new(format!(
        "Unable to locate .parcelrc from {}",
        from.as_os_str().to_str().unwrap()
      )))
  }

  fn resolve_extends(
    &self,
    config_path: &PathBuf,
    extend: &String,
  ) -> Result<PathBuf, DiagnosticError> {
    let path = if extend.starts_with(".") {
      config_path.parent().unwrap_or(config_path).join(extend)
    } else {
      self.package_manager.resolve(extend, config_path)?.resolved
    };

    canonicalize(path).map_err(|source| {
      DiagnosticError::new_source(
        format!(
          "Unable to resolve extended config {} from {}",
          extend,
          config_path.as_os_str().to_str().unwrap()
        ),
        source,
      )
    })
  }

  fn process_config(
    &self,
    parcel_rc: &ParcelRcFile,
  ) -> Result<(PartialParcelConfig, Vec<PathBuf>), DiagnosticError> {
    // TODO Validation: e.g. empty, name format, etc
    // TODO Named reserved pipelines

    let mut files = vec![parcel_rc.path.clone()];
    let extends = parcel_rc.contents.extends.as_ref();
    let mut config = PartialParcelConfig::from(parcel_rc);
    if extends.is_none() || extends.is_some_and(|e| e.is_empty()) {
      return Ok((config, files));
    }

    // TODO Ensure extends can be an array / single value
    let extends = extends.unwrap();
    // let merged_config: Option<PartialParcelConfig> = None;
    // for extend in extends {
    //   let extended_file_path = self.resolve_extends(&parcel_rc.path, extend)?;

    //   files.push(extended_file_path.clone());

    //   let (extended_config, mut extended_file_paths) =
    //     self.process_config(&self.load_parcel_rc(extended_file_path)?)?;

    //   merged_config = match merged_config {
    //     None => Some(extended_config),
    //     Some(c) => Some(PartialParcelConfig::merge(c, extended_config)),
    //   };

    //   files.append(&mut extended_file_paths);
    // }

    // config.merge(merged_config.unwrap());

    Ok((config, files))
  }

  fn resolve_from(&self, project_root: &PathBuf) -> PathBuf {
    let cwd = self.fs.cwd();
    let relative = diff_paths(project_root, cwd.clone());
    // TODO check logic
    let is_cwd_inside_root = !relative.is_some_and(|p| p.starts_with("..") && p.is_absolute());
    let dir = if is_cwd_inside_root {
      &cwd
    } else {
      project_root
    };

    dir.join("index")
  }

  fn load_parcel_rc(&self, path: PathBuf) -> Result<ParcelRcFile, DiagnosticError> {
    let contents = serde_json5::from_str(&self.fs.read_file(&path)?)
      .map_err(|source| DiagnosticError::new(source.to_string()))?;

    Ok(ParcelRcFile { path, contents })
  }

  pub fn load(
    &self,
    project_root: &PathBuf,
    config: Option<String>,
    fallback_config: Option<String>,
  ) -> Result<(PartialParcelConfig, Vec<PathBuf>), DiagnosticError> {
    let resolve_from = self.resolve_from(project_root);
    let mut config_path = match config {
      Some(config) => self
        .package_manager
        .resolve(&config, &resolve_from)
        .map(|r| r.resolved),
      None => self.resolve_config(project_root, &resolve_from),
    };

    let mut used_fallback = false;
    if !config_path.is_ok() && fallback_config.is_some() {
      used_fallback = true;
      config_path = self
        .package_manager
        .resolve(&fallback_config.unwrap(), &resolve_from)
        .map(|r| r.resolved)
    }

    if config_path.is_err() {
      return Err(config_path.unwrap_err());
    }

    let config_path = config_path.unwrap();
    let parcel_config = self.process_config(&self.load_parcel_rc(config_path)?)?;

    //   TODO
    //   if (options.additionalReporters.length > 0) {
    //     config.reporters = [
    //       ...options.additionalReporters.map(({packageName, resolveFrom}) => ({
    //         packageName,
    //         resolveFrom,
    //       })),
    //       ...(config.reporters ?? []),
    //     ];
    //   }

    Ok(parcel_config)
  }
}

#[cfg(test)]
mod tests {
  use std::{env, rc::Rc};

  use indexmap::{indexmap, IndexMap};
  use mockall::predicate::eq;

  use crate::{
    config::parcel_config::PluginNode,
    package_manager::{MockPackageManager, Resolution},
  };

  use super::*;

  fn project_root() -> PathBuf {
    env::current_dir().unwrap()
  }

  fn fail_package_manager_resolution(package_manager: &mut MockPackageManager) {
    package_manager
      .expect_resolve()
      .return_once(|specifier, from| {
        Err(DiagnosticError::new(format!(
          "Failed to resolve {} from {}",
          specifier,
          from.display()
        )))
      });
  }

  fn package_manager_resolution(
    package_manager: &mut MockPackageManager,
    specifier: String,
    from: PathBuf,
  ) -> PathBuf {
    let resolved = project_root()
      .join("node_modules")
      .join(specifier.clone())
      .join("index.json");

    package_manager
      .expect_resolve()
      .with(eq(specifier), eq(from))
      .returning(move |specifier, _from| {
        Ok(Resolution {
          resolved: project_root()
            .join("node_modules")
            .join(specifier)
            .join("index.json"),
        })
      });

    resolved
  }

  fn to_partial_eq_parcel_config(
    config: (PartialParcelConfig, Vec<PathBuf>),
  ) -> (PartialParcelConfig, Vec<String>) {
    (
      config.0,
      config.1.iter().map(|p| p.display().to_string()).collect(),
    )
  }

  fn parcel_rc_fixture(resolve_from: Rc<String>) -> (String, PartialParcelConfig) {
    (
      String::from(
        r#"
          {
            "bundler": "@parcel/bundler-default",
            "compressors": {
              "*": ["@parcel/compressor-raw"]
            },
            "namers": ["@parcel/namer-default"],
            "optimizers": {
              "*.{js,mjs,cjs}": ["@parcel/optimizer-swc"]
            },
            "packagers": {
              "*.{js,mjs,cjs}": "@parcel/packager-js"
            },
            "reporters": ["@parcel/reporter-dev-server"],
            "resolvers": ["@parcel/resolver-default"],
            "runtimes": ["@parcel/runtime-js"],
            "transformers": {
              "*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}": [
                "@parcel/transformer-js"
              ],
            }
          }
        "#,
      ),
      PartialParcelConfig {
        bundler: Some(PluginNode {
          package_name: String::from("@parcel/bundler-default"),
          resolve_from: Rc::clone(&resolve_from),
        }),
        compressors: indexmap! {
          String::from("*") => vec!(PluginNode {
            package_name: String::from("@parcel/compressor-raw"),
            resolve_from: Rc::clone(&resolve_from),
          })
        },
        namers: vec![PluginNode {
          package_name: String::from("@parcel/namer-default"),
          resolve_from: Rc::clone(&resolve_from),
        }],
        optimizers: indexmap! {
          String::from("*.{js,mjs,cjs}") => vec!(PluginNode {
            package_name: String::from("@parcel/optimizer-swc"),
            resolve_from: Rc::clone(&resolve_from),
          })
        },
        packagers: indexmap! {
          String::from("*.{js,mjs,cjs}") => PluginNode {
            package_name: String::from("@parcel/packager-js"),
            resolve_from: Rc::clone(&resolve_from),
          }
        },
        reporters: vec![PluginNode {
          package_name: String::from("@parcel/reporter-dev-server"),
          resolve_from: Rc::clone(&resolve_from),
        }],
        resolvers: vec![PluginNode {
          package_name: String::from("@parcel/resolver-default"),
          resolve_from: Rc::clone(&resolve_from),
        }],
        runtimes: vec![PluginNode {
          package_name: String::from("@parcel/runtime-js"),
          resolve_from: Rc::clone(&resolve_from),
        }],
        transformers: indexmap! {
          String::from("*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}") => vec!(PluginNode {
            package_name: String::from("@parcel/transformer-js"),
            resolve_from: Rc::clone(&resolve_from),
          })
        },
        validators: IndexMap::new(),
      },
    )
  }

  mod empty_config_and_fallback {
    use std::collections::HashMap;

    use crate::fs::memory_file_system::MemoryFileSystem;

    use super::*;

    #[test]
    fn errors_on_missing_parcelrc_file() {
      let project_root = project_root();

      let err = ParcelRcConfig::new(&MemoryFileSystem::default(), &MockPackageManager::new()).load(
        &project_root,
        None,
        None,
      );

      assert_eq!(
        err.map_err(|e| e.to_string()),
        Err(
          DiagnosticError::new(format!(
            "Unable to locate .parcelrc from {}",
            project_root.display()
          ))
          .to_string()
        )
      );
    }

    #[test]
    fn returns_default_parcel_config() {
      let project_root = project_root();
      let (parcel_rc, expected_parcel_config) = parcel_rc_fixture(Rc::new(
        project_root.join(".parcelrc").display().to_string(),
      ));

      let parcel_config = ParcelRcConfig::new(
        &MemoryFileSystem::new(HashMap::from([(project_root.join(".parcelrc"), parcel_rc)])),
        &MockPackageManager::default(),
      )
      .load(&project_root, None, None);

      assert_eq!(
        parcel_config.map(to_partial_eq_parcel_config),
        Ok((
          expected_parcel_config,
          vec!(project_root.join(".parcelrc").display().to_string())
        ))
      );
    }
  }

  mod config {
    use std::{collections::HashMap, rc::Rc};

    use crate::{
      config::parcel_rc_config::{
        tests::{
          fail_package_manager_resolution, package_manager_resolution, parcel_rc_fixture,
          project_root, to_partial_eq_parcel_config, DiagnosticError,
        },
        ParcelRcConfig,
      },
      fs::memory_file_system::MemoryFileSystem,
      package_manager::MockPackageManager,
    };

    #[test]
    fn errors_on_unresolved_config_specifier() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      fail_package_manager_resolution(&mut package_manager);

      let err = ParcelRcConfig::new(&MemoryFileSystem::default(), &package_manager).load(
        &project_root,
        Some(String::from("@scope/config")),
        None,
      );

      assert_eq!(
        err,
        Err(DiagnosticError::new(format!(
          "Failed to resolve @scope/config from {}",
          project_root.join("index").display()
        )))
      );
    }

    #[test]
    fn errors_on_missing_config_file() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let config_path = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let err = ParcelRcConfig::new(
        &MemoryFileSystem::new(HashMap::from([(
          project_root.join(".parcelrc"),
          String::from("{}"),
        )])),
        &package_manager,
      )
      .load(&project_root, Some(String::from("@scope/config")), None);

      assert_eq!(
        err,
        Err(DiagnosticError::new(format!(
          "Failed to read file {}",
          config_path.display()
        )))
      );
    }

    #[test]
    fn returns_specified_config() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let config_path = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let files = vec![config_path.display().to_string()];
      let (config, expected_parcel_config) =
        parcel_rc_fixture(Rc::new(config_path.display().to_string()));

      let parcel_config = ParcelRcConfig::new(
        &MemoryFileSystem::new(HashMap::from([
          (project_root.join(".parcelrc"), String::from("{}")),
          (config_path, config),
        ])),
        &package_manager,
      )
      .load(&project_root, Some(String::from("@scope/config")), None);

      assert_eq!(
        parcel_config.map(to_partial_eq_parcel_config),
        Ok((expected_parcel_config, files))
      );
    }
  }

  mod fallback_config {
    use std::{collections::HashMap, rc::Rc};

    use crate::{
      config::parcel_rc_config::{
        tests::{
          fail_package_manager_resolution, package_manager_resolution, parcel_rc_fixture,
          project_root, to_partial_eq_parcel_config, DiagnosticError,
        },
        ParcelRcConfig,
      },
      fs::memory_file_system::MemoryFileSystem,
      package_manager::MockPackageManager,
    };

    #[test]
    fn errors_on_unresolved_fallback_specifier() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      fail_package_manager_resolution(&mut package_manager);

      let err = ParcelRcConfig::new(&MemoryFileSystem::default(), &package_manager).load(
        &project_root,
        Some(String::from("@scope/config")),
        None,
      );

      assert_eq!(
        err,
        Err(DiagnosticError::new(format!(
          "Failed to resolve @scope/config from {}",
          project_root.join("index").display()
        )))
      );
    }

    #[test]
    fn errors_on_missing_fallback_config_file() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let fallback_config_path = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let err = ParcelRcConfig::new(&MemoryFileSystem::default(), &package_manager).load(
        &project_root,
        None,
        Some(String::from("@scope/config")),
      );

      assert_eq!(
        err,
        Err(DiagnosticError::new(format!(
          "Failed to read file {}",
          fallback_config_path.display()
        )))
      );
    }

    #[test]
    fn returns_project_root_parcel_rc() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let fallback_config_path = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let (config, expected_parcel_config) = parcel_rc_fixture(Rc::new(
        project_root.join(".parcelrc").display().to_string(),
      ));

      let parcel_config = ParcelRcConfig::new(
        &MemoryFileSystem::new(HashMap::from([
          (project_root.join(".parcelrc"), config),
          (fallback_config_path, String::from("{}")),
        ])),
        &package_manager,
      )
      .load(&project_root, None, Some(String::from("@scope/config")));

      assert_eq!(
        parcel_config.map(to_partial_eq_parcel_config),
        Ok((
          expected_parcel_config,
          vec!(project_root.join(".parcelrc").display().to_string())
        ))
      );
    }

    #[test]
    fn returns_fallback_config_when_parcel_rc_is_missing() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let fallback_config_path = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let files = vec![fallback_config_path.display().to_string()];
      let (fallback_config, expected_parcel_config) =
        parcel_rc_fixture(Rc::new(fallback_config_path.display().to_string()));

      let parcel_config = ParcelRcConfig::new(
        &MemoryFileSystem::new(HashMap::from([(fallback_config_path, fallback_config)])),
        &package_manager,
      )
      .load(&project_root, None, Some(String::from("@scope/config")));

      assert_eq!(
        parcel_config.map(to_partial_eq_parcel_config),
        Ok((expected_parcel_config, files))
      );
    }
  }

  mod fallback_with_config {
    use std::{collections::HashMap, rc::Rc};

    use crate::{
      config::parcel_rc_config::{
        tests::{
          package_manager_resolution, parcel_rc_fixture, project_root, to_partial_eq_parcel_config,
        },
        ParcelRcConfig,
      },
      fs::memory_file_system::MemoryFileSystem,
      package_manager::MockPackageManager,
    };

    #[test]
    fn returns_specified_config() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let config_path = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let files = vec![config_path.display().to_string()];
      let (config, expected_parcel_config) =
        parcel_rc_fixture(Rc::new(config_path.display().to_string()));

      let parcel_config = ParcelRcConfig::new(
        &MemoryFileSystem::new(HashMap::from([
          (project_root.join(".parcelrc"), String::from("{}")),
          (config_path, config),
        ])),
        &package_manager,
      )
      .load(&project_root, Some(String::from("@scope/config")), None);

      assert_eq!(
        parcel_config.map(to_partial_eq_parcel_config),
        Ok((expected_parcel_config, files))
      );
    }

    #[test]
    fn returns_fallback_config_when_no_matching_config() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let fallback_config_path = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let files = vec![fallback_config_path.display().to_string()];
      let (falback_config, expected_parcel_config) =
        parcel_rc_fixture(Rc::new(fallback_config_path.display().to_string()));

      let parcel_config = ParcelRcConfig::new(
        &MemoryFileSystem::new(HashMap::from([
          (project_root.join(".parcelrc"), String::from("{}")),
          (fallback_config_path, falback_config),
        ])),
        &package_manager,
      )
      .load(&project_root, Some(String::from("@scope/config")), None);

      assert_eq!(
        parcel_config.map(to_partial_eq_parcel_config),
        Ok((expected_parcel_config, files))
      );
    }
  }

  mod validates {}
}
