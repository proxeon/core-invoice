use crate::identifier::Identifier;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditTransfer {
    pub account_id: Identifier,
    pub account_name: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentCard {
    pub pan: String,
    pub holder: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectDebit {
    pub mandate: Option<String>,
    pub creditor_id: Option<Identifier>,
    pub debited_account: Option<Identifier>,
}

/// BG-17 xor BG-18 xor BG-19. Several IBANs are several credit-transfer accounts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentMeans {
    CreditTransfer(Vec<CreditTransfer>),
    Card(PaymentCard),
    DirectDebit(DirectDebit),
}
