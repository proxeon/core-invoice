use crate::FormatError;
use core_invoice::Invoice;

/// UN/CEFACT CII D16B is not implemented. Do not wrap UBL in a CII costume.
pub fn write(_invoice: &Invoice) -> Result<String, FormatError> {
    Err(FormatError::CiiNotImplemented)
}

pub fn read(_xml: &str) -> Result<Invoice, FormatError> {
    Err(FormatError::CiiNotImplemented)
}
