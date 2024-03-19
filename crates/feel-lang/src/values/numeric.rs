use super::value::Value;
use bigdecimal::*;
use num_bigint::Sign;
use std::cmp;
use std::fmt;
use std::ops;
use std::str::FromStr;

#[derive(Clone)]
pub enum Numeric {
    Integer(i32),
    Decimal(BigDecimal),
}

impl fmt::Display for Numeric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Integer(v) => write!(f, "{}", v),
            Self::Decimal(v) => write!(f, "{}", v),
        }
    }
}

impl fmt::Debug for Numeric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Integer(v) => write!(f, "<Int {}>", v),
            Self::Decimal(v) => write!(f, "<Decimal {}>", v),
        }
    }
}

impl Numeric {
    pub const ZERO: Self = Self::Integer(0);
    pub const ONE: Self = Self::Integer(1);
    pub const TWO: Self = Self::Integer(2);

    fn max_integer() -> BigDecimal {
        BigDecimal::from(i32::MAX)
    }

    fn min_integer() -> BigDecimal {
        BigDecimal::from(i32::MIN)
    }

    pub fn from_str(input: &str) -> Option<Numeric> {
        let bign = match BigDecimal::from_str(input) {
            Ok(v) => v,
            Err(_) => return None,
        };
        Some(Self::from_decimal(bign))
    }

    pub fn from_decimal(bign: BigDecimal) -> Numeric {
        if bign.is_integer() && bign < Self::max_integer() && bign > Self::min_integer() {
            if let Some(v) = bign.to_i32() {
                return Self::Integer(v);
            }
        }

        Self::Decimal(bign)
    }

    pub fn from_value(value: &Value) -> Option<Numeric> {
        match value {
            Value::NumberV(v) => Some(v.clone()),
            Value::StrV(v) => Self::from_str(v.as_str()),
            _ => None,
        }
    }

    pub fn from_usize(v: usize) -> Numeric {
        if v >= (i32::MAX as usize) {
            Self::Decimal(BigDecimal::from_usize(v).unwrap())
        } else {
            Self::Integer(v as i32)
        }
    }

    pub fn from_i32(v: i32) -> Numeric {
        Self::Integer(v)
    }

    pub fn from_f64(v: f64) -> Numeric {
        Self::Decimal(BigDecimal::from_f64(v).unwrap())
    }

    pub fn to_decimal(&self) -> BigDecimal {
        match self {
            Self::Integer(v) => BigDecimal::from_i32(*v).unwrap(),
            Self::Decimal(v) => v.clone(),
        }
    }

    pub fn is_integer(&self) -> bool {
        match self {
            Self::Integer(_) => true,
            Self::Decimal(v) => v.is_integer(),
        }
    }

    pub fn is_sign_positive(&self) -> bool {
        match self {
            Self::Integer(v) => *v >= 0,
            Self::Decimal(v) => v.is_positive(),
        }
    }

    pub fn floor(&self) -> Numeric {
        self.with_scale_down(0)
    }

    pub fn with_scale_down(&self, scale: i64) -> Numeric {
        let v = self.to_decimal();
        if v.sign() == Sign::Minus {
            Self::from_decimal(v.with_scale_round(scale, RoundingMode::Up))
        } else {
            Self::from_decimal(v.with_scale_round(scale, RoundingMode::Down))
        }
    }

    pub fn with_scale_up(&self, scale: i64) -> Numeric {
        let v = self.to_decimal();
        if v.sign() == Sign::Minus {
            Self::from_decimal(v.with_scale_round(scale, RoundingMode::Down))
        } else {
            Self::from_decimal(v.with_scale_round(scale, RoundingMode::Up))
        }
    }

    pub fn with_scale_even(&self, scale: i64) -> Numeric {
        let v = self.to_decimal();
        Self::from_decimal(v.with_scale_round(scale, RoundingMode::HalfEven))
    }

    pub fn to_usize(&self) -> Option<usize> {
        match self {
            Self::Integer(v) => {
                if *v > 0 {
                    Some(*v as usize)
                } else {
                    None
                }
            }
            Self::Decimal(v) => v.to_usize(),
        }
    }

    pub fn to_isize(&self) -> Option<isize> {
        match self {
            Self::Integer(v) => Some(*v as isize),
            Self::Decimal(v) => v.to_isize(),
        }
    }

    pub fn sqrt(&self) -> Option<Numeric> {
        self.to_decimal().sqrt().map(|v| Self::from_decimal(v))
    }
}

macro_rules! complex_op {
    ($one:ident, $another:ident, $op:tt) => {
        match $one {
            Self::Integer(a) => match $another {
                Self::Integer(b) => {
                    let r = (a as i64) $op (b as i64);
                    if r > (i32::MAX as i64) || r < (i32::MIN as i64) {
                        return Self::Decimal(BigDecimal::from_i64(r).unwrap());
                    }
                    Self::Integer(r as i32)
                },
                Self::Decimal(b) => {
                    let r = BigDecimal::from_i32(a).unwrap() $op b;
                    Self::Decimal(r)
                }
            },
            Self::Decimal(a) => match $another {
                Self::Integer(b) => {
                    let r = a $op BigDecimal::from_i32(b).unwrap();
                    Self::Decimal(r)
                },
                Self::Decimal(b) => {
                    Self::Decimal(a $op b)
                }
            }
        }
    };
}

macro_rules! complex_op_assign {
    ($one:ident, $another:ident, $op:tt) => {
        match $one {
            Self::Integer(ref a) => match $another {
                Self::Integer(b) => {
                    let r = (*a as i64) $op (b as i64);
                    if r > (i32::MAX as i64) || r < (i32::MIN as i64) {
                        Self::Decimal(BigDecimal::from_i64(r).unwrap())
                    } else {
                        Self::Integer(r as i32)
                    }
                },
                Self::Decimal(b) => {
                    let r = BigDecimal::from_i32(*a).unwrap() $op b;
                    Self::Decimal(r)
                }
            },
            Self::Decimal(a) => match $another {
                Self::Integer(b) => {
                    let r = a.clone() $op BigDecimal::from_i32(b).unwrap();
                    Self::Decimal(r)
                },
                Self::Decimal(b) => {
                    Self::Decimal(a.clone() $op b)
                }
            }
        }
    };
}

impl ops::Add for Numeric {
    type Output = Numeric;

    #[inline(always)]
    fn add(self, other: Self) -> Self::Output {
        complex_op!(self, other, +)
    }
}

impl ops::AddAssign for Numeric {
    fn add_assign(&mut self, other: Self) {
        *self = complex_op_assign!(self, other, +);
        ()
    }
}

impl ops::Sub for Numeric {
    type Output = Numeric;

    #[inline(always)]
    fn sub(self, other: Self) -> Self::Output {
        complex_op!(self, other, -)
    }
}

impl ops::SubAssign for Numeric {
    fn sub_assign(&mut self, other: Self) {
        *self = complex_op_assign!(self, other, -);
        ()
    }
}

impl ops::Mul for Numeric {
    type Output = Numeric;

    #[inline(always)]
    fn mul(self, other: Self) -> Self::Output {
        complex_op!(self, other, *)
    }
}

impl ops::MulAssign for Numeric {
    fn mul_assign(&mut self, other: Self) {
        *self = complex_op_assign!(self, other, *);
        ()
    }
}

impl ops::Div for Numeric {
    type Output = Numeric;

    #[inline(always)]
    fn div(self, other: Self) -> Self::Output {
        Self::Decimal(self.to_decimal() / other.to_decimal())
    }
}

impl ops::DivAssign for Numeric {
    fn div_assign(&mut self, other: Self) {
        *self = Self::Decimal(self.to_decimal() / other.to_decimal());
        ()
    }
}

impl ops::Rem for Numeric {
    type Output = Numeric;

    #[inline(always)]
    fn rem(self, other: Self) -> Self::Output {
        complex_op!(self, other, %)
    }
}

impl ops::RemAssign for Numeric {
    fn rem_assign(&mut self, other: Self) {
        *self = complex_op_assign!(self, other, %);
        ()
    }
}

impl ops::Neg for Numeric {
    type Output = Numeric;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        match self {
            Self::Integer(a) => Self::Integer(a.neg()),
            Self::Decimal(a) => Self::Decimal(a.neg()),
        }
    }
}

impl cmp::PartialEq for Numeric {
    fn eq(&self, other: &Numeric) -> bool {
        if let Self::Integer(a) = *self {
            if let Self::Integer(b) = *other {
                return a == b;
            }
        }
        self.to_decimal() == other.to_decimal()
    }
}

impl cmp::Eq for Numeric {}

impl cmp::PartialOrd for Numeric {
    fn partial_cmp(&self, other: &Numeric) -> Option<cmp::Ordering> {
        if let Self::Integer(a) = *self {
            if let Self::Integer(b) = *other {
                return a.partial_cmp(&b);
            }
        }
        self.to_decimal().partial_cmp(&other.to_decimal())
    }
}

impl cmp::Ord for Numeric {
    fn cmp(&self, other: &Numeric) -> cmp::Ordering {
        if let Self::Integer(a) = *self {
            if let Self::Integer(b) = *other {
                return a.cmp(&b);
            }
        }
        self.to_decimal().cmp(&other.to_decimal())
    }
}
