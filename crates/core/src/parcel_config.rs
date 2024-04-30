use std::{
  collections::HashMap,
  fs::canonicalize,
  path::{Path, PathBuf},
};

use pathdiff::diff_paths;
use serde_derive::Deserialize;

use crate::{config::PartialConfig, fs::file_system::FileSystem};
use crate::{diagnostic::diagnostic_error::DiagnosticError, package_manager::PackageManager};

#[derive(Debug, Deserialize)]
pub struct ParcelRcContents {
  pub extends: Option<Vec<String>>,
  pub resolvers: Option<Vec<String>>,
  pub transformers: Option<HashMap<String, Vec<String>>>,
  pub bundler: Option<String>,
  pub namers: Option<Vec<String>>,
  pub runtimes: Option<Vec<String>>,
  pub packagers: Option<Vec<(String, String)>>,
  // pub packagers: Option<HashMap<String, String>>,
  pub optimizers: Option<HashMap<String, Vec<String>>>,
  pub validators: Option<HashMap<String, String>>,
  pub compressors: Option<HashMap<String, String>>,
  pub reporters: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct ParcelRc {
  pub path: PathBuf,
  pub contents: ParcelRcContents,
}

pub struct ParcelConfig<'a, T, U> {
  fs: &'a T,
  package_manager: &'a U,
}

impl<'a, T: FileSystem, U: PackageManager> ParcelConfig<'a, T, U> {
  pub fn new(fs: &'a T, package_manager: &'a U) -> Self {
    ParcelConfig {
      fs,
      package_manager,
    }
  }

  fn resolve_config(
    &self,
    project_root: &Path,
    path: &PathBuf,
  ) -> Result<PathBuf, DiagnosticError> {
    // TODO Add caching

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
    parcel_rc: &ParcelRc,
  ) -> Result<(PartialConfig, Vec<PathBuf>), DiagnosticError> {
    // TODO Validation: e.g. empty, name format, etc
    // TODO Named reserved pipelines

    let files = vec![parcel_rc.path.clone()];
    let extends = parcel_rc.contents.extends.as_ref();
    if extends.is_none() || extends.is_some_and(|e| e.is_empty()) {
      return Ok((PartialConfig::from(parcel_rc), files));
    }

    // let errors;
    // TODO Ensure array extends in serde?
    // config.extends.iter().flat_map(|config| {
    //   let extended_file = self.resolve_extends(path, ext);
    //   files.push(extended_file);
    // });
    // for (let ext of exts) {
    //   try {
    //     let resolved = await resolveExtends(ext, filePath, key, options);
    //     extendedFiles.push(resolved);
    //     let {extendedFiles: moreExtendedFiles, config: nextConfig} =
    //       await processExtendedConfig(filePath, key, ext, resolved, options);
    //     extendedFiles = extendedFiles.concat(moreExtendedFiles);
    //     extStartConfig = extStartConfig
    //       ? mergeConfigs(extStartConfig, nextConfig)
    //       : nextConfig;
    //   } catch (err) {
    //     errors.push(err);
    //   }
    // }

    // if errors {
    // return Err(DiagnosticError::new(String::from("Lots of errors")));
    // throw new ThrowableDiagnostic({
    //   diagnostic: errors.flatMap(e => e.diagnostics),
    // });
    // }

    return Err(DiagnosticError::new(String::from("Unimplemented")));
    // Ok((config, files))
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

  pub fn load(
    &self,
    project_root: &PathBuf,
    config: Option<String>,
    fallback_config: Option<String>,
  ) -> Result<(PartialConfig, Vec<PathBuf>), DiagnosticError> {
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
    let config = self.fs.read_file(&config_path)?;

    let mut parcel_config = self.process_config(&ParcelRc {
      path: config_path,
      contents: serde_json5::from_str(&config)
        .map_err(|source| DiagnosticError::new(source.to_string()))?,
    })?;

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

    // return Err(DiagnosticError::new(String::from("Unimplemented")));
    // return {config, extendedFiles, usedDefault};

    Ok(parcel_config)
  }
}

#[cfg(test)]
mod tests {
  use std::env;

  use mockall::predicate::eq;

  use crate::package_manager::{MockPackageManager, Resolution};

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

  fn package_manager_resolution<'a>(
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
    config: (PartialConfig, Vec<PathBuf>),
  ) -> (PartialConfig, Vec<String>) {
    (
      config.0,
      config.1.iter().map(|p| p.display().to_string()).collect(),
    )
  }

  mod empty_config_and_fallback {
    use crate::fs::memory_file_system::MemoryFileSystem;

    use super::*;

    #[test]
    fn errors_on_missing_parcelrc_file() {
      let project_root = project_root();

      let err = ParcelConfig::new(&MemoryFileSystem::default(), &MockPackageManager::new()).load(
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

      let parcel_config = ParcelConfig::new(
        &MemoryFileSystem::new(HashMap::from([(
          project_root.join(".parcelrc"),
          String::from("{}"),
        )])),
        &MockPackageManager::new(),
      )
      .load(&project_root, None, None);

      assert_eq!(
        parcel_config.map(to_partial_eq_parcel_config),
        Ok((
          PartialConfig::default(),
          vec!(project_root.join(".parcelrc").display().to_string())
        ))
      );
    }
  }

  mod config {
    use std::collections::HashMap;

    use crate::{
      config::PartialConfig,
      fs::memory_file_system::MemoryFileSystem,
      package_manager::MockPackageManager,
      parcel_config::{
        tests::{
          fail_package_manager_resolution, package_manager_resolution, project_root,
          to_partial_eq_parcel_config, DiagnosticError,
        },
        ParcelConfig,
      },
    };

    #[test]
    fn errors_on_unresolved_config_specifier() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      fail_package_manager_resolution(&mut package_manager);

      let err = ParcelConfig::new(&MemoryFileSystem::default(), &package_manager).load(
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

      let config = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let err = ParcelConfig::new(
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
          config.display()
        )))
      );
    }

    #[test]
    fn returns_specified_config() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let config = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let files = vec![config.display().to_string()];

      let parcel_config = ParcelConfig::new(
        &MemoryFileSystem::new(HashMap::from([
          (project_root.join(".parcelrc"), String::from("{}")),
          (config, String::from("{}")),
        ])),
        &package_manager,
      )
      .load(&project_root, Some(String::from("@scope/config")), None);

      assert_eq!(
        parcel_config.map(to_partial_eq_parcel_config),
        Ok((PartialConfig::default(), files))
      );
    }
  }

  mod fallback_config {
    use std::collections::HashMap;

    use crate::{
      config::PartialConfig,
      fs::memory_file_system::MemoryFileSystem,
      package_manager::MockPackageManager,
      parcel_config::{
        tests::{
          fail_package_manager_resolution, package_manager_resolution, project_root,
          to_partial_eq_parcel_config, DiagnosticError,
        },
        ParcelConfig,
      },
    };

    #[test]
    fn errors_on_unresolved_fallback_specifier() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      fail_package_manager_resolution(&mut package_manager);

      let err = ParcelConfig::new(&MemoryFileSystem::default(), &package_manager).load(
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

      let fallback_config = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let err = ParcelConfig::new(&MemoryFileSystem::default(), &package_manager).load(
        &project_root,
        None,
        Some(String::from("@scope/config")),
      );

      assert_eq!(
        err,
        Err(DiagnosticError::new(format!(
          "Failed to read file {}",
          fallback_config.display()
        )))
      );
    }

    #[test]
    fn returns_project_root_parcel_rc() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let fallback_config = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let parcel_config = ParcelConfig::new(
        &MemoryFileSystem::new(HashMap::from([
          (project_root.join(".parcelrc"), String::from("{}")),
          (fallback_config, String::from("{}")),
        ])),
        &package_manager,
      )
      .load(&project_root, None, Some(String::from("@scope/config")));

      assert_eq!(
        parcel_config.map(to_partial_eq_parcel_config),
        Ok((
          PartialConfig::default(),
          vec!(project_root.join(".parcelrc").display().to_string())
        ))
      );
    }

    #[test]
    fn returns_fallback_config_when_parcel_rc_is_missing() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let fallback_config = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let files = vec![fallback_config.display().to_string()];

      let parcel_config = ParcelConfig::new(
        &MemoryFileSystem::new(HashMap::from([(fallback_config, String::from("{}"))])),
        &package_manager,
      )
      .load(&project_root, None, Some(String::from("@scope/config")));

      assert_eq!(
        parcel_config.map(to_partial_eq_parcel_config),
        Ok((PartialConfig::default(), files))
      );
    }
  }

  mod fallback_with_config {
    use std::collections::HashMap;

    use crate::{
      config::PartialConfig,
      fs::memory_file_system::MemoryFileSystem,
      package_manager::MockPackageManager,
      parcel_config::{
        tests::{package_manager_resolution, project_root, to_partial_eq_parcel_config},
        ParcelConfig,
      },
    };

    #[test]
    fn returns_specified_config() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let config = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let files = vec![config.display().to_string()];

      let parcel_config = ParcelConfig::new(
        &MemoryFileSystem::new(HashMap::from([
          (project_root.join(".parcelrc"), String::from("{}")),
          (config, String::from("{}")),
        ])),
        &package_manager,
      )
      .load(&project_root, Some(String::from("@scope/config")), None);

      assert_eq!(
        parcel_config.map(to_partial_eq_parcel_config),
        Ok((PartialConfig::default(), files))
      );
    }

    #[test]
    fn returns_fallback_config_when_no_matching_config() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let fallback_config = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let files = vec![fallback_config.display().to_string()];

      let parcel_config = ParcelConfig::new(
        &MemoryFileSystem::new(HashMap::from([
          (project_root.join(".parcelrc"), String::from("{}")),
          (fallback_config, String::from("{}")),
        ])),
        &package_manager,
      )
      .load(&project_root, Some(String::from("@scope/config")), None);

      assert_eq!(
        parcel_config.map(to_partial_eq_parcel_config),
        Ok((PartialConfig::default(), files))
      );
    }
  }

  mod validates {}
}
