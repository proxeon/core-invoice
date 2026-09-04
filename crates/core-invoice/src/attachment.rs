//! Binary [`Attachment`]: bytes, MIME, and filename (BT-125).

use crate::error::AttachmentError;

/// Binary object: bytes + mime + filename, all mandatory and non-blank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    /// BT-125 attached document bytes.
    pub bytes: Vec<u8>,
    /// MIME code of BT-125. Mandatory, non-blank.
    pub mime: String,
    /// Filename of BT-125. Mandatory, non-blank.
    pub filename: String,
}

impl Attachment {
    /// CEN receiver-must-accept MIME list. Advisory; the constructor does not restrict to this.
    pub const RECEIVER_MUST_ACCEPT: &'static [&'static str] = &[
        "application/pdf",
        "image/png",
        "image/jpeg",
        "text/csv",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "application/vnd.oasis.opendocument.spreadsheet",
    ];

    /// Bytes + mime + filename. Blank mime or filename is `Err`.
    pub fn new(
        bytes: Vec<u8>,
        mime: impl Into<String>,
        filename: impl Into<String>,
    ) -> Result<Self, AttachmentError> {
        let mime = mime.into();
        let filename = filename.into();
        if mime.trim().is_empty() {
            return Err(AttachmentError::EmptyMime);
        }
        if filename.trim().is_empty() {
            return Err(AttachmentError::EmptyFilename);
        }
        Ok(Self {
            bytes,
            mime,
            filename,
        })
    }

    /// Whether mime is on the CEN receiver-must-accept list.
    pub fn is_universally_accepted(&self) -> bool {
        Self::RECEIVER_MUST_ACCEPT.contains(&self.mime.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_blank_mime_or_filename() {
        assert!(Attachment::new(vec![], "", "x.pdf").is_err());
        assert!(Attachment::new(vec![], "application/pdf", "  ").is_err());
        let a = Attachment::new(b"%PDF".to_vec(), "application/pdf", "terms.pdf").unwrap();
        assert!(a.is_universally_accepted());
    }
}
