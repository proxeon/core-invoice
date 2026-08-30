use crate::tax::TaxSystem;

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
}

impl Profile {
    pub fn slug(self) -> &'static str {
        match self {
            Self::En16931 => "en16931",
            Self::PeppolBis3 => "peppol",
            Self::Pint => "pint",
            Self::PintMy => "pint-my",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "en16931" | "en-16931" | "core" => Some(Self::En16931),
            "peppol" | "bis3" | "peppol-bis-3" => Some(Self::PeppolBis3),
            "pint" => Some(Self::Pint),
            "pint-my" | "pintmy" | "my" => Some(Self::PintMy),
            _ => None,
        }
    }

    pub fn specification_id(self) -> &'static str {
        match self {
            Self::En16931 => "urn:cen.eu:en16931:2017",
            Self::PeppolBis3 => {
                "urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0"
            }
            Self::Pint => "urn:peppol:pint:billing-1",
            Self::PintMy => "urn:peppol:pint:billing-1@my-1",
        }
    }

    /// Tax systems this profile accepts.
    pub fn tax_systems(self) -> &'static [TaxSystem] {
        match self {
            Self::En16931 | Self::PeppolBis3 => &[TaxSystem::Vat],
            Self::Pint | Self::PintMy => &[
                TaxSystem::Vat,
                TaxSystem::Gst,
                TaxSystem::Sst,
                TaxSystem::Consumption,
            ],
        }
    }

    pub fn allows(self, system: TaxSystem) -> bool {
        self.tax_systems().contains(&system)
    }
}
