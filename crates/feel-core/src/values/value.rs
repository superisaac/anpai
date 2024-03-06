use super::super::ast::Node;
use super::super::eval::EvalResult;
use super::super::helpers::{compare_value, escape, fmt_map, fmt_vec};

extern crate chrono;
extern crate iso8601;

use super::func::{MacroT, NativeFunc};
use super::range::RangeT;
use super::temporal::{compare_date, datetime_op, timedelta_to_duration};
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

// type error
#[derive(Clone, Debug)]
pub struct TypeError(pub String);

impl From<String> for TypeError {
    fn from(err: String) -> Self {
        Self(err)
    }
}

impl From<&str> for TypeError {
    fn from(err: &str) -> Self {
        Self(err.to_owned())
    }
}

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
        func: NativeFunc,
        require_args: Vec<String>,
        optional_args: Vec<String>,
    },
    MacroV {
        macro_: MacroT,
        require_args: Vec<String>,
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
                require_args: _,
                optional_args: _,
                func: _,
            } => write!(f, "{}", "function"),
            Self::MacroV {
                require_args: _,
                macro_: _,
            } => write!(f, "{}", "function"),
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
                require_args: _,
                optional_args: _,
                func: _,
            } => "nativefunc".to_owned(),
            Self::MacroV {
                require_args: _,
                macro_: _,
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

    pub fn parse_number(&self) -> Result<Decimal, ValueError> {
        match self {
            Self::StrV(s) => match Decimal::from_str_exact(s) {
                Ok(d) => Ok(d),
                Err(err) => Err(ValueError(err.to_string())),
            },
            Self::NumberV(n) => Ok(*n),
            _ => Err(ValueError("fail to parse number".to_owned())),
        }
    }

    pub fn expect_string(&self, hint: &str) -> Result<String, ValueError> {
        if let Self::StrV(s) = self {
            return Ok(s.clone());
        }
        Err(ValueError(format!(
            "{}, expect string, found {}",
            hint,
            self.data_type()
        )))
    }

    pub fn expect_number(&self, hint: &str) -> Result<Decimal, ValueError> {
        if let Self::NumberV(n) = self {
            return Ok(n.clone());
        }
        Err(ValueError(format!(
            "{}, expect number, but {} found",
            hint,
            self.data_type()
        )))
    }

    pub fn expect_integer(&self) -> Result<isize, TypeError> {
        if let Self::NumberV(n) = self {
            if n.is_integer() {
                return Ok(n.to_isize().unwrap());
            }
        }
        Err(TypeError("integer".to_owned()))
    }

    pub fn expect_usize(&self, hint: &str) -> Result<usize, ValueError> {
        if let Self::NumberV(n) = self {
            if n.is_integer() {
                if n.is_sign_positive() {
                    return Ok(n.to_usize().unwrap());
                } else {
                    return Err(ValueError(format!(
                        "{}, expect possitive integer, but negative found",
                        hint
                    )));
                }
            }
        }
        Err(ValueError(format!(
            "{}, expect possitive integer, but {} found",
            hint,
            self.data_type()
        )))
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

pub fn add_preludes(prelude: &mut super::super::prelude::Prelude) {
    // conversion functions
    prelude.add_native_func("string", &["from"], |_, args| -> EvalResult {
        let v = args.get(&"from".to_owned()).unwrap();
        Ok(Value::StrV(v.to_string()))
    });

    prelude.add_native_func("number", &["from"], |_, args| -> EvalResult {
        let v = args.get(&"from".to_owned()).unwrap();
        let n = v.parse_number()?;
        Ok(Value::NumberV(n))
    });

    prelude.add_native_func("not", &["from"], |_, args| -> EvalResult {
        let v = args.get(&"from".to_owned()).unwrap();
        Ok(Value::BoolV(!v.bool_value()))
    });

    // string functions
    prelude.add_native_func("string length", &["string"], |_, args| -> EvalResult {
        let v = args.get(&"string".to_owned()).unwrap();
        let s = v.expect_string("argument[1]")?;
        let lenn = Decimal::from_usize(s.len()).unwrap();
        Ok(Value::NumberV(lenn))
    });

    prelude.add_native_func_with_optional_args(
        "substring",
        &["string", "start position"],
        &["length"],
        |_, args| -> EvalResult {
            let v = args.get(&"string".to_owned()).unwrap();
            let s = v.expect_string("argument[1] `string`")?;
            let start_v = args.get(&"start position".to_owned()).unwrap();
            let start_position = start_v.expect_usize("argument[2] `start position`")?;
            if start_position < 1 || start_position > s.len() {
                return Ok(Value::StrV("".to_owned()));
            }
            // 'length' is the optional value
            let substr = if let Some(lenv) = args.get(&"length".to_owned()) {
                let len = lenv.expect_usize("argument[3] `length`")?;
                &s.as_str()[(start_position - 1)..(cmp::min(start_position - 1 + len, s.len()))]
            } else {
                &s.as_str()[(start_position - 1)..]
            };
            Ok(Value::StrV(substr.to_owned()))
        },
    );

    prelude.add_native_func("upper case", &["string"], |_, args| -> EvalResult {
        let v = args.get(&"string".to_owned()).unwrap();
        let s = v.expect_string("argument[1] `string`")?;
        Ok(Value::StrV(s.to_uppercase()))
    });

    prelude.add_native_func("lower case", &["string"], |_, args| -> EvalResult {
        let v = args.get(&"string".to_owned()).unwrap();
        let s = v.expect_string("argument[1] `string`")?;
        Ok(Value::StrV(s.to_lowercase()))
    });

    prelude.add_native_func("contains", &["string", "match"], |_, args| -> EvalResult {
        let v = args.get(&"string".to_owned()).unwrap();
        let s = v.expect_string("argument[1] `string`")?;
        let mv = args.get(&"match".to_owned()).unwrap();
        let match_s = mv.expect_string("argument[2] `match`")?;
        Ok(Value::BoolV(s.contains(match_s.as_str())))
    });

    prelude.add_native_func(
        "starts with",
        &["string", "match"],
        |_, args| -> EvalResult {
            let v = args.get(&"string".to_owned()).unwrap();
            let s = v.expect_string("argument[1] `string`")?;
            let mv = args.get(&"match".to_owned()).unwrap();
            let match_s = mv.expect_string("argument[2] `match`")?;
            Ok(Value::BoolV(s.starts_with(match_s.as_str())))
        },
    );

    prelude.add_native_func("ends with", &["string", "match"], |_, args| -> EvalResult {
        let v = args.get(&"string".to_owned()).unwrap();
        let s = v.expect_string("argument[1] `string`")?;
        let mv = args.get(&"match".to_owned()).unwrap();
        let match_s = mv.expect_string("argument[2] `match`")?;
        Ok(Value::BoolV(s.ends_with(match_s.as_str())))
    });
}

#[test]
fn test_decimal_trailing_zeros() {
    let a = Decimal::from_str_exact("7").unwrap();
    let b = Decimal::from_str_exact("2").unwrap();
    let d = a / b;
    assert_eq!(d.to_string(), "3.50");
    assert_eq!(d.normalize().to_string(), "3.5");
}
