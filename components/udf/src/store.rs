use crate::Result;
use serde_json::{Map, Value};
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::path::Path;

const STORE_PATH: &'static str = ".wasm_store";
const STORE_MANIFEST: &'static str = "manifest";

/// Store all the wasm modules
pub struct Store {
    file: File,
    // wasm name => load path
    manifest: Map<String, Value>,
}

impl Store {
    /// Init a wasm store with default location on local filesystem
    pub fn init() -> Result<Self> {
        let _ = fs::create_dir(STORE_PATH);
        let path = Path::new(STORE_PATH).join(STORE_MANIFEST);
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .append(false)
            .open(&path)?;
        let content = fs::read(&path)?;
        let v = serde_json::from_slice::<Map<String, Value>>(&content)?;
        Ok(Store {
            file: f,
            manifest: v,
        })
    }

    /// Store the wasm payload
    pub fn insert(&mut self, name: &str, payload: Vec<u8>) -> Result<()> {
        let wasm_file = Path::new(STORE_PATH).join(name);
        let mut f = OpenOptions::new().read(true).write(true).open(&wasm_file)?;
        f.write_all(&payload)?;
        let _ = self.manifest.insert(name.to_owned(), Value::Null);
        self.flush()
    }

    /// Get wasm content by name
    pub fn get(&mut self, name: &str) -> Result<Option<Vec<u8>>> {
        if !self.manifest.contains_key(name) {
            return Ok(None);
        }
        let wasm_file = Path::new(STORE_PATH).join(name);
        let content = fs::read(wasm_file)?;
        Ok(Some(content))
    }

    /// Get all the wasm module names
    pub fn list(&self) -> Vec<String> {
        self.manifest.keys().cloned().collect::<Vec<_>>()
    }

    fn flush(&mut self) -> Result<()> {
        serde_json::to_writer(&self.file, &self.manifest)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_store() {
        let store = Store::init().unwrap();
        let empty: Vec<String> = vec![];
        assert_eq!(store.list(), empty);
    }
}
