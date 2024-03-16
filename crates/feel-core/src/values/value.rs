use super::super::ast::Node;
use super::super::helpers::{compare_value, escape, fmt_vec};
use core::cell::Ref;

extern crate chrono;
extern crate iso8601;

use std::cell::RefCell;
use std::cmp;
use std::fmt;
use std::ops;
use std::rc::Rc;

use super::context::{Context, ContextRef};
use super::func::{MacroT, NativeFunc};
use super::numeric::Numeric;
use super::range::RangeT;
use super::temporal::{compare_date, datetime_op, timedelta_to_duration};

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

#[derive(Clone, Ord, PartialOrd, PartialEq, Eq)]
pub enum CompareKey {
    Str(String),
    Number(Numeric),
}

pub type ArrayRef = Rc<RefCell<Vec<Value>>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    NullV,
    BoolV(bool),
    NumberV(Numeric),
    StrV(String),
    DateTimeV(chrono::DateTime<chrono::FixedOffset>),
    DateV(iso8601::Date),
    TimeV(iso8601::Time),
    DurationV {
        duration: iso8601::Duration,
        negative: bool,
    },
    RangeV(RangeT),
    ArrayV(ArrayRef),
    ContextV(ContextRef),
    NativeFuncV {
        func: NativeFunc,
        require_args: Vec<String>,
        optional_args: Vec<String>,
        var_arg: Option<String>,
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
            Self::NumberV(v) => write!(f, "{}", v), // .normalize
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
            Self::ContextV(map) => write!(f, "{}", map.borrow()),
            Self::NativeFuncV {
                require_args: _,
                optional_args: _,
                var_arg: _,
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
    pub fn from_usize(n: usize) -> Value {
        Self::NumberV(Numeric::from_usize(n))
    }

    pub fn from_str(s: &str) -> Value {
        Self::StrV(s.to_owned())
    }

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
            Self::ContextV(_) => "map".to_owned(),
            Self::NativeFuncV {
                require_args: _,
                optional_args: _,
                var_arg: _,
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
            Self::NumberV(v) => *v != Numeric::ZERO,
            Self::StrV(v) => v.len() > 0,
            Self::ArrayV(v) => v.borrow().len() > 0,
            Self::ContextV(v) => v.borrow().len() > 0,
            _ => true,
        }
    }

    pub(crate) fn compare_key(&self) -> CompareKey {
        match self {
            Self::StrV(v) => CompareKey::Str(v.clone()),
            Self::NumberV(v) => CompareKey::Number(v.clone()),
            Self::BoolV(v) if *v == true => CompareKey::Number(Numeric::ONE),
            _ => CompareKey::Number(Numeric::ZERO),
        }
    }

    pub fn parse_number(&self) -> Result<Numeric, ValueError> {
        match self {
            Self::StrV(s) => Numeric::from_str(s),
            Self::NumberV(n) => Ok(n.clone()),
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

    pub fn expect_number(&self, hint: &str) -> Result<Numeric, ValueError> {
        if let Self::NumberV(n) = self {
            return Ok(n.clone());
        }
        Err(ValueError(format!(
            "{}, expect number, but {} found",
            hint,
            self.data_type()
        )))
    }

    pub fn expect_integer(&self, hint: &str) -> Result<isize, ValueError> {
        if let Self::NumberV(n) = self {
            if n.is_integer() {
                return Ok(n.to_isize().unwrap());
            }
        }
        Err(ValueError(format!(
            "{}, expect integer, but {} found",
            hint,
            self.data_type()
        )))
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

    // pub fn expect_array(&self, hint: &str) -> Result<ArrayRef, TypeError> {
    //     if let Self::ArrayV(arr) = self {
    //         return Ok(arr.clone());
    //     }
    //     Err(TypeError(format!(
    //         "{}, expect array, but {} found",
    //         hint,
    //         self.data_type(),
    //     )))
    // }

    pub fn expect_array(&self, hint: &str) -> Result<Ref<'_, Vec<Value>>, TypeError> {
        if let Self::ArrayV(arr) = self {
            return Ok(arr.as_ref().borrow());
        }
        Err(TypeError(format!(
            "{}, expect array, but {} found",
            hint,
            self.data_type(),
        )))
    }

    pub fn expect_array_mut(&self, hint: &str) -> Result<ArrayRef, TypeError> {
        if let Self::ArrayV(arr) = self {
            return Ok(arr.clone());
        }
        Err(TypeError(format!(
            "{}, expect array, but {} found",
            hint,
            self.data_type(),
        )))
    }

    pub fn expect_context(&self, hint: &str) -> Result<Ref<'_, Context>, TypeError> {
        if let Self::ContextV(m) = self {
            return Ok(m.as_ref().borrow());
        }
        Err(TypeError(format!(
            "{}, expect context, but {} found",
            hint,
            self.data_type(),
        )))
    }

    pub fn expect_context_ref(&self, hint: &str) -> Result<ContextRef, TypeError> {
        if let Self::ContextV(m) = self {
            return Ok(m.clone());
        }
        Err(TypeError(format!(
            "{}, expect context, but {} found",
            hint,
            self.data_type(),
        )))
    }

    pub fn expect_range(&self, hint: &str) -> Result<&RangeT, TypeError> {
        if let Self::RangeV(r) = self {
            return Ok(r);
        }
        Err(TypeError(format!(
            "{}, expect range, but {} found",
            hint,
            self.data_type(),
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

impl cmp::Ord for Value {
    fn cmp(&self, other: &Value) -> cmp::Ordering {
        if let Some(ord) = self.partial_cmp(other) {
            ord
        } else {
            self.compare_key().cmp(&other.compare_key())
        }
    }
}

// #[test]
// fn test_decimal_trailing_zeros() {
//     let a = Decimal::from_str_exact("7").unwrap();
//     let b = Decimal::from_str_exact("2").unwrap();
//     let d = a / b;
//     assert_eq!(d.to_string(), "3.50");
//     assert_eq!(d.normalize().to_string(), "3.5");
// }
