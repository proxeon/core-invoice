//! C ABI. Python and WASM are not implemented yet; they will bind here.

use core_invoice::Profile;
use core_invoice_formats::validate_xml;
use std::os::raw::{c_char, c_int};

/// 0 valid, 1 invalid, 2 unreadable / bad args.
///
/// `profile` NULL means auto from BT-24 (CustomizationID).
/// Known slugs: en16931, peppol, pint, pint-my.
/// `xml` must outlive the call. `err` is `err_len` writable bytes, UTF-8, always NUL-terminated if err_len > 0.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
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
    let xml = unsafe { std::ffi::CStr::from_ptr(xml) }.to_string_lossy();
    let forced = if profile.is_null() {
        None
    } else {
        let s = unsafe { std::ffi::CStr::from_ptr(profile) }.to_string_lossy();
        if s.is_empty() {
            None
        } else {
            match Profile::parse(s.as_ref()) {
                Some(p) => Some(p),
                None => {
                    return write_err(
                        err,
                        err_len,
                        &format!("unknown profile {s}; known: {}", Profile::known_slugs()),
                        2,
                    );
                }
            }
        }
    };
    match validate_xml(xml.as_ref(), forced) {
        Ok(report) if report.ok() => 0,
        Ok(report) => write_err(err, err_len, &report.to_string(), 1),
        Err(e) => write_err(err, err_len, &e.to_string(), 2),
    }
}

fn write_err(err: *mut c_char, err_len: usize, msg: &str, code: c_int) -> c_int {
    if err.is_null() || err_len == 0 {
        return code;
    }
    let max = err_len - 1;
    let mut end = msg.len().min(max);
    let bytes = msg.as_bytes();
    while end > 0 && end < bytes.len() && (bytes[end] & 0b1100_0000) == 0b1000_0000 {
        end -= 1;
    }
    end = end.min(bytes.len()).min(max);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), err, end);
        *err.add(end) = 0;
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn unknown_profile_is_unreadable() {
        let xml = CString::new("<Invoice/>").unwrap();
        let profile = CString::new("xrechnung").unwrap();
        let mut buf = vec![0 as c_char; 128];
        let code =
            core_invoice_validate_ubl(xml.as_ptr(), profile.as_ptr(), buf.as_mut_ptr(), buf.len());
        assert_eq!(code, 2);
        let msg = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }.to_string_lossy();
        assert!(msg.contains("unknown profile"), "{msg}");
        assert!(msg.contains("en16931"), "{msg}");
    }
}
