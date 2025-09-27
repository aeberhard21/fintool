use serde::de;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use std::fmt;
use strum::{Display, EnumIter, FromRepr};

pub mod stocks;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LedgerEntry {
    pub date: String,
    pub amount: f32,
    #[serde(deserialize_with = "deserialize_transfer_type")]
    pub transfer_type: TransferType,
    pub participant: String,
    pub category: String,
    pub description: String,
    #[serde(serialize_with = "serialize_stock_info")]
    pub stock_info: Option<StockInfo>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StockInfo {
    pub shares: f32,
    pub costbasis: f32,
    pub remaining: f32,
    pub is_buy: bool,
    pub is_split: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FlatLedgerEntry {
    pub date: String,
    pub amount: f32,
    #[serde(serialize_with = "serialize_transfer_type")]
    pub transfer_type: TransferType,
    pub participant: String,
    pub category: String,
    pub description: String,
    pub shares: Option<String>,
    pub costbasis: Option<String>,
    pub remaining: Option<String>,
    pub is_buy: Option<String>,
    pub is_split: Option<String>,
}

impl From<LedgerEntry> for FlatLedgerEntry {
    fn from(l: LedgerEntry) -> Self {
        match l.stock_info {
            Some(s) => FlatLedgerEntry {
                date: l.date,
                amount: l.amount,
                transfer_type: l.transfer_type,
                participant: l.participant,
                category: l.category,
                description: l.description,
                shares: Some(s.shares.to_string()),
                costbasis: Some(s.costbasis.to_string()),
                remaining: Some(s.remaining.to_string()),
                is_buy: Some(s.is_buy.to_string()),
                is_split: Some(s.is_split.to_string()),
            },
            None => FlatLedgerEntry {
                date: l.date,
                amount: l.amount,
                transfer_type: l.transfer_type,
                participant: l.participant,
                category: l.category,
                description: l.description,
                shares: None,
                costbasis: None,
                remaining: None,
                is_buy: None,
                is_split: None,
            },
        }
    }
}

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize, Display, FromRepr, EnumIter)]
pub enum TransferType {
    #[strum(to_string = "Withdrawal")]
    WithdrawalToExternalAccount,
    #[strum(to_string = "Deposit")]
    DepositFromExternalAccount,
    #[strum(to_string = "Withdrawal")]
    WithdrawalToInternalAccount,
    #[strum(to_string = "Deposit")]
    DepositFromInternalAccount,
    ZeroSumChange,
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

impl TransferType {
    pub fn is_deposit(&self) -> bool {
        match self {
            TransferType::DepositFromExternalAccount | TransferType::DepositFromInternalAccount => {
                true
            }
            _ => false,
        }
    }

    pub fn is_withdrawal(&self) -> bool {
        match self {
            TransferType::WithdrawalToExternalAccount
            | TransferType::WithdrawalToInternalAccount => true,
            _ => false,
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

fn serialize_transfer_type<S>(tt: &TransferType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let output = match tt {
        TransferType::DepositFromExternalAccount => "1".to_string(),
        TransferType::DepositFromInternalAccount => "3".to_string(),
        TransferType::WithdrawalToExternalAccount => "0".to_string(),
        TransferType::WithdrawalToInternalAccount => "2".to_string(),
        TransferType::ZeroSumChange => "4".to_string(),
    };
    serializer.serialize_str(&output)
}

fn serialize_stock_info<S>(info: &Option<StockInfo>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let output = match info {
        Some(info) => (
            info.shares.to_string(),
            info.costbasis.to_string(),
            info.remaining.to_string(),
            info.is_buy.to_string(),
            info.is_split.to_string(),
        ),
        None => (
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ),
    };

    serializer.serialize_str(
        format!(
            "{},{},{},{},{}",
            output.0, output.1, output.2, output.3, output.4
        )
        .as_str(),
    )
}
