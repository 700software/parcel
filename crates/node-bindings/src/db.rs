use std::sync::Arc;

use lazy_static::lazy_static;
use napi::{bindgen_prelude::BigInt, Env, NapiValue};
use napi_derive::napi;
use parcel_db::{InternedString, ParcelDb, ParcelDbWrapper};

#[napi(js_name = "ParcelDb")]
pub struct JsParcelDb {
  db: Arc<ParcelDbWrapper>,
}

// impl Drop for JsParcelDb {
//   fn drop(&mut self) {
//     let x = &*self.db;
//     println!("Drop JS {:p} {}", x, Arc::strong_count(&self.db));
//   }
// }

#[napi(object)]
pub struct SerializedParcelDb {
  pub id: BigInt,
}

#[napi]
impl JsParcelDb {
  #[napi(constructor)]
  pub fn new() -> Self {
    // println!("NEW DB");
    JsParcelDb {
      db: Arc::new(ParcelDb::new()),
    }
  }

  #[napi]
  pub fn serialize(&self) -> SerializedParcelDb {
    // println!("serialize");
    SerializedParcelDb {
      id: BigInt::from(Arc::as_ptr(&self.db) as u64),
    }
  }

  #[napi(factory)]
  pub fn deserialize_native(value: SerializedParcelDb) -> Self {
    let ptr = value.id.words[0] as *const ParcelDbWrapper;
    let db = unsafe {
      Arc::increment_strong_count(ptr);
      Arc::from_raw(ptr)
    };
    // println!(
    //   "deserialize {:?} {:?}",
    //   value.id.words[0],
    //   Arc::strong_count(&db)
    // );
    Self { db }
  }

  pub fn with<T, F: FnOnce(&ParcelDb) -> T>(&self, f: F) -> T {
    self.db.with(f)
  }

  pub fn db(&self) -> Arc<ParcelDbWrapper> {
    self.db.clone()
  }

  #[napi]
  pub fn get_page(&self, env: Env, page: u32) -> napi::Result<napi::JsBuffer> {
    self.db.with(|db| {
      let slice = db.heap_page(page);
      unsafe {
        Ok(
          env
            .create_buffer_with_borrowed_data(
              slice.as_mut_ptr(),
              slice.len(),
              0,
              napi::noop_finalize,
            )?
            .into_raw(),
        )
      }
    })
  }

  #[napi]
  pub fn alloc(&self, type_id: u32) -> u32 {
    self.db.with(|db| db.alloc(type_id))
  }

  #[napi]
  pub fn dealloc(&self, type_id: u32, addr: u32) {
    self.db.with(|db| db.dealloc(type_id, addr))
  }

  #[napi]
  pub fn read_string(&self, addr: u32, env: Env) -> napi::Result<napi::JsString> {
    self.db.with(|db| {
      let string = db.read_string(addr);
      if let Some(node_api_create_external_string_latin1) = *CREATE_EXTERNAL_STRING {
        let mut value = std::ptr::null_mut();
        let status = unsafe {
          node_api_create_external_string_latin1(
            env.raw(),
            string.as_ptr() as *const std::os::raw::c_char,
            string.len(),
            Some(finalize),
            std::ptr::null_mut(),
            &mut value,
            std::ptr::null_mut(),
          )
        };
        if status != napi::sys::Status::napi_ok {
          return Err(napi::Error::new(
            napi::Status::GenericFailure,
            "Failed to create string",
          ));
        }
        Ok(unsafe { napi::JsString::from_raw_unchecked(env.raw(), value) })
      } else {
        env.create_string(string)
      }
    })
  }

  #[napi]
  pub fn get_string_id(&self, s: String) -> u32 {
    self.db.with(|_| InternedString::from(s).0.into())
  }

  #[napi]
  pub fn extend_vec(&self, addr: u32, size: u32, count: u32) {
    self.db.with(|db| db.extend_vec(addr, size, count))
  }

  #[napi]
  pub fn create_environment(&self, addr: u32) -> u32 {
    self
      .db
      .with(|db| db.environment_id(db.get_environment(addr)).0)
  }
}

type CreateExternalString = unsafe extern "C" fn(
  env: napi::sys::napi_env,
  str: *const std::os::raw::c_char,
  length: usize,
  finalize_cb: napi::sys::napi_finalize,
  finalize_hint: *mut std::os::raw::c_void,
  result: *mut napi::sys::napi_value,
  copied: *mut bool,
) -> napi::sys::napi_status;

lazy_static! {
  static ref CREATE_EXTERNAL_STRING: Option<CreateExternalString> = {
    unsafe {
      let ptr = libc::dlsym(
        libc::RTLD_DEFAULT,
        "node_api_create_external_string_latin1\0".as_ptr() as *const _,
      );
      if !ptr.is_null() {
        Some(std::mem::transmute_copy::<
          *mut libc::c_void,
          CreateExternalString,
        >(&ptr))
      } else {
        None
      }
    }
  };
}

unsafe extern "C" fn finalize(
  env: napi::sys::napi_env,
  _finalize_data: *mut std::os::raw::c_void,
  _finalize_hint: *mut std::os::raw::c_void,
) {
}
