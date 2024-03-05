use crate::ast::Node;
use crate::helpers::{compare_value, escape, fmt_map, fmt_vec};
use crate::values::func::{MacroCbT, NativeFuncT};
use crate::values::range::RangeT;
extern crate chrono;
extern crate iso8601;

use crate::values::temporal::{compare_date, datetime_op, timedelta_to_duration};
use rust_decimal::prelude::*;
use rust_decimal_macros::*;
use std::cell::RefCell;
use std::cmp;
use std::collections::BTreeMap;
use std::fmt;
use std::ops;
use std::rc::Rc;

// value error
#[derive(Clone, Debug)]
pub struct ValueError(pub String);

impl From<String> for ValueError {
    fn from(err: String) -> Self {
        Self(err)
    }
}

impl From<&str> for ValueError {
    fn from(err: &str) -> Self {
        Self(err.to_owned())
    }
}

type ValueResult = Result<Value, ValueError>;

#[derive(Clone, Debug, PartialEq)]
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
    RangeV(RangeT),
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
            Self::StrV(v) => write!(f, "\"{}\"", escape(v)),
            Self::DateTimeV(v) => write!(f, "{}", v.format("%Y-%m-%dT%H:%M:%S%:z")),
            Self::DateV(v) => write!(f, "{}", v),
            Self::TimeV(v) => write!(f, "{}", v),
            Self::DurationV { duration, negative } => {
                let sign = if *negative { "-" } else { "" };
                write!(f, "{}{}", sign, duration)
            }
            Self::RangeV(v) => write!(f, "{}", v),
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
            Self::RangeV(_) => "range".to_owned(),
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

// ops traits
impl ops::Add for Value {
    type Output = ValueResult;

    #[inline(always)]
    fn add(self, other: Self) -> Self::Output {
        match self {
            Self::NumberV(a) => match other {
                Self::NumberV(b) => Ok(Self::NumberV(a + b)),
                _ => Err(ValueError(format!(
                    "canot + number and {}",
                    other.data_type()
                ))),
            },
            Self::StrV(a) => match other {
                Self::StrV(b) => Ok(Self::StrV(a + &b)),
                _ => Err(ValueError(format!(
                    "canot + string and {}",
                    other.data_type()
                ))),
            },
            Self::DateTimeV(dt) => match other {
                Self::DurationV { duration, negative } => {
                    let v = datetime_op(true, dt, duration, negative)?;
                    Ok(Self::DateTimeV(v))
                }
                _ => Err(ValueError(format!(
                    "canot + datetime and {}",
                    other.data_type()
                ))),
            },
            Self::DurationV { duration, negative } => match other {
                Self::DateTimeV(b) => {
                    let v = datetime_op(true, b, duration, negative)?;
                    Ok(Self::DateTimeV(v))
                }
                _ => Err(ValueError(format!(
                    "canot + duration and {}",
                    other.data_type()
                ))),
            },
            _ => Err(ValueError(format!(
                "canot + {} and {}",
                self.data_type(),
                other.data_type()
            ))),
        }
    }
}

impl ops::Sub for Value {
    type Output = ValueResult;

    #[inline(always)]
    fn sub(self, other: Self) -> Self::Output {
        match self {
            Self::NumberV(a) => match other {
                Self::NumberV(b) => Ok(Self::NumberV(a - b)),
                _ => Err(ValueError(format!(
                    "canot - number and {}",
                    other.data_type()
                ))),
            },
            Self::DateTimeV(a) => match other {
                Self::DurationV { duration, negative } => {
                    match datetime_op(false, a, duration, negative) {
                        Ok(v) => Ok(Self::DateTimeV(v)),
                        Err(err) => Err(ValueError(err)),
                    }
                }
                Self::DateTimeV(b) => {
                    let delta = a - b;
                    let (duration, negative) = timedelta_to_duration(delta);
                    Ok(Self::DurationV { duration, negative })
                }
                _ => Err(ValueError(format!(
                    "canot - datetime and {}",
                    other.data_type()
                ))),
            },
            _ => Err(ValueError(format!(
                "canot - {} and {}",
                self.data_type(),
                other.data_type()
            ))),
        }
    }
}

impl ops::Mul for Value {
    type Output = ValueResult;

    #[inline(always)]
    fn mul(self, other: Self) -> Self::Output {
        match self {
            Self::NumberV(a) => match other {
                Self::NumberV(b) => Ok(Self::NumberV(a * b)),
                _ => Err(ValueError(format!(
                    "canot * number and {}",
                    other.data_type()
                ))),
            },
            _ => Err(ValueError(format!(
                "canot * {} and {}",
                self.data_type(),
                other.data_type()
            ))),
        }
    }
}

impl ops::Div for Value {
    type Output = ValueResult;

    #[inline(always)]
    fn div(self, other: Self) -> Self::Output {
        match self {
            Self::NumberV(a) => match other {
                Self::NumberV(b) => Ok(Self::NumberV(a / b)),
                _ => Err(ValueError(format!(
                    "canot / number and {}",
                    other.data_type()
                ))),
            },
            _ => Err(ValueError(format!(
                "canot / {} and {}",
                self.data_type(),
                other.data_type()
            ))),
        }
    }
}

impl ops::Rem for Value {
    type Output = ValueResult;

    #[inline(always)]
    fn rem(self, other: Self) -> Self::Output {
        match self {
            Self::NumberV(a) => match other {
                Self::NumberV(b) => Ok(Self::NumberV(a % b)),
                _ => Err(ValueError(format!(
                    "canot % number and {}",
                    other.data_type()
                ))),
            },
            _ => Err(ValueError(format!(
                "canot % {} and {}",
                self.data_type(),
                other.data_type()
            ))),
        }
    }
}

impl ops::Neg for Value {
    type Output = ValueResult;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        match self {
            Self::NumberV(a) => Ok(Self::NumberV(a.neg())),
            _ => Err(ValueError(format!("canot neg {}", self.data_type()))),
        }
    }
}

impl ops::Not for Value {
    type Output = Value;

    #[inline(always)]
    fn not(self) -> Self::Output {
        Self::BoolV(!self.bool_value())
    }
}

impl cmp::PartialOrd for Value {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        match self {
            Self::NumberV(a) => match other {
                Self::NumberV(b) => Some(compare_value(a, b)),
                _ => None,
            },
            Self::StrV(a) => match other {
                Self::StrV(b) => Some(compare_value(a, b)),
                _ => None,
            },
            Self::DateTimeV(a) => match other {
                Self::DateTimeV(b) => Some(compare_value(a, b)),
                _ => None,
            },
            Self::DateV(a) => match other {
                Self::DateV(b) => compare_date(a, b),
                _ => None,
            },
            _ => None,
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
