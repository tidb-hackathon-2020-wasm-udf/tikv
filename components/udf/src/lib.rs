pub mod store;
pub mod wasm;
pub use store::Store;
pub type Result<T> = anyhow::Result<T>;
pub use anyhow::Error as WasmError;
pub use wasmer::Val;
