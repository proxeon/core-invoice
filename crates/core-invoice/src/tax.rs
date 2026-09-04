//! Tax system and category. SST is in-memory; [`wire_scheme`] never emits TaxScheme `SST`.

/// Tax system on the invoice. PINT is the reason this is not "VAT or nothing".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaxSystem {
    /// VAT (UNCL 5305 / TaxScheme `VAT`).
    Vat,
    /// GST. Wire TaxScheme `GST` on PINT, not PINT-MY.
    Gst,
    /// SST in memory. Never TaxScheme `SST` on the wire (VAT/AAL via [`wire_scheme`]).
    Sst,
    /// Consumption tax.
    Consumption,
}

impl TaxSystem {
    /// Canonical name (`VAT`, `GST`, `SST`, `CONSUMPTION`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vat => "VAT",
            Self::Gst => "GST",
            Self::Sst => "SST",
            Self::Consumption => "CONSUMPTION",
        }
    }

    /// Parse a tax-system name. `VAT/CGST` is not VAT.
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
    /// In-memory tax system. SST is never TaxScheme `SST` on the wire.
    pub system: TaxSystem,
    /// BT-151 / BT-118 category code. `String` (not [`crate::Code`]) so PINT extra
    /// codes (SA, SE, HVG, TTX, …) stay representable without a VAT-only enum.
    pub code: String,
    /// BT-152 / BT-119 rate. `None` when the family has no IBT-119 (EN `O`, PINT-MY TTX).
    pub percent: Option<crate::numeric::Percentage>,
}

impl TaxCategory {
    /// VAT category with a stated rate.
    pub fn vat(code: impl Into<String>, percent: impl Into<crate::numeric::Percentage>) -> Self {
        Self {
            system: TaxSystem::Vat,
            code: code.into(),
            percent: Some(percent.into()),
        }
    }

    /// SST category with a stated rate. Wire TaxScheme is still not `SST`.
    pub fn sst(code: impl Into<String>, percent: impl Into<crate::numeric::Percentage>) -> Self {
        Self {
            system: TaxSystem::Sst,
            code: code.into(),
            percent: Some(percent.into()),
        }
    }

    /// GST category with a stated rate.
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

/// PINT-MY category codes: `SA`, `SE`, `HVG`, `LVG`, `TTX`, `E`, `O`.
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
