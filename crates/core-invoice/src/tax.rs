/// Tax system on the invoice. PINT is the reason this is not "VAT or nothing".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaxSystem {
    Vat,
    Gst,
    Sst,
    Consumption,
}

impl TaxSystem {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vat => "VAT",
            Self::Gst => "GST",
            Self::Sst => "SST",
            Self::Consumption => "CONSUMPTION",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "VAT" | "VAT/CGST" => Some(Self::Vat),
            "GST" => Some(Self::Gst),
            "SST" | "SALES" | "SERVICE" => Some(Self::Sst),
            "CONSUMPTION" | "CT" => Some(Self::Consumption),
            _ => None,
        }
    }
}

/// A tax category on a line or breakdown (rate + system).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxCategory {
    pub system: TaxSystem,
    pub code: String,
    pub percent: rust_decimal::Decimal,
}

impl TaxCategory {
    pub fn vat(code: impl Into<String>, percent: rust_decimal::Decimal) -> Self {
        Self {
            system: TaxSystem::Vat,
            code: code.into(),
            percent,
        }
    }

    pub fn sst(code: impl Into<String>, percent: rust_decimal::Decimal) -> Self {
        Self {
            system: TaxSystem::Sst,
            code: code.into(),
            percent,
        }
    }

    pub fn gst(code: impl Into<String>, percent: rust_decimal::Decimal) -> Self {
        Self {
            system: TaxSystem::Gst,
            code: code.into(),
            percent,
        }
    }
}
