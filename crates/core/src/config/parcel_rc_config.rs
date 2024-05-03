use std::path::{Path, PathBuf};

use pathdiff::diff_paths;

use crate::fs::file_system::FileSystem;
use crate::{diagnostic::diagnostic_error::DiagnosticError, package_manager::PackageManager};

use super::{
  parcel_config::ParcelConfig,
  parcel_rc::{Extends, ParcelRcFile},
  partial_parcel_config::PartialParcelConfig,
};

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
      self
        .package_manager
        .resolve(extend, config_path)
        .map_err(|_source| {
          DiagnosticError::new(format!(
            "Failed to resolve extended config {} from {}",
            extend,
            config_path.display()
          ))
        })?
        .resolved
    };

    self.fs.canonicalize(path).map_err(|_source| {
      DiagnosticError::new(format!(
        "Failed to resolve extended config {} from {}",
        extend,
        config_path.display()
      ))
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
    if extends.is_none()
      || extends.is_some_and(|extends| match extends {
        Extends::One(ext) => ext.is_empty(),
        Extends::Many(ext) => ext.is_empty(),
      })
    {
      return Ok((PartialParcelConfig::from(parcel_rc), files));
    }

    let extends = match extends.unwrap() {
      Extends::One(ext) => vec![String::from(ext)],
      Extends::Many(ext) => ext.to_vec(),
    };

    let mut merged_config: Option<PartialParcelConfig> = None;
    for extend in extends {
      let extended_file_path = self.resolve_extends(&parcel_rc.path, &extend)?;
      let (extended_config, mut extended_file_paths) =
        self.process_config(&self.load_parcel_rc(extended_file_path)?)?;

      merged_config = match merged_config {
        None => Some(extended_config),
        Some(config) => Some(PartialParcelConfig::merge(config, extended_config)),
      };

      files.append(&mut extended_file_paths);
    }

    let config =
      PartialParcelConfig::merge(PartialParcelConfig::from(parcel_rc), merged_config.unwrap());

    Ok((config, files))
  }

  fn resolve_from(&self, project_root: &PathBuf) -> PathBuf {
    let cwd = self.fs.cwd();
    let relative = diff_paths(cwd.clone(), project_root);
    let is_cwd_inside_project_root =
      relative.is_some_and(|p| !p.starts_with("..") && !p.is_absolute());

    let dir = if is_cwd_inside_project_root {
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
  ) -> Result<(ParcelConfig, Vec<PathBuf>), DiagnosticError> {
    let resolve_from = self.resolve_from(project_root);
    let mut config_path = match config {
      Some(config) => self
        .package_manager
        .resolve(&config, &resolve_from)
        .map(|r| r.resolved),
      None => self.resolve_config(project_root, &resolve_from),
    };

    let mut _used_fallback = false;
    if !config_path.is_ok() && fallback_config.is_some() {
      _used_fallback = true;
      config_path = self
        .package_manager
        .resolve(&fallback_config.unwrap(), &resolve_from)
        .map(|r| r.resolved)
    }

    if config_path.is_err() {
      return Err(config_path.unwrap_err());
    }

    let config_path = config_path.unwrap();
    let (parcel_config, files) = self.process_config(&self.load_parcel_rc(config_path)?)?;
    let parcel_config = ParcelConfig::try_from(parcel_config)?;

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

    Ok((parcel_config, files))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{env, rc::Rc};

  use mockall::predicate::eq;

  use crate::{
    fs::memory_file_system::MemoryFileSystem,
    package_manager::{MockPackageManager, Resolution},
  };

  fn cwd() -> PathBuf {
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

  struct InMemoryPackageManager<'a> {
    fs: &'a MemoryFileSystem,
  }

  impl<'a> InMemoryPackageManager<'a> {
    pub fn new(fs: &'a MemoryFileSystem) -> Self {
      Self { fs }
    }
  }

  impl<'a> PackageManager for InMemoryPackageManager<'a> {
    fn resolve(&self, specifier: &String, from: &Path) -> Result<Resolution, DiagnosticError> {
      let path = match "true" {
        _s if specifier.starts_with(".") => from.join(specifier),
        _s if specifier.starts_with("@") => self
          .fs
          .cwd()
          .join("node_modules")
          .join(specifier)
          .join("index.json"),
        _ => PathBuf::from("Not found"),
      };

      if !self.fs.is_file(&path) {
        return Err(DiagnosticError::new(format!(
          "Failed to resolve {} from {}",
          specifier,
          from.display()
        )));
      }

      Ok(Resolution { resolved: path })
    }
  }

  fn package_manager_resolution(
    package_manager: &mut MockPackageManager,
    specifier: String,
    from: PathBuf,
  ) -> PathBuf {
    let resolved = cwd()
      .join("node_modules")
      .join(specifier.clone())
      .join("index.json");

    package_manager
      .expect_resolve()
      .with(eq(specifier), eq(from))
      .returning(|specifier, _from| {
        Ok(Resolution {
          resolved: cwd()
            .join("node_modules")
            .join(specifier)
            .join("index.json"),
        })
      });

    resolved
  }

  mod empty_config_and_fallback {
    use crate::config::parcel_config_fixtures::{default_config, default_extended_config};

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn errors_on_missing_parcelrc_file() {
      let project_root = cwd();

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
    fn errors_on_failed_extended_parcelrc_resolution() {
      let project_root = cwd();
      let config = default_extended_config(&project_root);
      let fs = MemoryFileSystem::new(HashMap::from([(
        config.base_config.path.clone(),
        config.base_config.parcel_rc,
      )]));

      let err =
        ParcelRcConfig::new(&fs, &InMemoryPackageManager::new(&fs)).load(&project_root, None, None);

      assert_eq!(
        err.map_err(|e| e.to_string()),
        Err(
          DiagnosticError::new(format!(
            "Failed to resolve extended config @parcel/config-default from {}",
            config.base_config.path.display(),
          ))
          .to_string()
        )
      );
    }

    #[test]
    fn returns_default_parcel_config() {
      let project_root = cwd();
      let default_config = default_config(&Rc::new(project_root.join(".parcelrc")));
      let files = vec![default_config.path.clone()];

      let parcel_config = ParcelRcConfig::new(
        &MemoryFileSystem::new(HashMap::from([(
          default_config.path,
          default_config.parcel_rc,
        )])),
        &MockPackageManager::default(),
      )
      .load(&project_root, None, None);

      assert_eq!(parcel_config, Ok((default_config.parcel_config, files)));
    }

    #[test]
    fn returns_default_parcel_config_from_project_root() {
      let project_root = cwd().join("src").join("packages").join("root");
      let default_config = default_config(&Rc::new(project_root.join(".parcelrc")));
      let files = vec![default_config.path.clone()];

      let parcel_config = ParcelRcConfig::new(
        &MemoryFileSystem::new(HashMap::from([(
          default_config.path,
          default_config.parcel_rc,
        )])),
        &MockPackageManager::default(),
      )
      .load(&project_root, None, None);

      assert_eq!(parcel_config, Ok((default_config.parcel_config, files)));
    }

    #[test]
    fn returns_default_parcel_config_from_project_root_when_outside_cwd() {
      let project_root = PathBuf::from("/root");
      let default_config = default_config(&Rc::new(project_root.join(".parcelrc")));
      let files = vec![default_config.path.clone()];

      let parcel_config = ParcelRcConfig::new(
        &MemoryFileSystem::new(HashMap::from([(
          default_config.path,
          default_config.parcel_rc,
        )])),
        &MockPackageManager::default(),
      )
      .load(&project_root, None, None);

      assert_eq!(parcel_config, Ok((default_config.parcel_config, files)));
    }

    #[test]
    fn returns_merged_default_parcel_config() {
      let project_root = cwd();
      let default_config = default_extended_config(&project_root);
      let files = vec![
        default_config.base_config.path.clone(),
        default_config.extended_config.path.clone(),
      ];

      let fs = MemoryFileSystem::new(HashMap::from([
        (
          default_config.base_config.path,
          default_config.base_config.parcel_rc,
        ),
        (
          default_config.extended_config.path,
          default_config.extended_config.parcel_rc,
        ),
      ]));

      let parcel_config =
        ParcelRcConfig::new(&fs, &InMemoryPackageManager::new(&fs)).load(&project_root, None, None);

      assert_eq!(parcel_config, Ok((default_config.parcel_config, files)));
    }
  }

  mod config {
    use crate::config::parcel_config_fixtures::{config, extended_config};

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn errors_on_failed_config_resolution() {
      let project_root = cwd();
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
    fn errors_on_failed_extended_config_resolution() {
      let project_root = cwd();
      let (specifier, config) = extended_config(&project_root);
      let fs = MemoryFileSystem::new(HashMap::from([(
        config.base_config.path.clone(),
        config.base_config.parcel_rc,
      )]));

      let err = ParcelRcConfig::new(&fs, &InMemoryPackageManager::new(&fs)).load(
        &project_root,
        Some(specifier),
        None,
      );

      assert_eq!(
        err.map_err(|e| e.to_string()),
        Err(
          DiagnosticError::new(format!(
            "Failed to resolve extended config @parcel/config-default from {}",
            config.base_config.path.display()
          ))
          .to_string()
        )
      );
    }

    #[test]
    fn errors_on_missing_config_file() {
      let project_root = cwd();
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
      let project_root = cwd();
      let (specifier, specified_config) = config(&project_root);
      let files = vec![specified_config.path.clone()];

      let fs = MemoryFileSystem::new(HashMap::from([
        (project_root.join(".parcelrc"), String::from("{}")),
        (specified_config.path, specified_config.parcel_rc),
      ]));

      let parcel_config = ParcelRcConfig::new(&fs, &InMemoryPackageManager::new(&fs)).load(
        &project_root,
        Some(specifier),
        None,
      );

      assert_eq!(parcel_config, Ok((specified_config.parcel_config, files)));
    }
  }

  mod fallback_config {
    use crate::config::parcel_config_fixtures::{default_config, extended_config, fallback_config};

    use super::*;
    use std::{collections::HashMap, rc::Rc};

    #[test]
    fn errors_on_failed_fallback_resolution() {
      let project_root = cwd();
      let mut package_manager = MockPackageManager::new();

      fail_package_manager_resolution(&mut package_manager);

      let err = ParcelRcConfig::new(&MemoryFileSystem::default(), &package_manager).load(
        &project_root,
        None,
        Some(String::from("@parcel/config-default")),
      );

      assert_eq!(
        err,
        Err(DiagnosticError::new(format!(
          "Failed to resolve @parcel/config-default from {}",
          project_root.join("index").display()
        )))
      );
    }

    #[test]
    fn errors_on_failed_extended_fallback_config_resolution() {
      let project_root = cwd();
      let (fallback_specifier, fallback) = extended_config(&project_root);

      let fs = MemoryFileSystem::new(HashMap::from([(
        fallback.base_config.path.clone(),
        fallback.base_config.parcel_rc,
      )]));

      let err = ParcelRcConfig::new(&fs, &InMemoryPackageManager::new(&fs)).load(
        &project_root,
        None,
        Some(fallback_specifier),
      );

      assert_eq!(
        err.map_err(|e| e.to_string()),
        Err(
          DiagnosticError::new(format!(
            "Failed to resolve extended config @parcel/config-default from {}",
            fallback.base_config.path.display()
          ))
          .to_string()
        )
      );
    }

    #[test]
    fn errors_on_missing_fallback_config_file() {
      let project_root = cwd();
      let mut package_manager = MockPackageManager::new();

      let fallback_config_path = package_manager_resolution(
        &mut package_manager,
        String::from("@parcel/config-default"),
        project_root.join("index"),
      );

      let err = ParcelRcConfig::new(&MemoryFileSystem::default(), &package_manager).load(
        &project_root,
        None,
        Some(String::from("@parcel/config-default")),
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
      let project_root = cwd();
      let (fallback_specifier, fallback) = fallback_config(&project_root);
      let project_root_config = default_config(&Rc::new(project_root.join(".parcelrc")));

      let fs = MemoryFileSystem::new(HashMap::from([
        (
          project_root_config.path.clone(),
          project_root_config.parcel_rc,
        ),
        (fallback.path, String::from("{}")),
      ]));

      let parcel_config = ParcelRcConfig::new(&fs, &InMemoryPackageManager::new(&fs)).load(
        &project_root,
        None,
        Some(fallback_specifier),
      );

      assert_eq!(
        parcel_config,
        Ok((
          project_root_config.parcel_config,
          vec!(project_root_config.path)
        ))
      );
    }

    #[test]
    fn returns_fallback_config_when_parcel_rc_is_missing() {
      let project_root = cwd();
      let (fallback_specifier, fallback) = fallback_config(&project_root);
      let files = vec![fallback.path.clone()];
      let fs = MemoryFileSystem::new(HashMap::from([(fallback.path, fallback.parcel_rc)]));

      let parcel_config = ParcelRcConfig::new(&fs, &InMemoryPackageManager::new(&fs)).load(
        &project_root,
        None,
        Some(fallback_specifier),
      );

      assert_eq!(parcel_config, Ok((fallback.parcel_config, files)));
    }
  }

  mod fallback_with_config {
    use crate::config::parcel_config_fixtures::{config, fallback_config};

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn returns_specified_config() {
      let project_root = cwd();
      let (config_specifier, config) = config(&project_root);
      let (fallback_config_specifier, fallback_config) = fallback_config(&project_root);
      let files = vec![config.path.clone()];

      let fs = MemoryFileSystem::new(HashMap::from([
        (config.path, config.parcel_rc),
        (fallback_config.path, fallback_config.parcel_rc),
      ]));

      let parcel_config = ParcelRcConfig::new(&fs, &InMemoryPackageManager::new(&fs)).load(
        &project_root,
        Some(config_specifier),
        Some(fallback_config_specifier),
      );

      assert_eq!(parcel_config, Ok((config.parcel_config, files)));
    }

    #[test]
    fn returns_fallback_config_when_config_file_missing() {
      let project_root = cwd();
      let (config_specifier, _config) = config(&project_root);
      let (fallback_specifier, fallback) = fallback_config(&project_root);
      let files = vec![fallback.path.clone()];
      let fs = MemoryFileSystem::new(HashMap::from([(fallback.path, fallback.parcel_rc)]));

      let parcel_config = ParcelRcConfig::new(&fs, &InMemoryPackageManager::new(&fs)).load(
        &project_root,
        Some(config_specifier),
        Some(fallback_specifier),
      );

      assert_eq!(parcel_config, Ok((fallback.parcel_config, files)));
    }
  }

  mod validates {}
}
