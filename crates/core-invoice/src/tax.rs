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
    pub percent: crate::numeric::Percentage,
}

impl TaxCategory {
    pub fn vat(code: impl Into<String>, percent: impl Into<crate::numeric::Percentage>) -> Self {
        Self {
            system: TaxSystem::Vat,
            code: code.into(),
            percent: percent.into(),
        }
    }

    pub fn sst(code: impl Into<String>, percent: impl Into<crate::numeric::Percentage>) -> Self {
        Self {
            system: TaxSystem::Sst,
            code: code.into(),
            percent: percent.into(),
        }
    }

    pub fn gst(code: impl Into<String>, percent: impl Into<crate::numeric::Percentage>) -> Self {
        Self {
            system: TaxSystem::Gst,
            code: code.into(),
            percent: percent.into(),
        }
    }
}

/// TaxScheme/cbc:ID on the wire. Never `SST` for PINT-MY.
pub fn wire_scheme(
    profile: crate::profile::Profile,
    system: TaxSystem,
    category: &str,
) -> &'static str {
    use crate::profile::Profile;
    match profile {
        Profile::En16931 | Profile::PeppolBis3 => "VAT",
        Profile::PintMy if category.eq_ignore_ascii_case("TTX") => "AAL",
        Profile::PintMy => "VAT",
        Profile::Pint => match system {
            TaxSystem::Gst => "GST",
            _ => "VAT",
        },
    }
}

pub fn pint_my_category(code: &str) -> bool {
    matches!(code, "SA" | "SE" | "HVG" | "LVG" | "TTX" | "E" | "O")
}
