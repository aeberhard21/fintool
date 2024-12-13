#[derive(PartialEq)]
pub enum TransferType {
    WidthdrawalToExternalAccount,
    DepositFromExternalAccount,
    WidthdrawalToInternalAccount,
    DepositFromInternalAccount,
}

impl From<u32> for TransferType {
    fn from(value: u32) -> Self {
        match value { 
            0 => TransferType::WidthdrawalToExternalAccount, 
            1 => TransferType::DepositFromExternalAccount, 
            2 => TransferType::WidthdrawalToInternalAccount, 
            3 => TransferType::DepositFromInternalAccount,
            _ => panic!("Invalid numeric value for TransferType!")
        }
    }
}
