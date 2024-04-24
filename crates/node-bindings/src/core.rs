use napi::{JsObject, Result};
use napi_derive::napi;

use crate::parcel_config::{PipelineMap, PipelinesMap, PluginNode};

#[napi]
pub fn create_parcel_config(config: JsObject) -> Result<bool> {
  let bundler: Result<Option<PluginNode>> = config.get("bundler");
  println!("got bundler {:?}", bundler);

  let compressors: Result<Option<PipelinesMap>> = config.get("compressors");
  println!("got compressors {:?}", compressors);

  let namers: Result<Option<Vec<PluginNode>>> = config.get("namers");
  println!("got namers {:?}", namers);

  let optimizers: Result<Option<Vec<PluginNode>>> = config.get("optimizers");
  println!("got opt {:?}", optimizers);

  // let packagers: Result<Option<PipelineMap>> = config.get("packagers");
  // println!("got packagers {:?}", packagers);

  let reporters: Result<Option<Vec<PluginNode>>> = config.get("reporters");
  println!("got reporters {:?}", reporters);

  let resolvers: Result<Option<Vec<PluginNode>>> = config.get("resolvers");
  println!("got resolvers {:?}", resolvers);

  let runtimes: Result<Option<Vec<PluginNode>>> = config.get("runtimes");
  println!("got runtimes {:?}", runtimes);

  // let transformers: Result<Option<PipelinesMap>> = config.get("transformers");
  // println!("got transformers {:?}", transformers);

  // let validators: Result<Option<PipelinesMap>> = config.get("validators");
  // println!("got validators {:?}", validators);

  Result::Ok(true)
}
