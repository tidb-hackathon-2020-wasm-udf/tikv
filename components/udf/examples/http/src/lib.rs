use std::slice;
use std::str;

// Define a function that is imported into the module.
// By default, the "env" namespace is used.
extern "C" {
    fn http_get(ptr: *const u8, len: u32) -> u32;
    fn print_str(ptr: *const u8, len: u32);
}

#[no_mangle]
pub extern "C" fn hello_http_get(ptr: u32, len: u32) {
    let slice = unsafe { slice::from_raw_parts(ptr as _, len as _) };
    let ss = str::from_utf8(&slice).unwrap();
    unsafe {
        let len = http_get(ss.as_ptr(), ss.len() as u32);
        print_str(ptr as _, len)
    }
}
