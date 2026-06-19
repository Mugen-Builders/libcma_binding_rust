use hex;
use ethers_core::types::{Address};
use json::{JsonValue, object};
use crate::parser::CmaVoucher;

pub fn hex_to_string(hex: &str) -> Result<String, Box<dyn std::error::Error>> {
    let hexstr = hex.strip_prefix("0x").unwrap_or(hex);
    let bytes = hex::decode(hexstr).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let s = String::from_utf8(bytes).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(s)
}

pub trait ToAddress {
    fn to_address(&self) -> Result<Address, String>;
}

impl ToAddress for str {
    fn to_address(&self) -> Result<Address, String> {
        self.parse::<Address>()
            .map_err(|e| format!("Invalid Ethereum address: {}", e))
    }
}

impl ToAddress for String {
    fn to_address(&self) -> Result<Address, String> {
        self.as_str().to_address()
    }
}

#[allow(dead_code)]
pub trait ToJson {
    fn to_json(&self) -> JsonValue;
}

impl ToJson for CmaVoucher {
    fn to_json(&self) -> JsonValue {
        object! {
            "destination" => format!("{}", self.destination),
            "value" => format!("{}", self.value),
            "payload" => format!("{}", self.payload),
        }
    }
}