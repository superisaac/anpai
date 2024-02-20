use rust_decimal::prelude::*;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone)]
pub enum Value {
    NullV,
    BoolV(bool),
    NumberV(Decimal),
    StrV(String),
    ArrayV(Vec<Value>),
    MapV(BTreeMap<String, Value>),
}

fn fmt_vec<T: fmt::Display>(
    f: &mut fmt::Formatter,
    vec: &Vec<T>,
    prefix: &str,
    suffix: &str,
) -> fmt::Result {
    match write!(f, "{}", prefix) {
        Err(err) => return Err(err),
        _ => (),
    }
    for (i, v) in vec.iter().enumerate() {
        if i > 0 {
            match write!(f, ", {}", v) {
                Err(err) => return Err(err),
                _ => (),
            }
        } else {
            match write!(f, "{}", v) {
                Err(err) => return Err(err),
                _ => (),
            }
        }
    }
    match write!(f, "{}", suffix) {
        Err(err) => return Err(err),
        _ => (),
    }
    Ok(())
}

fn fmt_map<T: fmt::Display>(
    f: &mut fmt::Formatter,
    map: &BTreeMap<String, T>,
    prefix: &str,
    suffix: &str,
) -> fmt::Result {
    match write!(f, "{}", prefix) {
        Err(err) => return Err(err),
        _ => (),
    }
    for (i, (k, v)) in map.iter().enumerate() {
        if i > 0 {
            match write!(f, ", {}:{}", k, v) {
                Err(err) => return Err(err),
                _ => (),
            }
        } else {
            match write!(f, "{}:{}", k, v) {
                Err(err) => return Err(err),
                _ => (),
            }
        }
    }
    match write!(f, "{}", suffix) {
        Err(err) => return Err(err),
        _ => (),
    }
    Ok(())
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::NullV => write!(f, "{}", "null"),
            Value::BoolV(v) => write!(f, "{}", v),
            Value::NumberV(v) => write!(f, "{}", v.normalize()),
            Value::StrV(v) => write!(f, "\"{}\"", v),
            Value::ArrayV(arr) => fmt_vec(f, arr, "[", "]"),
            Value::MapV(map) => fmt_map(f, map, "{", "}"),
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
            Value::ArrayV(_) => "array".to_owned(),
            Value::MapV(_) => "map".to_owned(),
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
