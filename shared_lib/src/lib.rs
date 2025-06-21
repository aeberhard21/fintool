use serde::de;
use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;
use strum::{Display, FromRepr, EnumIter};
use std::fmt;

#[derive(Clone, Debug, Deserialize)]
pub struct LedgerEntry {
    pub date: String,
    pub amount: f32,
    #[serde(deserialize_with = "deserialize_transfer_type")]
    pub transfer_type: TransferType,
    pub participant: String,
    pub category: String,
    pub description: String,
    pub ancillary_f32 : f32,
    pub stock_info: Option<StockInfo>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StockInfo {
    pub shares: f32,
    pub costbasis: f32,
    pub remaining: f32,
    pub is_buy: bool,
    pub is_split: bool,
}

#[derive(PartialEq, Clone, Debug, Deserialize, Display, FromRepr, EnumIter)]
pub enum TransferType {
    #[strum(to_string = "Withdrawal")]
    WithdrawalToExternalAccount,
    #[strum(to_string = "Deposit")]
    DepositFromExternalAccount,
    #[strum(to_string = "Withdrawal")]
    WithdrawalToInternalAccount,
    #[strum(to_string = "Deposit")]
    DepositFromInternalAccount,
    ZeroSumChange
}

impl From<u32> for TransferType {
    fn from(value: u32) -> Self {
        match value {
            0 => TransferType::WithdrawalToExternalAccount,
            1 => TransferType::DepositFromExternalAccount,
            2 => TransferType::WithdrawalToInternalAccount,
            3 => TransferType::DepositFromInternalAccount,
            4 => TransferType::ZeroSumChange,
            _ => panic!("Invalid numeric value for TransferType!"),
        }
    }
}

pub fn deserialize_transfer_type<'de, D>(deserializer: D) -> Result<TransferType, D::Error>
where
    D: Deserializer<'de>,
{
    struct TransferTypeVisitor;

    impl<'de> Visitor<'de> for TransferTypeVisitor {
        type Value = TransferType;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an integer representing a transfer type (0, 1, 2, 3, 4)")
        }

        fn visit_i64<E>(self, value: i64) -> Result<TransferType, E>
        where
            E: de::Error,
        {
            match value {
                0 => Ok(TransferType::WithdrawalToExternalAccount),
                1 => Ok(TransferType::DepositFromExternalAccount),
                2 => Ok(TransferType::WithdrawalToInternalAccount),
                3 => Ok(TransferType::DepositFromInternalAccount),
                4 => Ok(TransferType::ZeroSumChange),
                _ => Err(E::unknown_variant(
                    &value.to_string(),
                    &["0", "1", "2", "3", "4"],
                )),
            }
        }
    }
    deserializer.deserialize_i64(TransferTypeVisitor)
}
