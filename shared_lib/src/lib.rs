#[derive(Clone, Debug)]
pub struct LedgerEntry {
    pub date: String,
    pub amount: f32,
    pub transfer_type: TransferType,
    pub participant: String,
    pub category_id: String,
    pub description: String,
}

#[derive(PartialEq, Clone, Debug)]
pub enum TransferType {
    WithdrawalToExternalAccount,
    DepositFromExternalAccount,
    WithdrawalToInternalAccount,
    DepositFromInternalAccount,
}

impl From<u32> for TransferType {
    fn from(value: u32) -> Self {
        match value {
            0 => TransferType::WithdrawalToExternalAccount,
            1 => TransferType::DepositFromExternalAccount,
            2 => TransferType::WithdrawalToInternalAccount,
            3 => TransferType::DepositFromInternalAccount,
            _ => panic!("Invalid numeric value for TransferType!"),
        }
    }
}
