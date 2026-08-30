//! C ABI. Python and WASM bind here.

use core_invoice::Profile;
use core_invoice_formats::validate_xml;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

/// 0 valid, 1 invalid, 2 unreadable.
#[unsafe(no_mangle)]
pub extern "C" fn core_invoice_validate_ubl(
    xml: *const c_char,
    profile: *const c_char,
    err: *mut c_char,
    err_len: usize,
) -> c_int {
    if xml.is_null() {
        return write_err(err, err_len, "xml is null", 2);
    }
    let xml = unsafe { CStr::from_ptr(xml) }.to_string_lossy();
    let profile = if profile.is_null() {
        Profile::Pint
    } else {
        let s = unsafe { CStr::from_ptr(profile) }.to_string_lossy();
        Profile::parse(s.as_ref()).unwrap_or(Profile::Pint)
    };
    match validate_xml(xml.as_ref(), Some(profile)) {
        Ok(report) if report.ok() => 0,
        Ok(report) => write_err(err, err_len, &report.to_string(), 1),
        Err(e) => write_err(err, err_len, &e.to_string(), 2),
    }
}

fn write_err(err: *mut c_char, err_len: usize, msg: &str, code: c_int) -> c_int {
    if !err.is_null() && err_len > 0 {
        let c = CString::new(msg.chars().take(err_len.saturating_sub(1)).collect::<String>())
            .unwrap_or_else(|_| CString::new("error").unwrap());
        let bytes = c.as_bytes_with_nul();
        let n = bytes.len().min(err_len);
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), err, n);
            *err.add(n.saturating_sub(1)) = 0;
        }
    }
    code
}
