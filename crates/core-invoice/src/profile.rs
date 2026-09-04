//! Usage specification (BT-24). Profiles are siblings, not a ladder.

use crate::tax::TaxSystem;

/// EN 16931 edition. `En2026` is classified until CEN artefacts exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edition {
    /// EN 16931:2017+A1. Every shipped profile uses this.
    En2017A1,
    /// Classified 2026 edition. Unimplemented until CEN artefacts exist.
    En2026,
}

impl Edition {
    /// Whether this edition has artefacts in this crate.
    ///
    /// [`Edition::En2026`] is `false` until those artefacts exist. No shipped
    /// profile returns it from [`Profile::edition`].
    pub fn is_implemented(self) -> bool {
        matches!(self, Self::En2017A1)
    }
}

/// Result of matching BT-24. Profiles are siblings, not a ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileLookup {
    /// A shipped billing profile.
    Profile(Profile),
    /// Self-billing (or other) process URN; do not validate as billing.
    WrongProcess,
    /// BT-24 did not match a shipped billing or self-billing URN.
    Unknown,
}

/// Usage specification (BT-24). Not a ladder: Peppol BIS and PINT are siblings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profile {
    /// CEN EN 16931-1 core (2017+A1 until 2026 artefacts exist).
    En16931,
    /// Peppol BIS Billing 3.0 (EU VAT CIUS).
    PeppolBis3,
    /// Peppol International base (tax is not only VAT).
    Pint,
    /// PINT-MY specialisation (SST, TIN/BRN schemes).
    PintMy,
    /// Parsed BT-24 did not match a shipped billing profile. Do not invent En16931.
    Unknown,
}

impl Profile {
    /// CLI / report slug (`en16931`, `peppol`, `pint`, `pint-my`, `unknown`).
    pub fn slug(self) -> &'static str {
        match self {
            Self::En16931 => "en16931",
            Self::PeppolBis3 => "peppol",
            Self::Pint => "pint",
            Self::PintMy => "pint-my",
            Self::Unknown => "unknown",
        }
    }

    /// Parse a slug (`en16931`, `peppol`, `pint`, `pint-my`, plus aliases). `None` if unknown.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "en16931" | "en-16931" | "core" => Some(Self::En16931),
            "peppol" | "bis3" | "peppol-bis-3" => Some(Self::PeppolBis3),
            "pint" => Some(Self::Pint),
            "pint-my" | "pintmy" | "my" => Some(Self::PintMy),
            _ => None,
        }
    }

    /// Canonical BT-24 specification identifier. Empty for [`Profile::Unknown`].
    pub fn specification_id(self) -> &'static str {
        match self {
            Self::En16931 => "urn:cen.eu:en16931:2017",
            Self::PeppolBis3 => {
                "urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0"
            }
            Self::Pint => "urn:peppol:pint:billing-1",
            Self::PintMy => "urn:peppol:pint:billing-1@my-1",
            // Unknown BT-24 stays unknown; CORE-SPEC-01 is Fatal. Do not silently select En16931.
            Self::Unknown => "",
        }
    }

    /// BT-23 / ProfileID required by this profile. EN 16931 has none (omit, do not invent).
    pub fn process_id(self) -> Option<&'static str> {
        match self {
            Self::PeppolBis3 => Some("urn:fdc:peppol.eu:2017:poacc:billing:01:1.0"),
            Self::Pint | Self::PintMy => Some("urn:peppol:bis:billing"),
            Self::En16931 | Self::Unknown => None,
        }
    }

    /// Tax systems this profile accepts.
    pub fn tax_systems(self) -> &'static [TaxSystem] {
        match self {
            Self::En16931 | Self::PeppolBis3 => &[TaxSystem::Vat],
            Self::Pint => &[
                TaxSystem::Vat,
                TaxSystem::Gst,
                TaxSystem::Sst,
                TaxSystem::Consumption,
            ],
            // SST in memory; wire TaxScheme is VAT / AAL. GST is Pint, not PintMy.
            Self::PintMy => &[TaxSystem::Sst],
            Self::Unknown => &[],
        }
    }

    /// Whether `system` is in [`Self::tax_systems`].
    pub fn allows(self, system: TaxSystem) -> bool {
        self.tax_systems().contains(&system)
    }

    /// Comma-separated known slugs for CLI help.
    pub fn known_slugs() -> &'static str {
        "en16931, peppol, pint, pint-my"
    }

    /// Edition this profile validates. Always [`Edition::En2017A1`] until 2026 artefacts exist.
    pub fn edition(self) -> Edition {
        Edition::En2017A1
    }

    /// Peppol BIS 3.0 is a CIUS of EN 16931. PINT is not.
    pub fn is_conformant_cius(self) -> bool {
        matches!(self, Self::PeppolBis3)
    }

    /// Peppol extra_rules. PINT / PINT-MY / EN core do **not** inherit them.
    pub fn extra_rules(self) -> &'static [crate::rules::Rule] {
        match self {
            Self::PeppolBis3 => crate::peppol::RULES,
            _ => &[],
        }
    }

    /// Canonical Peppol BIS 3.0 BT-24 prefix (the CIUS URN).
    pub const PEPPOL_BIS3_PREFIX: &'static str =
        "urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0";

    /// Prefix match for BT-24. Never `contains("pint")`.
    pub fn for_specification_id(id: &str) -> ProfileLookup {
        let id = id.trim();
        if id.contains('*') {
            return ProfileLookup::Unknown;
        }
        if id.starts_with("urn:peppol:pint:selfbilling-1@my-1")
            || id.starts_with(
                "urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:selfbilling:3.0",
            )
        {
            return ProfileLookup::WrongProcess;
        }
        if id.starts_with("urn:peppol:pint:billing-1@my-1") {
            return ProfileLookup::Profile(Self::PintMy);
        }
        if id.starts_with("urn:peppol:pint:billing-1@") {
            return ProfileLookup::Profile(Self::Pint);
        }
        if id.starts_with("urn:peppol:pint:billing-1") {
            return ProfileLookup::Profile(Self::Pint);
        }
        if id.starts_with(Self::PEPPOL_BIS3_PREFIX) {
            return ProfileLookup::Profile(Self::PeppolBis3);
        }
        if id.starts_with("urn:cen.eu:en16931:2017#compliant#") || id == "urn:cen.eu:en16931:2017" {
            return ProfileLookup::Profile(Self::En16931);
        }
        ProfileLookup::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_is_prefix_not_contains() {
        assert_eq!(
            Profile::for_specification_id("urn:peppol:pint:billing-1@my-1"),
            ProfileLookup::Profile(Profile::PintMy)
        );
        assert_eq!(
            Profile::for_specification_id("urn:peppol:pint:billing-1"),
            ProfileLookup::Profile(Profile::Pint)
        );
        assert_eq!(
            Profile::for_specification_id(Profile::PEPPOL_BIS3_PREFIX),
            ProfileLookup::Profile(Profile::PeppolBis3)
        );
        assert_eq!(
            Profile::for_specification_id("urn:cen.eu:en16931:2017"),
            ProfileLookup::Profile(Profile::En16931)
        );
        assert_eq!(
            Profile::for_specification_id(
                "urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0"
            ),
            ProfileLookup::Profile(Profile::En16931)
        );
        assert_eq!(
            Profile::for_specification_id("urn:peppol:pint:selfbilling-1@my-1"),
            ProfileLookup::WrongProcess
        );
        assert_eq!(
            Profile::for_specification_id(
                "urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:selfbilling:3.0"
            ),
            ProfileLookup::WrongProcess
        );
        assert_eq!(
            Profile::for_specification_id("urn:example:painting"),
            ProfileLookup::Unknown
        );
        assert!(!Profile::PintMy.is_conformant_cius());
        assert!(Profile::PeppolBis3.is_conformant_cius());
        assert_eq!(
            Profile::PeppolBis3.process_id(),
            Some("urn:fdc:peppol.eu:2017:poacc:billing:01:1.0")
        );
        assert_eq!(Profile::En16931.process_id(), None);
        assert!(Profile::En16931.edition().is_implemented());
        assert!(!Edition::En2026.is_implemented());
        for p in [
            Profile::En16931,
            Profile::PeppolBis3,
            Profile::Pint,
            Profile::PintMy,
            Profile::Unknown,
        ] {
            assert_eq!(p.edition(), Edition::En2017A1);
            assert!(p.edition().is_implemented());
        }
    }

    #[test]
    fn siblings_sst() {
        assert!(!Profile::PeppolBis3.allows(TaxSystem::Sst));
        assert!(Profile::PintMy.allows(TaxSystem::Sst));
    }
}
