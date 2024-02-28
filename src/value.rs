use crate::ast::Node;
use crate::eval::{EvalError, Intepreter};
use crate::helpers::{fmt_map, fmt_vec};
extern crate chrono;
extern crate iso8601;

use rust_decimal::prelude::*;
use rust_decimal_macros::*;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::rc::Rc;

// native func
pub type NativeFunc =
    fn(intp: &mut Intepreter, args: HashMap<String, Value>) -> Result<Value, EvalError>;

#[derive(Clone)]
pub struct NativeFuncT(pub NativeFunc);
impl fmt::Debug for NativeFuncT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", "native func")
    }
}

// macro
pub type MacroCb =
    fn(intp: &mut Intepreter, nodes: HashMap<String, Box<Node>>) -> Result<Value, EvalError>;

#[derive(Clone)]
pub struct MacroCbT(pub MacroCb);

impl fmt::Debug for MacroCbT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", "macro")
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    NullV,
    BoolV(bool),
    NumberV(Decimal),
    StrV(String),
    DateTimeV(chrono::DateTime<chrono::FixedOffset>),
    DateV(iso8601::Date),
    TimeV(iso8601::Time),
    DurationV {
        duration: iso8601::Duration,
        negative: bool,
    },
    ArrayV(RefCell<Rc<Vec<Value>>>),
    MapV(RefCell<Rc<BTreeMap<String, Value>>>),
    NativeFuncV {
        func: NativeFuncT,
        arg_names: Vec<String>,
    },
    MacroV {
        callback: MacroCbT,
        arg_names: Vec<String>,
    },
    FuncV {
        func_def: Box<Node>,
    },
}

// FIXME: using more decent way to handle sync
unsafe impl Send for Value {}
unsafe impl Sync for Value {}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NullV => write!(f, "{}", "null"),
            Self::BoolV(v) => write!(f, "{}", v),
            Self::NumberV(v) => write!(f, "{}", v.normalize()),
            Self::StrV(v) => write!(f, "\"{}\"", v),
            Self::DateTimeV(v) => write!(f, "{}", v.format("%Y-%m-%dT%H:%M:%S%:z")),
            Self::DateV(v) => write!(f, "{}", v),
            Self::TimeV(v) => write!(f, "{}", v),
            Self::DurationV { duration, negative } => {
                let sign = if *negative { "-" } else { "" };
                write!(f, "{}{}", sign, duration)
            }
            Self::ArrayV(arr) => fmt_vec(f, arr.borrow().iter(), "[", "]"),
            Self::MapV(map) => fmt_map(f, &map.borrow(), "{", "}"),
            Self::NativeFuncV {
                arg_names: _,
                func: _,
            } => write!(f, "{}", "function"),
            Self::MacroV {
                arg_names: _,
                callback: _,
            } => write!(f, "{}", "macro"),
            Self::FuncV { func_def: _ } => write!(f, "{}", "function"),
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
            Self::DateTimeV(_) => "date time".to_owned(),
            Self::DateV(_) => "date".to_owned(),
            Self::TimeV(_) => "time".to_owned(),
            Self::DurationV {
                duration: _,
                negative: _,
            } => "duration".to_owned(),
            Self::ArrayV(_) => "array".to_owned(),
            Self::MapV(_) => "map".to_owned(),
            Self::NativeFuncV {
                arg_names: _,
                func: _,
            } => "nativefunc".to_owned(),
            Self::MacroV {
                arg_names: _,
                callback: _,
            } => "macro".to_owned(),
            Self::FuncV { func_def: _ } => "function".to_owned(),
        }
    }

    pub fn bool_value(&self) -> bool {
        match self {
            Self::NullV => false,
            Self::BoolV(v) => *v,
            Self::NumberV(v) => *v != dec!(0),
            Self::StrV(v) => v.len() > 0,
            Self::ArrayV(v) => v.borrow().len() > 0,
            Self::MapV(v) => v.borrow().len() > 0,
            _ => true,
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
