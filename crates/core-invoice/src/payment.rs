use crate::identifier::Identifier;

/// BG-17 credit transfer. BT-84 account id (IBAN), BT-85 name, BT-86 provider (BIC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditTransfer {
    pub account_id: Identifier,
    pub account_name: Option<String>,
    pub provider: Option<String>,
}

/// BG-18 card. BT-87 PAN, BT-88 holder. Tests use obviously fake PANs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentCard {
    pub pan: String,
    pub holder: Option<String>,
}

/// BG-19 direct debit. BT-89 mandate, BT-90 creditor id, BT-91 debited account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectDebit {
    pub mandate: Option<String>,
    pub creditor_id: Option<Identifier>,
    pub debited_account: Option<Identifier>,
}

/// BG-17 xor BG-18 xor BG-19. Several IBANs are several credit-transfer accounts.
/// Do not store IBAN as a string beside this enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentMeans {
    CreditTransfer(Vec<CreditTransfer>),
    Card(PaymentCard),
    DirectDebit(DirectDebit),
}
