use crate::Result;
use anyhow::{anyhow, bail};
use wasmer::{imports, ExportError, Function, Instance, Module, Store, Val, ValType};
use wasmer_wasi::{get_wasi_version, WasiError, WasiState};

#[derive(Clone)]
pub struct WASM {
    name: String,
    contents: Vec<u8>,
}

impl WASM {
    pub fn new(name: String, contents: Vec<u8>) -> Self {
        Self { name, contents }
    }

    /// Call an exported wasm func
    pub fn call(&self, func: &str, args: &[String]) -> Result<Box<[Val]>> {
        let store = Store::default();
        let module = Module::new(&store, &self.contents)?;
        let imports = imports! {};
        let instance = Instance::new(&module, &imports)?;
        self.invoke_function(&instance, func, args)
    }

    /// Execute a _start
    pub fn execute(&self, args: Vec<String>) -> Result<()> {
        let store = Store::default();
        let module = Module::new(&store, &self.contents)?;
        let import_object = {
            if self.has_wasi_imports(&module) {
                let args = args.iter().cloned().map(|arg| arg.into_bytes());
                let mut wasi_state_builder = WasiState::new(&self.name);
                wasi_state_builder.args(args);
                let mut wasi_env = wasi_state_builder.finalize()?;
                wasi_env.import_object(&module)?
            } else {
                imports! {}
            }
        };
        let instance = Instance::new(&module, &import_object)?;
        let start = instance.exports.get_function("_start")?;
        let result = start.call(&[]);
        match result {
            Ok(_) => Ok(()),
            Err(err) => {
                let err = match err.downcast::<WasiError>() {
                    Ok(WasiError::Exit(exit_code)) => {
                        // We should exit with the provided exit code
                        std::process::exit(exit_code as _);
                    }
                    Ok(err) => err.into(),
                    Err(err) => err.into(),
                };
                Err(err)
            }
        }
    }

    #[inline]
    fn has_wasi_imports(&self, module: &Module) -> bool {
        // Get the wasi version in non-strict mode, so no other imports
        // are allowed
        get_wasi_version(&module, false).is_some()
    }

    fn invoke_function(
        &self,
        instance: &Instance,
        invoke: &str,
        args: &[String],
    ) -> Result<Box<[Val]>> {
        let func: Function = self.try_find_function(&instance, invoke)?;
        let func_ty = func.ty();
        let required_arguments = func_ty.params().len();
        let provided_arguments = args.len();
        if required_arguments != provided_arguments {
            bail!(
                "Function expected {} arguments, but received {}: \"{}\"",
                required_arguments,
                provided_arguments,
                args.join(" ")
            );
        }
        let invoke_args = args
            .iter()
            .zip(func_ty.params().iter())
            .map(|(arg, param_type)| match param_type {
                ValType::I32 => {
                    Ok(Val::I32(arg.parse().map_err(|_| {
                        anyhow!("Can't convert `{}` into a i32", arg)
                    })?))
                }
                ValType::I64 => {
                    Ok(Val::I64(arg.parse().map_err(|_| {
                        anyhow!("Can't convert `{}` into a i64", arg)
                    })?))
                }
                ValType::F32 => {
                    Ok(Val::F32(arg.parse().map_err(|_| {
                        anyhow!("Can't convert `{}` into a f32", arg)
                    })?))
                }
                ValType::F64 => {
                    Ok(Val::F64(arg.parse().map_err(|_| {
                        anyhow!("Can't convert `{}` into a f64", arg)
                    })?))
                }
                _ => Err(anyhow!(
                    "Don't know how to convert {} into {:?}",
                    arg,
                    param_type
                )),
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(func.call(&invoke_args)?)
    }

    fn try_find_function(&self, instance: &Instance, name: &str) -> Result<Function> {
        Ok(instance
            .exports
            .get_function(&name)
            .map_err(|e| {
                if instance.module().info().functions.is_empty() {
                    anyhow!("The module has no exported functions to call.")
                } else {
                    match e {
                        ExportError::Missing(_) => {
                            anyhow!("No export `{}` found in the module", name)
                        }
                        ExportError::IncompatibleType => {
                            anyhow!("Export `{}` found, but is not a function.", name)
                        }
                    }
                }
            })?
            .clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_wasm() {
        let nbody = std::fs::read("nbody.wasm").unwrap();
        let wasm = WASM::new("nbody".to_owned(), nbody);
        wasm.execute(vec!["5000000".to_owned()]).unwrap();
    }
}
