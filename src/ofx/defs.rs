use std::str::FromStr;

use serde::{Deserialize, Deserializer};
use serde_xml_rs::from_str;
use chrono::{Date, DateTime, FixedOffset, NaiveDateTime};

#[derive(Debug, Deserialize)]
struct OFX { 
    #[serde(rename = "BANKMSGSRV1")]
    bank_sign_on_msg     : Option<BankMessage>,
    #[serde(rename = "INVSTMTMSGSRSV1")]
    investment_sign_on_msg : Option<InvestmentMessage>,
    #[serde(rename = "SECLISTMSGSRSV1")]
    security_list_sign_on_msg : Option<SecuritiesList>
}

#[derive(Debug, Deserialize)]
struct BankMessage { 
    #[serde(rename = "STMTTRNRS")]
    statement_transaction_response : Vec<StatementTransactionResponse>
}

#[derive(Debug, Deserialize)]
struct StatementTransactionResponse { 
    #[serde(rename = "TRNUID")]
    transaction_unique_id  : u32, 
    #[serde(rename = "STATUS")]
    status  : TransactionStatus,
    #[serde(rename = "STMTRS")]
    statement_response  : StatementResponse

}

#[derive(Debug, Deserialize)]
struct TransactionStatus {
    #[serde(rename = "CODE")]
    code        : u32, 
    #[serde(rename = "SEVERITY")]
    severity    : String, 
}

#[derive(Debug, Deserialize)]
struct StatementResponse { 
    #[serde(rename = "CURDEF")]
    currency_enum   : String, 
    #[serde(rename = "BANKACCTFROM")]
    bank_acct_from  : BankAccountFrom,
    #[serde(rename = "BANKACCTTO")]
    bank_acct_to    : Option<BankAccountTo>,
    #[serde(rename = "BANKTRANLIST")]
    bank_transaction_list    : BankTransactionList,
    #[serde(rename = "LEDGERBAL")]
    ledger_balance      : LedgerBalance,
    #[serde(rename = "AVAILBAL")]
    available_balance   : AvailableBalance,
}

#[derive(Debug, Deserialize)]
struct BankAccountFrom { 
    #[serde(rename = "BANKID")]
    bank_id         : u32,
    #[serde(rename = "ACCTID")]
    account_id      : u32,
    #[serde(rename = "ACCTTYPE")]
    account_type    : String
}

#[derive(Debug, Deserialize)]
struct BankAccountTo { 
    #[serde(rename = "BANKID")]
    bank_id         : u32,
    #[serde(rename = "ACCTID")]
    account_id      : u32,
    #[serde(rename = "ACCTTYPE")]
    account_type    : String
}


#[derive(Debug, Deserialize)]
struct BankTransactionList { 
    #[serde(rename = "DTSTART", deserialize_with  = "deserialize_datetime")]
    date_start         : DateTime<FixedOffset>, 
    #[serde(rename = "DTEND", deserialize_with  = "deserialize_datetime")]
    date_end           : DateTime<FixedOffset>,
    #[serde(rename = "STMTTRN")]
    statement_transaction         : Vec<StatementTransaction>,
}

#[derive(Debug, Deserialize)]
struct StatementTransaction { 
    #[serde(rename = "TRNTYPE")]
    transaction_type         : String, 
    #[serde(rename = "DTPOSTED", deserialize_with  = "deserialize_datetime")]
    date_posted        : DateTime<FixedOffset>, 
    #[serde(rename = "TRNAMT")]
    transaction_amount          : f32, 
    #[serde(rename = "FITID")]
    financial_institution_transaction_id           : u64,
    #[serde(rename = "NAME")]
    name            : String, 
    #[serde(rename = "MEMO")]
    memo            : String,
}

#[derive(Debug, Deserialize)]
struct LedgerBalance { 
    #[serde(rename = "BALAMT")]
    balance_amount          : f32,
    #[serde(rename = "DTASOF", deserialize_with  = "deserialize_datetime")]
    date_time_as_of          : DateTime<FixedOffset>,
}

#[derive(Debug, Deserialize)]
struct AvailableBalance { 
    #[serde(rename = "BALAMT")]
    balance_amount          : f32,
    #[serde(rename = "DTASOF", deserialize_with  = "deserialize_datetime")]
    date_time_as_of          : DateTime<FixedOffset>
}

#[derive(Debug, Deserialize)]
struct InvestmentMessage { 
    #[serde(rename = "INVSTMTTRNRS")]
    investment_statement_transaction_response    : Vec<InvestmentTransactionResponse>,
}

#[derive(Debug, Deserialize)]
struct InvestmentTransactionResponse { 
    #[serde(rename = "TRNUID")]
    transaction_unique_id          : u32, 
    #[serde(rename = "STATUS")]
    status          : TransactionStatus,
    #[serde(rename = "INVSTMTRS")]
    investment_statement_response       : InvestmentStatementResponse,
}

#[derive(Debug, Deserialize)]
struct InvestmentStatementResponse { 
    #[serde(rename = "DTASOF", deserialize_with  = "deserialize_datetime")]
    date_time_as_of          : DateTime<FixedOffset>, 
    #[serde(rename = "CURDEF")]
    currency          : String,
    #[serde(rename = "INVACCTFROM")]
    investment_account_from     : InvestmentAccountFrom,
    #[serde(rename = "INVTRANLIST")]
    investment_transaction_list     : Vec<InvestmentTransactionList>,
    #[serde(rename = "INVPOSLIST")]
    investment_position_list      : Vec<InvestmentPositionList>,
    #[serde(rename = "INVBAL")]
    investment_balance          : InvestmentBalance,
}

#[derive(Debug, Deserialize)]
struct INVACCTFROM {
    #[serde(rename = "BROKERID")]
    broker_id        : String, 
    #[serde(rename = "ACCTID")]
    account_id          : u32, 
}

#[derive(Debug, Deserialize)]
struct InvestmentTransactionList { 
    #[serde(rename = "DTSTART", deserialize_with  = "deserialize_datetime")]
    date_start         : DateTime<FixedOffset>, 
    #[serde(rename = "DTEND", deserialize_with  = "deserialize_datetime")]
    date_end           : DateTime<FixedOffset>, 
    #[serde(rename = "INVBANKTRAN")]
    investment_bank_transactions     : Vec<InvestmentBankTransaction>,
    #[serde(rename = "BUYSTOCK")]
    buy_stock       : Vec<BuyStock>,
    #[serde(rename = "SELLSTOCK")]
    sell_stock      : Vec<SellStock>,
    #[serde(rename = "INCOME")]
    income          : Vec<Income>
}

#[derive(Debug, Deserialize)]
struct InvestmentBankTransaction { 
    #[serde(rename = "STMTTRN")]
    statement_transactions  : Vec<StatementTransaction>,
    #[serde(rename = "SUBACCTFUND")]
    SUBACCTFUND     : String,
}

#[derive(Debug, Deserialize)]
struct BuyStock { 
    #[serde(rename = "INVBUY")]
    investment_buy  : InvestmentBuy,
    #[serde(rename = "BUYTYPE")]
    buy_type        : String,
}

#[derive(Debug, Deserialize)]
struct SellStock { 
    #[serde(rename = "INVSELL")]
    investment_sell : InvestmentSell,
    #[serde(rename = "SELLTYPE")]
    sell_type       : String,
}

#[derive(Debug, Deserialize)]
struct InvestmentBuy {
    #[serde(rename = "INVTRAN")]
    investment_transaction         : InvestmentTransaction,
    #[serde(rename = "SECID")]
    security_identifer           : SecurityId,
    #[serde(rename = "UNITS")]
    units           : f32,
    #[serde(rename = "UNITPRICE")]
    unit_price       : f32, 
    #[serde(rename = "FEES")]
    fees            : f32, 
    #[serde(rename = "TOTAL")]
    total           : f32,
    #[serde(rename = "SUBACCTSEC")]
    sub_account_security      : String, 
    #[serde(rename = "SUBACCTFUND")]
    sub_account_fund     : String,
}

#[derive(Debug, Deserialize)] 
struct InvestmentSell {
    #[serde(rename = "INVTRAN")]
    investment_transaction         : InvestmentTransaction,
    #[serde(rename = "SECID")]
    security_identifer           : SecurityId,
    #[serde(rename = "UNITS")]
    units           : f32,
    #[serde(rename = "UNITPRICE")]
    unit_price       : f32, 
    #[serde(rename = "FEES")]
    fees            : f32, 
    #[serde(rename = "TOTAL")]
    total           : f32,
    #[serde(rename = "SUBACCTSEC")]
    sub_account_security      : String, 
    #[serde(rename = "SUBACCTFUND")]
    sub_account_fund     : String,
}



#[derive(Debug, Deserialize)]
struct InvestmentTransaction { 
    #[serde(rename = "FITID")]
    financial_institution_transaction_id           : String, 
    #[serde(rename = "DTTRADE", deserialize_with  = "deserialize_datetime")]
    date_of_trade   : DateTime<FixedOffset>, 
    #[serde(rename = "MEMO")]
    memo            : String,
}

#[derive(Debug, Deserialize)]
struct SecurityId { 
    #[serde(rename = "UNIQUEID")]
    unique_id       : u32, 
    #[serde(rename = "UNIQUEIDTYPE")]
    unique_id_type  : String
}

#[derive(Debug, Deserialize)]
struct Income { 
    #[serde(rename = "INVTRAN")]
    investment_transaction         : InvestmentTransaction,
    #[serde(rename = "SECID")]
    security_identifer           : SecurityId,
    #[serde(rename = "INCOMETYPE")]
    income_type      : String, 
    #[serde(rename = "TOTAL")]
    total           : f32, 
    #[serde(rename = "SUBACCTSEC")]
    sub_account_security      : String, 
    #[serde(rename = "SUBACCTFUND")]
    sub_account_fund     : String,
}

#[derive(Debug, Deserialize)]
struct InvestmentPositionList {
    #[serde(rename = "POSSTOCK")]
    stock_position        : StockPosition
}

#[derive(Debug, Deserialize)]
struct StockPosition {
    #[serde(rename = "INVPOS")]
    investment_position          : InvestmentPosition,
}

#[derive(Debug, Deserialize)]
struct InvestmentPosition { 
    #[serde(rename = "SECID")]
    security_identifer           : SecurityId,
    #[serde(rename = "HELDINACCT")]
    held_in_account      : String, 
    #[serde(rename = "POSTYPE")]
    position_type         : String, 
    #[serde(rename = "UNITS")]
    units           : f32, 
    #[serde(rename = "UNITPRICE")]
    unit_price       : f32,
    #[serde(rename = "MKTVAL")]
    market_value          : f32,
    #[serde(rename = "DTPRICEASOF", deserialize_with  = "deserialize_datetime")]
    date_time_price_as_of     : DateTime<FixedOffset>, 
}

#[derive(Debug, Deserialize)]
struct InvestmentBalance { 
    #[serde(rename = "AVAILCASH")]
    available_cash       : f32, 
    #[serde(rename = "MARGINBALANCE")]
    margin_balance   : f32, 
    #[serde(rename = "SHORTBALANCE")]
    short_balance    : f32,
}

#[derive(Debug, Deserialize)]
struct SecuritiesList { 
    #[serde(rename = "STOCKINFO")]
    stock_info   : Vec<StockInfo>,
    #[serde(rename = "MFINFO")]
    mutual_fund_info      : Vec<MutualFundInfo>,
    #[serde(rename = "OTHERINFO")]
    other_info   : Vec<OtherInfo>
}

#[derive(Debug, Deserialize)]
struct StockInfo {
    #[serde(rename = "SECINFO")]
    security_info     : SecurityInfo,
    #[serde(rename = "SECNAME")]
    security_name     : String,
    #[serde(rename = "TICKER")]
    ticker      : String,
}

#[derive(Debug, Deserialize)]
struct SecurityInfo { 
    #[serde(rename = "SECID")]
    security_identifer       : SecurityId,
    #[serde(rename = "SECNAME")]
    security_name     : String, 
    #[serde(rename = "TICKER")]
    ticker      : String
}

#[derive(Debug, Deserialize)]
struct MutualFundInfo { 
    #[serde(rename = "SECINFO")]
    security_identifer     : SecurityInfo,
    #[serde(rename = "SECNAME")]
    security_name     : String,
    #[serde(rename = "TICKER")]
    ticker      : String,
}

#[derive(Debug, Deserialize)]
struct OtherInfo { 
    #[serde(rename = "SECINFO")]
    security_info     : SecurityInfo,
    #[serde(rename = "SECNAME")]
    security_name     : String,
}

#[repr(u16)]
enum CODE { 
    SUCCESS             = 0,  
    GENERAL_ERROR       = 2000,
    UNSUPPORTED_VERSION_ERROR = 2021,
    REQUESTED_ELEMENT_UNKNOWN_WARNING = 2028, 
    AUTH_ERROR          = 3000,
    MFACHALLENGE_ERROR  = 3001, 
    UNABLE_TO_PROCESS_EMBEDDED_TRN_ERROR = 6502,
    FI_MISSING_ERROR    = 13504,
    SERVER_ERROR        = 13505,
    MUST_CHANGE_PWD_INFO    = 15000,
    SIGNON_INVALID_ERROR      = 15501,
    USER_PASS_LOCKOUT_ERROR  = 15502,
    EMPTY_SIGNON_NOT_SUPPORTED_ERROR = 15506, 
    SIGNON_INVALID_PWD_ERROR  = 15507, 
    CLIENTUID_ERROR     = 15510,
    CONTACT_FIN_INST_ERROR    = 15511,
    AUTHTOKEN_INVALID_ERROR   = 15512,
    OFX_SERVER_ACCESSTOKEN_ERROR    = 15514, 
    ACCESS_TOKEN_INVALID_ERROR      = 15515, 
    ACCESS_TOKEN_EXPIRED_ERROR      = 15516
}

enum BALANCE_TYPE { 
    DOLLAR, 
    PERCENT, 
    NUMBER
}

enum SEVERITY { 
    INFO, 
    WARN, 
    ERROR
}

fn deserialize_datetime<'de, D>(deserializer : D) -> Result<DateTime<FixedOffset>, D::Error> 
where D: Deserializer <'de> 
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let dt = DateTime::parse_from_str(s.as_str(), "%Y%m%D%H%M%S%.3f [%z").unwrap();
    return Ok(dt);
}