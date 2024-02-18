use rust_decimal::prelude::*;
use std::fmt;

#[derive(Clone)]
pub enum Value {
    NullV,
    BoolV(bool),
    NumberV(Decimal),
    StrV(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::NullV => write!(f, "{}", "null"),
            Value::BoolV(v) => write!(f, "{}", v),
            Value::NumberV(v) => write!(f, "{}", v),
            Value::StrV(v) => write!(f, "\"{}\"", v),
        }
    }
}

impl Value {
    pub fn data_type(&self) -> String {
        match self {
            Value::NullV => "null".to_owned(),
            Value::BoolV(_) => "boolean".to_owned(),
            Value::NumberV(_) => "number".to_owned(),
            Value::StrV(_) => "string".to_owned(),
        }
    }
}
