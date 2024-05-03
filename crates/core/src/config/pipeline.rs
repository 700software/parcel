use glob_match::glob_match;
use indexmap::IndexMap;
use std::path::Path;

use super::parcel_config::PluginNode;

#[derive(Debug, Default, PartialEq)]
pub struct PipelineMap {
  map: IndexMap<String, Vec<PluginNode>>,
}

impl PipelineMap {
  pub fn new(map: IndexMap<String, Vec<PluginNode>>) -> Self {
    Self { map }
  }

  pub fn get(&self, path: &Path, pipeline: &Option<impl AsRef<str>>) -> Vec<PluginNode> {
    let basename = path.file_name().unwrap().to_str().unwrap();
    let path = path.as_os_str().to_str().unwrap();

    let mut matches: Vec<PluginNode> = Vec::new();
    if let Some(pipeline) = pipeline {
      let exact_match = self
        .map
        .iter()
        .find(|(pattern, _)| is_match(pattern, path, basename, pipeline.as_ref()));

      if let Some((_, pipelines)) = exact_match {
        matches.extend(pipelines.iter().cloned());
      } else {
        return Vec::new();
      }
    }

    for (pattern, pipelines) in self.map.iter() {
      if is_match(&pattern, path, basename, "") {
        matches.extend(pipelines.iter().cloned());
      }
    }

    matches
  }

  pub fn contains_named_pipeline(&self, pipeline: impl AsRef<str>) -> bool {
    let named_pipeline = format!("{}:", pipeline.as_ref());

    self
      .map
      .keys()
      .any(|glob| glob.starts_with(&named_pipeline))
  }

  pub fn named_pipelines(&self) -> Vec<&str> {
    self
      .map
      .keys()
      .filter_map(|glob| glob.split_once(':').map(|g| g.0))
      .collect()
  }
}

pub(crate) fn is_match(pattern: &str, path: &str, basename: &str, pipeline: &str) -> bool {
  let (pattern_pipeline, glob) = pattern.split_once(':').unwrap_or(("", pattern));
  pipeline == pattern_pipeline && (glob_match(glob, basename) || glob_match(glob, path))
}

#[cfg(test)]
mod pipeline_map_tests {
  use super::*;
  use std::{path::PathBuf, rc::Rc};

  fn pipelines() -> Vec<PluginNode> {
    vec![PluginNode {
      package_name: String::from("@parcel/plugin-1"),
      resolve_from: Rc::new(PathBuf::default()),
    }]
  }

  fn pipelines_two() -> Vec<PluginNode> {
    vec![PluginNode {
      package_name: String::from("@parcel/plugin-2"),
      resolve_from: Rc::new(PathBuf::default()),
    }]
  }

  fn pipelines_three() -> Vec<PluginNode> {
    vec![PluginNode {
      package_name: String::from("@parcel/plugin-3"),
      resolve_from: Rc::new(PathBuf::default()),
    }]
  }

  mod get {
    use std::env;

    use super::*;
    use indexmap::indexmap;

    fn paths(filename: &str) -> Vec<PathBuf> {
      let cwd = env::current_dir().unwrap();
      vec![
        PathBuf::from(filename),
        cwd.join(filename),
        cwd.join("src").join(filename),
      ]
    }

    #[test]
    fn returns_empty_vec_for_empty_map() {
      let empty_map = PipelineMap::default();
      let empty_pipeline: Option<&str> = None;
      let empty_vec: Vec<PluginNode> = Vec::new();

      assert_eq!(
        empty_map.get(&PathBuf::from("a.js"), &empty_pipeline),
        empty_vec
      );

      assert_eq!(
        empty_map.get(&PathBuf::from("a.toml"), &empty_pipeline),
        empty_vec
      );
    }

    #[test]
    fn returns_empty_vec_when_no_matching_path() {
      let empty_pipeline: Option<&str> = None;
      let empty_vec: Vec<PluginNode> = Vec::new();
      let map = PipelineMap::new(indexmap! {
        String::from("*.{js,ts}") => pipelines(),
        String::from("*.toml") => pipelines()
      });

      assert_eq!(map.get(&PathBuf::from("a.css"), &empty_pipeline), empty_vec);
      assert_eq!(map.get(&PathBuf::from("a.jsx"), &empty_pipeline), empty_vec);
      assert_eq!(map.get(&PathBuf::from("a.tsx"), &empty_pipeline), empty_vec);
      assert_eq!(map.get(&PathBuf::from("a.tom"), &empty_pipeline), empty_vec);
      assert_eq!(
        map.get(&PathBuf::from("a.tomla"), &empty_pipeline),
        empty_vec
      );
    }

    #[test]
    fn returns_empty_vec_when_no_matching_pipeline() {
      let empty_vec: Vec<PluginNode> = Vec::new();
      let map = PipelineMap::new(indexmap! {
        String::from("*.{js,ts}") => pipelines(),
        String::from("*.toml") => pipelines(),
        String::from("types:*.{ts,tsx}") => pipelines(),
        String::from("url:*") => pipelines_two()
      });

      assert_eq!(map.get(&PathBuf::from("a.css"), &Some("css")), empty_vec);
      assert_eq!(map.get(&PathBuf::from("a.jsx"), &Some("jsx")), empty_vec);
      assert_eq!(map.get(&PathBuf::from("a.tsx"), &Some("tsx")), empty_vec);
      assert_eq!(map.get(&PathBuf::from("a.ts"), &Some("typesa")), empty_vec);
      assert_eq!(
        map.get(&PathBuf::from("a.js"), &Some("data-url")),
        empty_vec
      );
    }

    #[test]
    fn returns_matching_plugins_for_empty_pipeline() {
      let empty_pipeline: Option<&str> = None;
      let map = PipelineMap::new(indexmap! {
        String::from("*.{js,ts}") => pipelines(),
        String::from("*.toml") => pipelines_two()
      });

      for path in paths("a.js") {
        assert_eq!(map.get(&path, &empty_pipeline), pipelines());
      }

      for path in paths("a.ts") {
        assert_eq!(map.get(&path, &empty_pipeline), pipelines());
      }

      for path in paths("a.toml") {
        assert_eq!(map.get(&path, &empty_pipeline), pipelines_two());
      }
    }

    #[test]
    fn returns_matching_plugins_for_pipeline() {
      let map = PipelineMap::new(indexmap! {
        String::from("*.{js,ts}") => pipelines_three(),
        String::from("*.toml") => pipelines_three(),
        String::from("types:*.{ts,tsx}") => pipelines(),
        String::from("url:*") => pipelines_two()
      });

      let expected_ts: Vec<PluginNode> = [pipelines(), pipelines_three()].concat();
      for path in paths("a.ts") {
        assert_eq!(map.get(&path, &Some("types")), expected_ts);
      }

      for path in paths("a.tsx") {
        assert_eq!(map.get(&path, &Some("types")), pipelines());
      }

      for path in paths("a.url") {
        assert_eq!(map.get(&path, &Some("url")), pipelines_two());
      }
    }
  }

  mod contains_named_pipeline {
    use super::*;
    use indexmap::indexmap;

    #[test]
    fn returns_true_when_named_pipeline_exists() {
      let map = PipelineMap::new(indexmap! {
        String::from("data-url:*") => pipelines()
      });

      assert_eq!(map.contains_named_pipeline("data-url"), true);
    }

    #[test]
    fn returns_false_for_empty_map() {
      let empty_map = PipelineMap::default();

      assert_eq!(empty_map.contains_named_pipeline("data-url"), false);
      assert_eq!(empty_map.contains_named_pipeline("types"), false);
    }

    #[test]
    fn returns_false_when_named_pipeline_does_not_exist() {
      let map = PipelineMap::new(indexmap! {
        String::from("*.{js,ts}") => pipelines(),
        String::from("*.toml") => pipelines(),
        String::from("url:*") => pipelines()
      });

      assert_eq!(map.contains_named_pipeline("*"), false);
      assert_eq!(map.contains_named_pipeline("data-url"), false);
      assert_eq!(map.contains_named_pipeline("types"), false);
      assert_eq!(map.contains_named_pipeline("urls"), false);
    }
  }

  mod named_pipelines {
    use super::*;
    use indexmap::indexmap;

    #[test]
    fn returns_empty_vec_when_no_named_pipelines() {
      let empty_vec: Vec<&str> = Vec::new();

      assert_eq!(PipelineMap::default().named_pipelines(), empty_vec);
      assert_eq!(
        PipelineMap::new(indexmap! {
          String::from("*.{js,ts}") => pipelines(),
          String::from("*.toml") => pipelines(),
        })
        .named_pipelines(),
        empty_vec,
      );
    }

    #[test]
    fn returns_list_of_named_pipelines() {
      assert_eq!(
        PipelineMap::new(indexmap! {
          String::from("data-url:*") => pipelines()
        })
        .named_pipelines(),
        vec!("data-url")
      );

      assert_eq!(
        PipelineMap::new(indexmap! {
          String::from("types:*.{ts,tsx}") => pipelines()
        })
        .named_pipelines(),
        vec!("types")
      );

      assert_eq!(
        PipelineMap::new(indexmap! {
          String::from("url:*") => pipelines()
        })
        .named_pipelines(),
        vec!("url")
      );

      assert_eq!(
        PipelineMap::new(indexmap! {
          String::from("*.{js,ts}") => pipelines(),
          String::from("*.toml") => pipelines(),
          String::from("bundle-text:*") => pipelines(),
          String::from("data-url:*") => pipelines(),
          String::from("types:*.{ts,tsx}") => pipelines(),
          String::from("url:*") => pipelines()
        })
        .named_pipelines(),
        vec!("bundle-text", "data-url", "types", "url")
      );
    }
  }
}
