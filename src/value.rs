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
            Value::NumberV(v) => write!(f, "{}", v.normalize()),
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

#[test]
fn test_decimal_trailing_zeros() {
    let a = Decimal::from_str_exact("7").unwrap();
    let b = Decimal::from_str_exact("2").unwrap();
    let d = a / b;
    assert_eq!(d.to_string(), "3.50");
    assert_eq!(d.normalize().to_string(), "3.5");
}
