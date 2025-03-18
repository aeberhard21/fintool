// #[derive(PartialEq, Clone)]
// pub enum TransferType {
//     WithdrawalToExternalAccount,
//     DepositFromExternalAccount,
//     WithdrawalToInternalAccount,
//     DepositFromInternalAccount,
// }

// impl From<u32> for TransferType {
//     fn from(value: u32) -> Self {
//         match value {
//             0 => TransferType::WithdrawalToExternalAccount,
//             1 => TransferType::DepositFromExternalAccount,
//             2 => TransferType::WithdrawalToInternalAccount,
//             3 => TransferType::DepositFromInternalAccount,
//             _ => panic!("Invalid numeric value for TransferType!"),
//         }
//     }
// }
