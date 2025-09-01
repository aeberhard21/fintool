use std::str::FromStr;

use serde::{Deserialize, Deserializer};
use serde_xml_rs::from_str;
use chrono::{Date, DateTime, FixedOffset, NaiveDateTime};

#[derive(Debug, Deserialize)]
pub struct OFX { 
    #[serde(rename = "BANKMSGSRSV1")]
	pub bank_sign_on_msg     : Option<BankMessage>,
    #[serde(rename = "INVSTMTMSGSRSV1")]
	pub investment_sign_on_msg : Option<InvestmentMessage>,
    #[serde(rename = "SECLISTMSGSRSV1")]
	pub security_list_sign_on_msg : Option<SecuritiesList>
}

#[derive(Debug, Deserialize)]
pub struct BankMessage { 
    #[serde(rename = "STMTTRNRS")]
	pub statement_transaction_response : Vec<StatementTransactionResponse>
}

#[derive(Debug, Deserialize)]
pub struct StatementTransactionResponse { 
    #[serde(rename = "TRNUID")]
	pub transaction_unique_id  : u64, 
    #[serde(rename = "STATUS")]
	pub status  : TransactionStatus,
    #[serde(rename = "STMTRS")]
	pub statement_response  : StatementResponse

}

#[derive(Debug, Deserialize)]
pub struct TransactionStatus {
    #[serde(rename = "CODE")]
	pub code        : u64, 
    #[serde(rename = "SEVERITY")]
	pub severity    : String, 
}

#[derive(Debug, Deserialize)]
pub struct StatementResponse { 
    #[serde(rename = "CURDEF")]
	pub currency_enum   : String, 
    #[serde(rename = "BANKACCTFROM")]
	pub bank_acct_from  : BankAccountFrom,
    #[serde(rename = "BANKACCTTO")]
	pub bank_acct_to    : Option<BankAccountTo>,
    #[serde(rename = "BANKTRANLIST")]
	pub bank_transaction_list    : BankTransactionList,
    #[serde(rename = "LEDGERBAL")]
	pub ledger_balance      : LedgerBalance,
    #[serde(rename = "AVAILBAL")]
	pub available_balance   : AvailableBalance,
}

#[derive(Debug, Deserialize)]
pub struct BankAccountFrom { 
    #[serde(rename = "BANKID")]
	pub bank_id         : u64,
    #[serde(rename = "ACCTID")]
	pub account_id      : u64,
    #[serde(rename = "ACCTTYPE")]
	pub account_type    : String
}

#[derive(Debug, Deserialize)]
pub struct BankAccountTo { 
    #[serde(rename = "BANKID")]
	pub bank_id         : u64,
    #[serde(rename = "ACCTID")]
	pub account_id      : u64,
    #[serde(rename = "ACCTTYPE")]
	pub account_type    : String
}


#[derive(Debug, Deserialize)]
pub struct BankTransactionList { 
    #[serde(rename = "DTSTART")]
	pub date_start         : String, 
    #[serde(rename = "DTEND")]
	pub date_end           : String,
    #[serde(rename = "STMTTRN")]
	pub statement_transaction         : Vec<StatementTransaction>,
}

#[derive(Debug, Deserialize)]
pub struct StatementTransaction { 
    #[serde(rename = "TRNTYPE")]
	pub transaction_type         : String, 
    #[serde(rename = "DTPOSTED")]
	pub date_posted        : String, 
    #[serde(rename = "TRNAMT")]
	pub transaction_amount          : f32, 
    #[serde(rename = "FITID")]
	pub financial_institution_transaction_id           : String,
    #[serde(rename = "CHECKNUM")]
    pub check_number    : Option<u32>,
    #[serde(rename = "NAME")]
	pub name            : String, 
    #[serde(rename = "MEMO")]
	pub memo            : String,
}

#[derive(Debug, Deserialize)]
pub struct LedgerBalance { 
    #[serde(rename = "BALAMT")]
	pub balance_amount          : f32,
    #[serde(rename = "DTASOF")]
	pub date_time_as_of          : String,
}

#[derive(Debug, Deserialize)]
pub struct AvailableBalance { 
    #[serde(rename = "BALAMT")]
	pub balance_amount          : f32,
    #[serde(rename = "DTASOF")]
	pub date_time_as_of          : String
}

#[derive(Debug, Deserialize)]
pub struct InvestmentMessage { 
    #[serde(rename = "INVSTMTTRNRS")]
	pub investment_statement_transaction_response    : Vec<InvestmentTransactionResponse>,
}

#[derive(Debug, Deserialize)]
pub struct InvestmentTransactionResponse { 
    #[serde(rename = "TRNUID")]
	pub transaction_unique_id          : u64, 
    #[serde(rename = "STATUS")]
	pub status          : TransactionStatus,
    #[serde(rename = "INVSTMTRS")]
	pub investment_statement_response       : InvestmentStatementResponse,
}

#[derive(Debug, Deserialize)]
pub struct InvestmentStatementResponse { 
    #[serde(rename = "DTASOF")]
	pub date_time_as_of          : String, 
    #[serde(rename = "CURDEF")]
	pub currency          : String,
    #[serde(rename = "INVACCTFROM")]
	pub investment_account_from     : InvestmentAccountFrom,
    #[serde(rename = "INVTRANLIST")]
	pub investment_transaction_list     : Vec<InvestmentTransactionList>,
    #[serde(rename = "INVPOSLIST")]
	pub investment_position_list      : Vec<InvestmentPositionList>,
    #[serde(rename = "INVBAL")]
	pub investment_balance          : InvestmentBalance,
}

#[derive(Debug, Deserialize)]
pub struct InvestmentAccountFrom {
    #[serde(rename = "BROKERID")]
	pub broker_id        : String, 
    #[serde(rename = "ACCTID")]
	pub account_id          : u64, 
}

#[derive(Debug, Deserialize)]
pub struct InvestmentTransactionList { 
    #[serde(rename = "DTSTART")]
	pub date_start         : String, 
    #[serde(rename = "DTEND")]
	pub date_end           : String, 
    #[serde(rename = "INVBANKTRAN")]
	pub investment_bank_transactions     : Vec<InvestmentBankTransaction>,
    #[serde(rename = "BUYSTOCK")]
	pub buy_stock       : Vec<BuyStock>,
    #[serde(rename = "SELLSTOCK")]
	pub sell_stock      : Vec<SellStock>,
    #[serde(rename = "INCOME")]
	pub income          : Vec<Income>
}

#[derive(Debug, Deserialize)]
pub struct InvestmentBankTransaction { 
    #[serde(rename = "STMTTRN")]
	pub statement_transactions  : Vec<StatementTransaction>,
    #[serde(rename = "SUBACCTFUND")]
	pub sub_account_found     : String,
}

#[derive(Debug, Deserialize)]
pub struct BuyStock { 
    #[serde(rename = "INVBUY")]
	pub investment_buy  : InvestmentBuy,
    #[serde(rename = "BUYTYPE")]
	pub buy_type        : String,
}

#[derive(Debug, Deserialize)]
pub struct SellStock { 
    #[serde(rename = "INVSELL")]
	pub investment_sell : InvestmentSell,
    #[serde(rename = "SELLTYPE")]
	pub sell_type       : String,
}

#[derive(Debug, Deserialize)]
pub struct InvestmentBuy {
    #[serde(rename = "INVTRAN")]
	pub investment_transaction         : InvestmentTransaction,
    #[serde(rename = "SECID")]
	pub security_identifer           : SecurityId,
    #[serde(rename = "UNITS")]
	pub units           : f32,
    #[serde(rename = "UNITPRICE")]
	pub unit_price       : f32, 
    #[serde(rename = "FEES")]
	pub fees            : f32, 
    #[serde(rename = "TOTAL")]
	pub total           : f32,
    #[serde(rename = "SUBACCTSEC")]
	pub sub_account_security      : String, 
    #[serde(rename = "SUBACCTFUND")]
	pub sub_account_fund     : String,
}

#[derive(Debug, Deserialize)] 
pub struct InvestmentSell {
    #[serde(rename = "INVTRAN")]
	pub investment_transaction         : InvestmentTransaction,
    #[serde(rename = "SECID")]
	pub security_identifer           : SecurityId,
    #[serde(rename = "UNITS")]
	pub units           : f32,
    #[serde(rename = "UNITPRICE")]
	pub unit_price       : f32, 
    #[serde(rename = "FEES")]
	pub fees            : f32, 
    #[serde(rename = "TOTAL")]
	pub total           : f32,
    #[serde(rename = "SUBACCTSEC")]
	pub sub_account_security      : String, 
    #[serde(rename = "SUBACCTFUND")]
	pub sub_account_fund     : String,
}



#[derive(Debug, Deserialize)]
pub struct InvestmentTransaction { 
    #[serde(rename = "FITID")]
	pub financial_institution_transaction_id           : String, 
    #[serde(rename = "DTTRADE")]
	pub date_of_trade   : String, 
    #[serde(rename = "MEMO")]
	pub memo            : String,
}

#[derive(Debug, Deserialize)]
pub struct SecurityId { 
    #[serde(rename = "UNIQUEID")]
	pub unique_id       : u64, 
    #[serde(rename = "UNIQUEIDTYPE")]
	pub unique_id_type  : String
}

#[derive(Debug, Deserialize)]
pub struct Income { 
    #[serde(rename = "INVTRAN")]
	pub investment_transaction         : InvestmentTransaction,
    #[serde(rename = "SECID")]
	pub security_identifer           : SecurityId,
    #[serde(rename = "INCOMETYPE")]
	pub income_type      : String, 
    #[serde(rename = "TOTAL")]
	pub total           : f32, 
    #[serde(rename = "SUBACCTSEC")]
	pub sub_account_security      : String, 
    #[serde(rename = "SUBACCTFUND")]
	pub sub_account_fund     : String,
}

#[derive(Debug, Deserialize)]
pub struct InvestmentPositionList {
    #[serde(rename = "POSSTOCK")]
	pub stock_position        : StockPosition
}

#[derive(Debug, Deserialize)]
pub struct StockPosition {
    #[serde(rename = "INVPOS")]
	pub investment_position          : InvestmentPosition,
}

#[derive(Debug, Deserialize)]
pub struct InvestmentPosition { 
    #[serde(rename = "SECID")]
	pub security_identifer           : SecurityId,
    #[serde(rename = "HELDINACCT")]
	pub held_in_account      : String, 
    #[serde(rename = "POSTYPE")]
	pub position_type         : String, 
    #[serde(rename = "UNITS")]
	pub units           : f32, 
    #[serde(rename = "UNITPRICE")]
	pub unit_price       : f32,
    #[serde(rename = "MKTVAL")]
	pub market_value          : f32,
    #[serde(rename = "DTPRICEASOF")]
	pub date_time_price_as_of     : String, 
}

#[derive(Debug, Deserialize)]
pub struct InvestmentBalance { 
    #[serde(rename = "AVAILCASH")]
	pub available_cash       : f32, 
    #[serde(rename = "MARGINBALANCE")]
	pub margin_balance   : f32, 
    #[serde(rename = "SHORTBALANCE")]
	pub short_balance    : f32,
}

#[derive(Debug, Deserialize)]
pub struct SecuritiesList { 
    #[serde(rename = "STOCKINFO")]
	pub stock_info   : Vec<StockInfo>,
    #[serde(rename = "MFINFO")]
	pub mutual_fund_info      : Vec<MutualFundInfo>,
    #[serde(rename = "OTHERINFO")]
	pub other_info   : Vec<OtherInfo>
}

#[derive(Debug, Deserialize)]
pub struct StockInfo {
    #[serde(rename = "SECINFO")]
	pub security_info     : SecurityInfo,
    #[serde(rename = "SECNAME")]
	pub security_name     : String,
    #[serde(rename = "TICKER")]
	pub ticker      : String,
}

#[derive(Debug, Deserialize)]
pub struct SecurityInfo { 
    #[serde(rename = "SECID")]
	pub security_identifer       : SecurityId,
    #[serde(rename = "SECNAME")]
	pub security_name     : String, 
    #[serde(rename = "TICKER")]
	pub ticker      : String
}

#[derive(Debug, Deserialize)]
pub struct MutualFundInfo { 
    #[serde(rename = "SECINFO")]
	pub security_identifer     : SecurityInfo,
    #[serde(rename = "SECNAME")]
	pub security_name     : String,
    #[serde(rename = "TICKER")]
	pub ticker      : String,
}

#[derive(Debug, Deserialize)]
pub struct OtherInfo { 
    #[serde(rename = "SECINFO")]
	pub security_info     : SecurityInfo,
    #[serde(rename = "SECNAME")]
	pub security_name     : String,
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