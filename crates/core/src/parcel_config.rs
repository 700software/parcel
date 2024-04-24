use std::{
  fs::canonicalize,
  path::{Path, PathBuf},
};

use pathdiff::diff_paths;

use crate::config::Config;

struct Fs {
  cwd: fn() -> PathBuf,
  find_ancestor_file: fn(files: Vec<String>) -> String,
  read_file: fn(path: &PathBuf, encoding: String) -> Result<String, String>,
}

struct PackageManager {
  resolve: fn(specifier: String, from: &Path) -> Result<(), String>,
}

pub struct ParcelConfig {
  fs: Fs,
  package_manager: PackageManager,
}

impl ParcelConfig {
  fn resolve_config(&self, project_root: &Path, path: &PathBuf) -> Result<&Path, String> {
    // TODO Add caching

    let resolved = self
      .fs
      .find_ancestor_file([".parcelrc"], path.parent(), project_root)?;

    Ok(resolved)
  }

  fn resolve_extends(&self, config_path: &Path, extend: &String) -> Result<PathBuf, String> {
    if extend.starts_with(".") {
      let dir = config_path.parent().unwrap_or(config_path);
      return canonicalize(dir.join(extend)).map_err(|e| e.to_string());
    }

    let resolution = self.package_manager.resolve(extend, config_path)?;

    // TODO Error handling
    canonicalize(resolution.resolved).map_err(|e| e.to_string())
  }

  fn process_config(
    &self,
    path: &Path,
    config: Config,
  ) -> Result<(Config, Vec<&Path>), impl AsRef<str>> {
    // TODO Check if validation needed or done by serde
    // TODO Named reserved pipelines

    let files = vec![path];
    if (config.extends.is_empty()) {
      return Ok((config, files));
    }

    let errors;
    // TODO Ensure array extends in serde?
    config.extends.iter().flat_map(|config| {
      let extended_file = self.resolve_extends(path, ext);
      files.push(extended_file);
    });
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

    if errors {
      return Err("Lots of errors");
      // throw new ThrowableDiagnostic({
      //   diagnostic: errors.flatMap(e => e.diagnostics),
      // });
    }

    Ok((config, files));
  }

  pub fn load(
    &self,
    project_root: &Path,
    config: Option<String>,
    fallback_config: Option<String>,
  ) -> Result<(), impl AsRef<str>> {
    let cwd = self.fs.cwd();
    let resolve_from = {
      let relative = diff_paths(project_root, cwd);
      // TODO check logic
      let is_cwd_inside_root = !relative.is_some_and(|p| p.starts_with("..") && p.is_absolute());
      let dir = if is_cwd_inside_root {
        cwd
      } else {
        project_root
      };

      dir.join("index")
    };

    let config_path = match config {
      Some(config) => self.package_manager.resolve(config, resolve_from)?.resolved,
      None => self.resolve_config(project_root, &resolve_from),
    };

    let used_fallback = false;
    if !config_path.is_ok() && fallback_config.is_some() {
      used_fallback = true;
      config_path = self
        .package_manager
        .resolve(fallback_config, resolve_from)?
        .resolved;
    }

    if config_path.is_err() {
      return Err("Unable to locate .parcelrc");
    }

    let config_path = config_path.unwrap();
    let config = self.fs.read_file(config_path, "utf8").map_err(|e| {
      format!(
        "Unable to locate parcel config at {}",
        diff_paths(project_root, config_path).unwrap_or_default(config_path),
      )
    })?;

    let mut parcel_config = self.process_config(
      config_path,
      serde_json5::from_str(config)
        .map_err(|e| format!("Failed to parse .parcelrc at {}", config_path).as_str())?,
    );

    //   let {config, extendedFiles}: ParcelConfigChain = await parseAndProcessConfig(
    //     configPath,
    //     contents,
    //     options,
    //   );

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

    // return {config, extendedFiles, usedDefault};

    Ok(parcel_config)
  }
}
