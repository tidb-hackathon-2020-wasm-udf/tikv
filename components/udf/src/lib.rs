#![feature(min_specialization)]

pub mod store;
pub mod wasm;
pub use store::Store;
pub type Result<T> = anyhow::Result<T>;
