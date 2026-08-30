/// Tax system on the invoice. PINT is the reason this is not "VAT or nothing".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
            "VAT" => Some(Self::Vat),
            "GST" => Some(Self::Gst),
            "SST" | "SALES" | "SERVICE" => Some(Self::Sst),
            "CONSUMPTION" | "CT" => Some(Self::Consumption),
            _ => None,
        }
    }
}

/// A tax category on a line or breakdown (rate + system).
///
/// `percent` is `None` when the family has no IBT-119 (EN `O`, PINT-MY TTX).
/// Zero is a stated 0 % (EN `Z`), not an absent rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxCategory {
    pub system: TaxSystem,
    pub code: String,
    pub percent: Option<crate::numeric::Percentage>,
}

impl TaxCategory {
    pub fn vat(code: impl Into<String>, percent: impl Into<crate::numeric::Percentage>) -> Self {
        Self {
            system: TaxSystem::Vat,
            code: code.into(),
            percent: Some(percent.into()),
        }
    }

    pub fn sst(code: impl Into<String>, percent: impl Into<crate::numeric::Percentage>) -> Self {
        Self {
            system: TaxSystem::Sst,
            code: code.into(),
            percent: Some(percent.into()),
        }
    }

    pub fn gst(code: impl Into<String>, percent: impl Into<crate::numeric::Percentage>) -> Self {
        Self {
            system: TaxSystem::Gst,
            code: code.into(),
            percent: Some(percent.into()),
        }
    }

    /// EN / PINT `O`: no rate on the line.
    pub fn out_of_scope() -> Self {
        Self {
            system: TaxSystem::Vat,
            code: "O".into(),
            percent: None,
        }
    }

    /// PINT-MY TTX: amount-only, scheme AAL, no Percent.
    pub fn ttx() -> Self {
        Self {
            system: TaxSystem::Sst,
            code: "TTX".into(),
            percent: None,
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
        // Production write never stamps Unknown; keep a VAT scheme if tests serialise.
        Profile::Unknown => "VAT",
    }
}

pub fn pint_my_category(code: &str) -> bool {
    matches!(code, "SA" | "SE" | "HVG" | "LVG" | "TTX" | "E" | "O")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vat_cgst_is_not_vat() {
        assert_eq!(TaxSystem::parse("VAT/CGST"), None);
        assert_eq!(TaxSystem::parse("VAT"), Some(TaxSystem::Vat));
    }
}
