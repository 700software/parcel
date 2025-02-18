//! Provides helpers to cast from a `JsObject` options object into a few common
//! options
//!
//! This corresponds to the `RequestOptions` javascript type.
//!
//! The options read are `options.inputFS` and `options.projectRoot`.
//!
//! This is either a no-copy (for inputFS) or a copy on read operation
//! (for projectRoot).
use std::rc::Rc;

use napi::Env;
use napi::JsObject;
use napi::JsString;
use parcel_filesystem::js_delegate_file_system::JSDelegateFileSystem;

pub fn project_root_from_options(options: &JsObject) -> napi::Result<String> {
  let Some(project_root): Option<JsString> = options.get("projectRoot")? else {
    return Err(napi::Error::from_reason(
      "[napi] Missing required projectRoot options field",
    ));
  };
  let project_root = project_root.into_utf8()?;
  let project_root = project_root.as_str()?;
  Ok(project_root.to_string())
}

pub fn input_fs_from_options(
  env: Rc<Env>,
  options: &JsObject,
) -> napi::Result<JSDelegateFileSystem> {
  let Some(input_fs) = options
    .get("inputFS")?
    .map(|input_fs| JSDelegateFileSystem::new(env, input_fs))
  else {
    // We need to make the `FileSystem` trait object-safe, so we can use dynamic
    // dispatch.
    return Err(napi::Error::from_reason(
      "[napi] Missing required inputFS options field",
    ));
  };
  Ok(input_fs)
}
