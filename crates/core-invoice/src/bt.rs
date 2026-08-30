use std::fmt;

/// Business term id `BT-151`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BtId(pub u16);

impl fmt::Display for BtId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BT-{}", self.0)
    }
}

/// Repeating or distinguishable groups. Index in [`Path`] is **0-based**;
/// `BG-25[2]/BT-151` is the third line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Group {
    Document,
    Seller,
    Buyer,
    Payee,
    TaxRepresentative,
    Delivery,
    Payment,
    DocumentAllowance,
    DocumentCharge,
    Totals,
    /// Tax breakdown (BG-23 / IBG-23), not VAT-only.
    TaxBreakdown,
    Attachment,
    Line,
}

impl Group {
    pub fn bg_id(self) -> Option<u16> {
        Some(match self {
            Self::Document => return None,
            Self::Seller => 4,
            Self::Buyer => 7,
            Self::Payee => 10,
            Self::TaxRepresentative => 11,
            Self::Delivery => 13,
            Self::Payment => 16,
            Self::DocumentAllowance => 20,
            Self::DocumentCharge => 21,
            Self::Totals => 22,
            Self::TaxBreakdown => 23,
            Self::Attachment => 24,
            Self::Line => 25,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Path {
    pub group: Group,
    pub index: Option<usize>,
    pub term: Option<BtId>,
}

impl Path {
    pub fn term(term: BtId) -> Self {
        Self {
            group: Group::Document,
            index: None,
            term: Some(term),
        }
    }

    pub fn at_term(group: Group, index: usize, term: BtId) -> Self {
        Self {
            group,
            index: Some(index),
            term: Some(term),
        }
    }

    pub fn group(group: Group) -> Self {
        Self {
            group,
            index: None,
            term: None,
        }
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.group.bg_id(), self.index, self.term) {
            (None, _, Some(t)) => write!(f, "{t}"),
            (None, _, None) => write!(f, "Invoice"),
            (Some(bg), None, None) => write!(f, "BG-{bg}"),
            (Some(bg), Some(i), None) => write!(f, "BG-{bg}[{i}]"),
            (Some(bg), None, Some(t)) => write!(f, "BG-{bg}/{t}"),
            (Some(bg), Some(i), Some(t)) => write!(f, "BG-{bg}[{i}]/{t}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_shapes() {
        assert_eq!(Path::term(BtId(1)).to_string(), "BT-1");
        assert_eq!(Path::group(Group::Totals).to_string(), "BG-22");
        assert_eq!(
            Path {
                group: Group::Totals,
                index: None,
                term: Some(BtId(109))
            }
            .to_string(),
            "BG-22/BT-109"
        );
        assert_eq!(Path::group(Group::Line).to_string(), "BG-25");
        assert_eq!(
            Path::at_term(Group::Line, 2, BtId(151)).to_string(),
            "BG-25[2]/BT-151"
        );
    }
}
