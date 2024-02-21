use rust_decimal::prelude::*;
use std::cell::Ref;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone)]
pub enum Value {
    NullV,
    BoolV(bool),
    NumberV(Decimal),
    StrV(String),
    ArrayV(RefCell<Vec<Value>>),
    MapV(RefCell<BTreeMap<String, Value>>),
}

fn fmt_vec<T: fmt::Display>(
    f: &mut fmt::Formatter,
    vec: Ref<'_, Vec<T>>,
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
    map: Ref<'_, BTreeMap<String, T>>,
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
            Self::NullV => write!(f, "{}", "null"),
            Self::BoolV(v) => write!(f, "{}", v),
            Self::NumberV(v) => write!(f, "{}", v.normalize()),
            Self::StrV(v) => write!(f, "\"{}\"", v),
            Self::ArrayV(arr) => fmt_vec(f, arr.borrow(), "[", "]"),
            Self::MapV(map) => fmt_map(f, map.borrow(), "{", "}"),
        }
    }
}

impl Value {
    pub fn data_type(&self) -> String {
        match self {
            Self::NullV => "null".to_owned(),
            Self::BoolV(_) => "boolean".to_owned(),
            Self::NumberV(_) => "number".to_owned(),
            Self::StrV(_) => "string".to_owned(),
            Self::ArrayV(_) => "array".to_owned(),
            Self::MapV(_) => "map".to_owned(),
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
