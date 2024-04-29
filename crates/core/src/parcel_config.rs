use std::{
  collections::HashMap,
  fs::canonicalize,
  path::{Path, PathBuf},
};

use pathdiff::diff_paths;

use crate::diagnostic_error::DiagnosticError;
use crate::{config::Config, fs::Fs};

struct Resolution {
  resolved: PathBuf,
}

trait PackageManager {
  fn resolve(&self, specifier: &String, from: &Path) -> Result<Resolution, DiagnosticError>;
}

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

pub struct ParcelConfig<T, U> {
  fs: T,
  package_manager: U,
}

impl<T: Fs, U: PackageManager> ParcelConfig<T, U> {
  pub fn new(fs: T, package_manager: U) -> Self {
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

    let files = vec![path];
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
      return Err(DiagnosticError::new_source(
        String::from("Unable to locate .parcelrc"),
        config_path.unwrap_err(),
      ));
    }

    let config_path = config_path.unwrap();
    let config = self.fs.read_file(&config_path).map_err(|source| {
      DiagnosticError::new_source(
        format!(
          "Unable to locate parcel config at {}",
          diff_paths(project_root, config_path.clone())
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
  use crate::{fs::FileSystem, parcel_config::ParcelConfig};

  #[test]
  fn errors_on_unfound_parcelrc() {
    let config = ParcelConfig::new(FileSystem::new(), 1);

    assert_eq!(config.load(), 4);
  }

  fn errors_on_unfound_parcelrc_path() {
    let result = 2 + 2;
    assert_eq!(result, 4);
  }
}
