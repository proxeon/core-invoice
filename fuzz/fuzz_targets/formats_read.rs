#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(xml) = std::str::from_utf8(data) else {
        return;
    };
    let _ = core_invoice_formats::read(xml);
    let _ = core_invoice_formats::read_with_trace(xml);
});
