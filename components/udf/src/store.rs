use crate::wasm::WASM;
use crate::Result;
use serde_json::{Map, Value};
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::{collections::HashMap, fs};

const STORE_PATH: &'static str = ".wasm_store";

fn wasm_path(id: u64) -> PathBuf {
    Path::new(STORE_PATH).join(format!("{}.wasm", id))
}
/// Store all the wasm modules
#[derive(Default, Debug)]
pub struct Store {
    cache: HashMap<u64, WASM>,
}

impl Store {
    /// Init a wasm store with default location on local filesystem
    pub fn init() -> Result<Self> {
        let _ = fs::create_dir(STORE_PATH);
        Ok(Store::default())
    }

    /// Store the wasm payload
    // pub fn insert(&mut self, name: &str, payload: Vec<u8>) -> Result<()> {
    //     let wasm_file = Path::new(STORE_PATH).join(name);
    //     let mut f = OpenOptions::new().read(true).write(true).open(&wasm_file)?;
    //     f.write_all(&payload)?;
    //     let _ = self.manifest.insert(name.to_owned(), Value::Null);
    //     self.flush()
    // }

    /// Get wasm content by name
    pub fn get(&mut self, id: u64) -> Result<Option<WASM>> {
        if !self.cache.contains_key(&id) {
            let contents = fs::read(wasm_path(id))?;
            let wasm = WASM::new("udf_main".to_owned(), contents);
            self.cache.insert(id, wasm.clone());
            Ok(Some(wasm))
        } else {
            Ok(self.cache.get(&id).cloned())
        }
    }

    // /// Get all the wasm module names
    // pub fn list(&self) -> Vec<String> {
    //     self.manifest.keys().cloned().collect::<Vec<_>>()
    // }

    // fn flush(&mut self) -> Result<()> {
    //     serde_json::to_writer(&self.file, &self.manifest)?;
    //     Ok(())
    // }
}
