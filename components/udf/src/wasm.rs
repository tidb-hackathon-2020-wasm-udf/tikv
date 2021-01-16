use crate::Result;
use anyhow::{anyhow, bail};
use std::collections::HashMap;
use std::str;
use wasmer::{imports, ExportError, Function, Instance, Module, Store, Val, ValType};
use wasmer_runtime::{func, imports as runtime_imports, instantiate, Ctx, Value};
use wasmer_wasi::{get_wasi_version, WasiError, WasiState};

#[derive(Clone, Debug)]
pub struct WASM {
    name: String,
    contents: Vec<u8>,
}

impl WASM {
    pub fn new(name: String, contents: Vec<u8>) -> Self {
        Self { name, contents }
    }

    pub fn execute(&self, endpoint: &str, args: Vec<String>) -> Result<Box<[Val]>> {
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
        self.invoke_function(&instance, endpoint, &args)
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
        let result = func.call(&invoke_args);
        match result {
            Ok(v) => Ok(v),
            Err(err) => {
                let err = match err.downcast::<WasiError>() {
                    Ok(WasiError::Exit(exit_code)) => {
                        std::process::exit(exit_code as _);
                    }
                    Ok(err) => err.into(),
                    Err(err) => err.into(),
                };
                Err(err)
            }
        }
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

fn http_get(ctx: &mut Ctx, ptr: u32, len: u32) -> u32 {
    let memory = ctx.memory(0);
    let raw_url: Vec<_> = memory.view()[ptr as usize..(ptr + len) as usize]
        .iter()
        .map(|cell| cell.get())
        .collect();

    let url = str::from_utf8(&raw_url).unwrap();
    println!("GET {}", url);
    let res = reqwest::blocking::get(url).map_or_else(
        |e| {
            let mut m = HashMap::new();
            m.insert("error".to_owned(), e.to_string());
            serde_json::to_string(&m).unwrap()
        },
        |resp| resp.text().unwrap(),
    );

    let res_len = res.len();
    for (byte, cell) in res.bytes().zip(memory.view()[0..res_len].iter()) {
        cell.set(byte);
    }
    res.len() as u32
}

fn print_str(ctx: &mut Ctx, ptr: u32, len: u32) {
    // Get a slice that maps to the memory currently used by the webassembly
    // instance.
    //
    // Webassembly only supports a single memory for now,
    // but in the near future, it'll support multiple.
    //
    // Therefore, we don't assume you always just want to access first
    // memory and force you to specify the first memory.
    let memory = ctx.memory(0);

    // Get a subslice that corresponds to the memory used by the string.
    let str_vec: Vec<_> = memory.view()[ptr as usize..(ptr + len) as usize]
        .iter()
        .map(|cell| cell.get())
        .collect();

    // Convert the subslice to a `&str`.
    let string = str::from_utf8(&str_vec).unwrap();

    // Print it!
    println!("{}", string);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_wasm() {
        let nbody = std::fs::read("nbody.wasm").unwrap();
        let wasm = WASM::new("nbody".to_owned(), nbody);
        wasm.execute("_start", vec!["5000000".to_owned()]).unwrap();
    }

    #[test]
    fn test_http_get() {
        let wasm = std::fs::read("http.wasm").unwrap();
        let import_object = runtime_imports! {
            // Define the "env" namespace that was implicitly used
            // by our sample application.
            "env" => {
                // name        // the func! macro autodetects the signature
                "http_get" => func!(http_get),
                "print_str" => func!(print_str),
            },
        };
        // Compile our webassembly into an `Instance`.
        let mut instance = instantiate(&wasm, &import_object).unwrap();
        let memory = instance.context_mut().memory(0);

        let host_string = "https://api.github.com/";

        // Write the string into the lineary memory
        for (byte, cell) in host_string
            .bytes()
            .zip(memory.view()[0 as usize..(host_string.len()) as usize].iter())
        {
            cell.set(byte);
        }

        // Call our exported function!
        instance
            .call(
                "hello_http_get",
                &[Value::I32(0), Value::I32(host_string.len() as _)],
            )
            .unwrap();
    }
}
