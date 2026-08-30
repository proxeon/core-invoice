//! Shared XML hygiene for UBL and CII. One DTD policy, one depth cap.

use crate::FormatError;

pub const MAX_DEPTH: usize = 64;

/// Hostile or mistaken multi-GB “invoice” is size, not a valid document.
pub const MAX_INPUT_BYTES: usize = 10 * 1024 * 1024;

pub fn refuse_dtd(xml: &str) -> Result<(), FormatError> {
    if xml.to_ascii_lowercase().contains("<!doctype") {
        return Err(FormatError::Parse("DTD is refused".into()));
    }
    Ok(())
}

pub fn refuse_oversize(xml: &str) -> Result<(), FormatError> {
    if xml.len() > MAX_INPUT_BYTES {
        return Err(FormatError::Parse(format!(
            "input exceeds {MAX_INPUT_BYTES} bytes"
        )));
    }
    Ok(())
}

pub fn refuse_depth(xml: &str) -> Result<(), FormatError> {
    if exceeds_depth(xml, MAX_DEPTH) {
        return Err(FormatError::Parse(format!("XML depth exceeds {MAX_DEPTH}")));
    }
    Ok(())
}

/// Syntax from the document element after skipping comments/PIs, not from substring search.
pub fn document_element_local(xml: &str) -> Option<&str> {
    first_element_local(skip_prolog(xml))
}

pub fn skip_prolog(xml: &str) -> &str {
    let mut rest = xml.trim_start();
    loop {
        rest = rest.trim_start();
        if rest.starts_with("<?") {
            rest = rest.split_once("?>").map(|(_, r)| r).unwrap_or("");
            continue;
        }
        if rest.starts_with("<!--") {
            rest = rest.split_once("-->").map(|(_, r)| r).unwrap_or("");
            continue;
        }
        if rest.to_ascii_lowercase().starts_with("<!doctype")
            && let Some(i) = rest.find('>')
        {
            rest = &rest[i + 1..];
            continue;
        }
        break;
    }
    rest.trim_start()
}

pub fn first_element_local(xml: &str) -> Option<&str> {
    let rest = xml.trim_start().strip_prefix('<')?;
    let rest = rest.trim_start_matches('/');
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .unwrap_or(rest.len());
    let qname = &rest[..end];
    Some(qname.rsplit_once(':').map(|(_, l)| l).unwrap_or(qname))
}

pub fn exceeds_depth(xml: &str, max: usize) -> bool {
    let mut depth = 0usize;
    let bytes = xml.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                depth = depth.saturating_sub(1);
            } else if i + 1 < bytes.len() && (bytes[i + 1] == b'!' || bytes[i + 1] == b'?') {
            } else {
                depth += 1;
                if depth > max {
                    return true;
                }
            }
        } else if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'>' {
            depth = depth.saturating_sub(1);
        }
        i += 1;
    }
    false
}

/// `xs:boolean`: true/false/1/0, with XML whitespace.
pub fn parse_xs_boolean(s: &str) -> Option<bool> {
    match s.trim() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}
