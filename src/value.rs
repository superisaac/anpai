use rust_decimal::prelude::*;
use std::fmt;

pub const DECIMAL_PLACES: u32 = 30;
#[derive(Clone)]
pub enum Value {
    NumberV(Decimal),
    StrV(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self) {
            Value::NumberV(v) => write!(f, "{}", v),
            Value::StrV(v) => write!(f, "\"{}\"", v),
        }
    }
}

impl Value {
    pub fn data_type(&self) -> String {
        match (self) {
            Value::NumberV(_) => "number".to_owned(),
            Value::StrV(_) => "string".to_owned(),
        }
    }
}
