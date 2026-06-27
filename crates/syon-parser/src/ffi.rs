use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::parser::parse;

/// Parse SYON from a null-terminated C string and return a JSON C string.
///
/// On success returns a pointer to a JSON UTF-8 string that must be freed with
/// [`syon_free`]. On error returns a JSON object `{"error": "<message>"}`.
/// Never returns null.
#[no_mangle]
pub extern "C" fn syon_parse_json(input: *const c_char) -> *mut c_char {
    let result = (|| -> Result<String, String> {
        if input.is_null() {
            return Err("null input pointer".into());
        }
        let c_str = unsafe { CStr::from_ptr(input) };
        let s = c_str.to_str().map_err(|e| e.to_string())?;
        let file = parse(s).map_err(|e| e.to_string())?;
        serde_json::to_string(&file).map_err(|e| e.to_string())
    })();

    let json = match result {
        Ok(s) => s,
        Err(e) => {
            let escaped = e.replace('"', "\\\"");
            format!("{{\"error\":\"{escaped}\"}}")
        }
    };

    CString::new(json)
        .unwrap_or_else(|_| CString::new("{\"error\":\"internal encoding error\"}").unwrap())
        .into_raw()
}

/// Free a string returned by [`syon_parse_json`].
///
/// # Safety
/// `ptr` must have been returned by `syon_parse_json` and must not be freed twice.
#[no_mangle]
pub unsafe extern "C" fn syon_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}
