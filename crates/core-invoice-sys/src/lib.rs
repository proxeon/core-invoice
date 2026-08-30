//! C ABI. Python binds here via ctypes. WASM is the model crate, not this cdylib.

use core_invoice::Profile;
use core_invoice_formats::{Syntax, convert_with_profile, diff, validate_xml};
use std::os::raw::{c_char, c_int};

/// 0 valid, 1 invalid, 2 unreadable / bad args. Same contract as the CLI.
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
    validate_any(xml, profile, err, err_len)
}

/// Same as [`core_invoice_validate_ubl`]: syntax is the document element (UBL or CII).
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn core_invoice_validate(
    xml: *const c_char,
    profile: *const c_char,
    err: *mut c_char,
    err_len: usize,
) -> c_int {
    validate_any(xml, profile, err, err_len)
}

fn validate_any(
    xml: *const c_char,
    profile: *const c_char,
    err: *mut c_char,
    err_len: usize,
) -> c_int {
    if xml.is_null() {
        return write_err(err, err_len, "xml is null", 2);
    }
    let xml = unsafe { std::ffi::CStr::from_ptr(xml) }.to_string_lossy();
    let forced = match parse_profile(profile, err, err_len) {
        Ok(p) => p,
        Err(code) => return code,
    };
    match validate_xml(xml.as_ref(), forced) {
        Ok(report) if report.ok() => 0,
        Ok(report) => write_err(err, err_len, &report.to_string(), 1),
        Err(e) => write_err(err, err_len, &e.to_string(), 2),
    }
}

fn parse_profile(
    profile: *const c_char,
    err: *mut c_char,
    err_len: usize,
) -> Result<Option<Profile>, c_int> {
    if profile.is_null() {
        return Ok(None);
    }
    let s = unsafe { std::ffi::CStr::from_ptr(profile) }.to_string_lossy();
    if s.is_empty() {
        return Ok(None);
    }
    match Profile::parse(s.as_ref()) {
        Some(p) => Ok(Some(p)),
        None => Err(write_err(
            err,
            err_len,
            &format!("unknown profile {s}; known: {}", Profile::known_slugs()),
            2,
        )),
    }
}

/// Convert through the semantic model. `to` is `ubl` or `cii`.
/// On success writes XML into `out` (NUL-terminated). Same 0/1/2 as CLI convert.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn core_invoice_convert(
    xml: *const c_char,
    to: *const c_char,
    profile: *const c_char,
    out: *mut c_char,
    out_len: usize,
    err: *mut c_char,
    err_len: usize,
) -> c_int {
    if xml.is_null() || to.is_null() {
        return write_err(err, err_len, "xml or to is null", 2);
    }
    let xml = unsafe { std::ffi::CStr::from_ptr(xml) }.to_string_lossy();
    let to = unsafe { std::ffi::CStr::from_ptr(to) }.to_string_lossy();
    let Some(syntax) = Syntax::parse(to.as_ref()) else {
        return write_err(err, err_len, "to must be ubl or cii", 2);
    };
    let forced = match parse_profile(profile, err, err_len) {
        Ok(p) => p,
        Err(code) => return code,
    };
    match convert_with_profile(xml.as_ref(), syntax, forced) {
        Ok(s) => write_out(out, out_len, &s),
        Err(core_invoice_formats::FormatError::Semantic(rej)) => {
            write_err(err, err_len, &rej.0.to_string(), 1)
        }
        Err(e) => write_err(err, err_len, &e.to_string(), 2),
    }
}

/// Semantic diff. 0 identical, 1 differ, 2 unreadable. Writes the report into `out`.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn core_invoice_diff(
    left: *const c_char,
    right: *const c_char,
    out: *mut c_char,
    out_len: usize,
    err: *mut c_char,
    err_len: usize,
) -> c_int {
    if left.is_null() || right.is_null() {
        return write_err(err, err_len, "left or right is null", 2);
    }
    let left = unsafe { std::ffi::CStr::from_ptr(left) }.to_string_lossy();
    let right = unsafe { std::ffi::CStr::from_ptr(right) }.to_string_lossy();
    match diff(left.as_ref(), right.as_ref()) {
        Ok(s) if s == "no semantic difference" => write_out(out, out_len, &s),
        Ok(s) => {
            let _ = write_out(out, out_len, &s);
            1
        }
        Err(e) => write_err(err, err_len, &e.to_string(), 2),
    }
}

/// Writes crate version into `out`. Always 0 unless `out` is null (2).
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn core_invoice_version(out: *mut c_char, out_len: usize) -> c_int {
    write_out(out, out_len, env!("CARGO_PKG_VERSION"))
}

fn write_out(out: *mut c_char, out_len: usize, msg: &str) -> c_int {
    if out.is_null() || out_len == 0 {
        return 2;
    }
    let max = out_len - 1;
    let mut end = msg.len().min(max);
    let bytes = msg.as_bytes();
    while end > 0 && end < bytes.len() && (bytes[end] & 0b1100_0000) == 0b1000_0000 {
        end -= 1;
    }
    end = end.min(bytes.len()).min(max);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), out, end);
        *out.add(end) = 0;
    }
    0
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

    #[test]
    fn validate_peppol_ok_is_0() {
        let xml = core_invoice_formats::write_unchecked(
            &core_invoice_fixtures::peppol_vat(),
            core_invoice_formats::Syntax::Ubl,
        )
        .unwrap();
        let xml = CString::new(xml).unwrap();
        let profile = CString::new("peppol").unwrap();
        let mut buf = vec![0 as c_char; 256];
        let code =
            core_invoice_validate(xml.as_ptr(), profile.as_ptr(), buf.as_mut_ptr(), buf.len());
        assert_eq!(code, 0);
    }

    #[test]
    fn validate_empty_number_is_1() {
        let mut inv = core_invoice_fixtures::peppol_vat();
        inv.number.clear();
        let xml =
            core_invoice_formats::write_unchecked(&inv, core_invoice_formats::Syntax::Ubl).unwrap();
        let xml = CString::new(xml).unwrap();
        let profile = CString::new("peppol").unwrap();
        let mut buf = vec![0 as c_char; 512];
        let code =
            core_invoice_validate(xml.as_ptr(), profile.as_ptr(), buf.as_mut_ptr(), buf.len());
        assert_eq!(code, 1);
    }

    #[test]
    fn version_is_nonzero() {
        let mut buf = vec![0 as c_char; 32];
        let code = core_invoice_version(buf.as_mut_ptr(), buf.len());
        assert_eq!(code, 0);
        let msg = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }.to_string_lossy();
        assert!(!msg.is_empty());
    }
}
