use std::{
  collections::HashMap,
  fs::canonicalize,
  path::{Path, PathBuf},
};

use pathdiff::diff_paths;

use crate::{config::Config, fs::file_system::FileSystem};
use crate::{diagnostic::diagnostic_error::DiagnosticError, package_manager::PackageManager};

#[derive(Debug)]
pub struct ParcelRc {
  extends: Vec<String>,
  resolvers: Vec<String>,
  transformers: HashMap<String, String>,
  bundler: Option<String>,
  namers: Vec<String>,
  runtimes: Vec<String>,
  packagers: HashMap<String, String>,
  optimizers: HashMap<String, String>,
  validators: HashMap<String, String>,
  compressors: HashMap<String, String>,
  reporters: Vec<String>,
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
    path: &PathBuf,
    config: ParcelRc,
  ) -> Result<(Config, Vec<&Path>), DiagnosticError> {
    // TODO Check if validation needed or done by serde
    // TODO Named reserved pipelines

    let _files = vec![path];
    if config.extends.is_empty() {
      return Err(DiagnosticError::new(String::from("Unimplemented")));
      // return Ok((config, files));
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
  ) -> Result<Config, DiagnosticError> {
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
    let _config = self.fs.read_file(&config_path).map_err(|source| {
      DiagnosticError::new_source(
        format!(
          "Failed to read parcel config at {}",
          diff_paths(config_path.clone(), project_root)
            .unwrap_or(config_path)
            .as_os_str()
            .to_str()
            .unwrap()
        ),
        source,
      )
    })?;

    // let mut parcel_config = self.process_config(
    //   &config_path,
    //   serde_json5::from_str(&config).map_err(|e| {
    //     DiagnosticError::new(
    //       &format!(
    //         "Failed to parse .parcelrc at {}",
    //         &config_path.as_os_str().to_str().unwrap()
    //       )
    //       .as_str(),
    //     )
    //   })?,
    // );

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

    return Err(DiagnosticError::new(String::from("Unimplemented")));
    // return {config, extendedFiles, usedDefault};

    // Ok(parcel_config)
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

  mod empty_config_and_fallback {
    use crate::fs::memory_file_system::MemoryFileSystem;

    use super::*;

    #[test]
    fn errors_on_unfound_parcelrc() {
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

      let config = ParcelConfig::new(
        &MemoryFileSystem::new(HashMap::from([(
          project_root.join(".parcelrc"),
          String::from("{}"),
        )])),
        &MockPackageManager::new(),
      )
      .load(&project_root, None, None);

      // TODO...
      assert_eq!(
        config.map_err(|e| e.to_string()),
        Err(DiagnosticError::new(String::from("Unimplemented",)).to_string())
      );
    }
  }

  mod config {
    use std::collections::HashMap;

    use crate::{
      fs::memory_file_system::MemoryFileSystem,
      package_manager::MockPackageManager,
      parcel_config::{
        tests::{
          fail_package_manager_resolution, package_manager_resolution, project_root,
          DiagnosticError,
        },
        ParcelConfig,
      },
    };

    #[test]
    fn errors_on_unfound_parcelrc_specifier() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      fail_package_manager_resolution(&mut package_manager);

      let err = ParcelConfig::new(&MemoryFileSystem::default(), &package_manager).load(
        &project_root,
        Some(String::from("@scope/config")),
        None,
      );

      assert_eq!(
        err.map_err(|e| e.to_string()),
        Err(
          DiagnosticError::new(format!(
            "Failed to resolve @scope/config from {}",
            project_root.join("index").display()
          ))
          .to_string()
        )
      );
    }

    #[test]
    fn errors_on_parcelrc_file() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let err = ParcelConfig::new(&MemoryFileSystem::default(), &package_manager).load(
        &project_root,
        Some(String::from("@scope/config")),
        None,
      );

      assert_eq!(
        err.map_err(|e| e.to_string()),
        Err(
          DiagnosticError::new(String::from(
            "Failed to read parcel config at node_modules/@scope/config/index.json: Failed to read file",
          ))
          .to_string()
        )
      );
    }

    #[test]
    fn returns_specified_config() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let scope_config = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let config = ParcelConfig::new(
        &MemoryFileSystem::new(HashMap::from([
          (project_root.join(".parcelrc"), String::from("{}")),
          (scope_config, String::from("{}")),
        ])),
        &package_manager,
      )
      .load(&project_root, Some(String::from("@scope/config")), None);

      // TODO...
      assert_eq!(
        config.map_err(|e| e.to_string()),
        Err(DiagnosticError::new(String::from("Unimplemented",)).to_string())
      );
    }
  }

  mod fallback_config {
    use crate::{
      fs::memory_file_system::MemoryFileSystem,
      package_manager::MockPackageManager,
      parcel_config::{
        tests::{
          fail_package_manager_resolution, package_manager_resolution, project_root,
          DiagnosticError,
        },
        ParcelConfig,
      },
    };

    #[test]
    fn errors_on_unfound_parcelrc_specifier() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      fail_package_manager_resolution(&mut package_manager);

      let err = ParcelConfig::new(&MemoryFileSystem::default(), &package_manager).load(
        &project_root,
        Some(String::from("@scope/config")),
        None,
      );

      assert_eq!(
        err.map_err(|e| e.to_string()),
        Err(
          DiagnosticError::new(format!(
            "Failed to resolve @scope/config from {}",
            project_root.join("index").display()
          ))
          .to_string()
        )
      );
    }

    #[test]
    fn errors_on_parcelrc_file() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      package_manager_resolution(
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
        err.map_err(|e| e.to_string()),
        Err(
          DiagnosticError::new(String::from(
            "Failed to read parcel config at node_modules/@scope/config/index.json: Failed to read file",
          ))
          .to_string()
        )
      );
    }
  }

  mod fallback_with_config {
    use std::collections::HashMap;

    use crate::{
      fs::memory_file_system::MemoryFileSystem,
      package_manager::MockPackageManager,
      parcel_config::{
        tests::{package_manager_resolution, project_root, DiagnosticError},
        ParcelConfig,
      },
    };

    #[test]
    fn returns_specified_config() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let scope_config = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let config = ParcelConfig::new(
        &MemoryFileSystem::new(HashMap::from([
          (project_root.join(".parcelrc"), String::from("{}")),
          (scope_config, String::from("{}")),
        ])),
        &package_manager,
      )
      .load(&project_root, Some(String::from("@scope/config")), None);

      // TODO...
      assert_eq!(
        config.map_err(|e| e.to_string()),
        Err(DiagnosticError::new(String::from("Unimplemented",)).to_string())
      );
    }

    #[test]
    fn returns_fallback_config_when_no_matching_config() {
      let project_root = project_root();
      let mut package_manager = MockPackageManager::new();

      let scope_config = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let config = ParcelConfig::new(
        &MemoryFileSystem::new(HashMap::from([
          (project_root.join(".parcelrc"), String::from("{}")),
          (scope_config, String::from("{}")),
        ])),
        &package_manager,
      )
      .load(&project_root, Some(String::from("@scope/config")), None);

      // TODO...
      assert_eq!(
        config.map_err(|e| e.to_string()),
        Err(DiagnosticError::new(String::from("Unimplemented",)).to_string())
      );
    }
  }

  mod validates {}
}
