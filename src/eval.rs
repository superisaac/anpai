use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::error;
use std::fmt;
use std::ops::Neg;

use crate::ast::{MapNodeItem, Node, Node::*};

use crate::value::Value::{self, *};
//use rust_decimal::prelude::*;
use rust_decimal::{Decimal, Error as DecimalError};

// EvalError
#[derive(Debug)]
pub enum EvalError {
    VarNotFound,
    Op(String),
    Decimal(DecimalError),
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::VarNotFound => write!(f, "{}", "VarNotFound"),
            Self::Op(message) => write!(f, "OpError: {}", message),
            Self::Decimal(err) => write!(f, "DecimalError: {}", err),
        }
    }
}

impl error::Error for EvalError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Decimal(err) => Some(err),
            _ => None,
        }
    }
}

impl From<DecimalError> for EvalError {
    fn from(err: DecimalError) -> EvalError {
        Self::Decimal(err)
    }
}

pub type ValueResult = Result<Value, EvalError>;

pub struct ScopeFrame {
    vars: HashMap<String, Value>,
}

pub struct Intepreter {
    scopes: Vec<RefCell<ScopeFrame>>,
}

macro_rules! ev_binop_add {
    ($self:ident, $op:expr, $left_value:expr, $right_value:expr) => {
        match $left_value {
            NumberV(a) => match $right_value {
                NumberV(b) => Ok(NumberV(a + b)),
                _ => Err(EvalError::Op(format!(
                    "canot {} number and {}",
                    $op,
                    $right_value.data_type()
                ))),
            },
            StrV(a) => match $right_value {
                StrV(b) => Ok(StrV(a + &b)),
                _ => Err(EvalError::Op(format!(
                    "canot {} string and {}",
                    $op,
                    $right_value.data_type()
                ))),
            },
            _ => Err(EvalError::Op(format!(
                "canot {} {} and {}",
                $op,
                $left_value.data_type(),
                $right_value.data_type()
            ))),
        }
    };
}

macro_rules! ev_binop_number {
    ($self:ident, $op:expr, $left_value:expr, $right_value:expr, $numop:tt) => {
        match $left_value {
            NumberV(numa) => match $right_value {
                NumberV(numb) => Ok(NumberV(numa $numop numb)),
                _ => Err(EvalError::Op(format!(
                    "canot {} number and {}",
                    $op,
                    $right_value.data_type()
                ))),
            },
            _ => Err(EvalError::Op(format!(
                "canot {} {} and {}",
                $op,
                $left_value.data_type(),
                $right_value.data_type()
            ))),
        }
    };
}

macro_rules! ev_binop_comparation {
    ($self:ident, $op:expr, $left_value:expr, $right_value:expr, $nativeop:tt) => {
        match $left_value {
            NumberV(a) => match $right_value {
                NumberV(b) => Ok(BoolV(a $nativeop b)),
                _ => Err(EvalError::Op(format!(
                    "canot {} number and {}",
                    $op,
                    $right_value.data_type()
                ))),
            },
            StrV(a) => match $right_value {
                StrV(b) => Ok(BoolV(a $nativeop b)),
                _ => Err(EvalError::Op(format!(
                    "canot {} string and {}",
                    $op,
                    $right_value.data_type()
                ))),
            },
            _ => Err(EvalError::Op(format!(
                "canot {} {} and {}",
                $op,
                $left_value.data_type(),
                $right_value.data_type()
            ))),
        }
    };
}

impl Intepreter {
    pub fn new() -> Intepreter {
        Intepreter { scopes: Vec::new() }
    }

    fn add_frame(&mut self) {
        let frame = ScopeFrame {
            vars: HashMap::new(),
        };
        self.scopes.push(RefCell::new(frame));
    }

    fn resolve(&self, name: String) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.borrow().vars.get(&name) {
                return Some(v.clone());
            }
        }
        None
    }

    fn set_var(&mut self, name: String, value: Value) {
        if self.scopes.len() == 0 {
            self.add_frame();
        }
        self.scopes
            .last()
            .unwrap()
            .borrow_mut()
            .vars
            .insert(name, value);
    }

    fn set_var_at(&mut self, name: String, value: Value, index: usize) {
        if let Some(frame) = self.scopes.get_mut(index) {
            frame.borrow_mut().vars.insert(name, value);
        }
    }

    pub fn eval(&mut self, node: Box<Node>) -> ValueResult {
        match *node {
            Null => Ok(NullV),
            Bool(value) => Ok(BoolV(value)),
            Number(value) => self.eval_number(value),
            Str(value) => self.eval_string(value),
            Ident(value) => Ok(StrV(value)),
            Var(name) => self.eval_var(name),
            Neg(value) => self.eval_neg(value),
            Binop { op, left, right } => self.eval_binop(op, left, right),
            Array(elements) => self.eval_array(&elements),
            Map(items) => self.eval_map(&items),
            _ => Err(EvalError::Op(format!("eval not supported {}", *node))),
        }
    }

    #[inline(always)]
    fn eval_string(&mut self, value: String) -> ValueResult {
        let content = String::from(&value[1..(value.len() - 1)]);
        Ok(StrV(content))
    }

    #[inline(always)]
    fn eval_number(&mut self, number_str: String) -> ValueResult {
        let d = Decimal::from_str_exact(number_str.as_str())?;
        Ok(NumberV(d))
    }

    #[inline(always)]
    fn eval_var(&mut self, name: String) -> ValueResult {
        if let Some(value) = self.resolve(name) {
            Ok(value)
        } else {
            Err(EvalError::VarNotFound)
        }
    }

    #[inline(always)]
    fn eval_array(&mut self, elements: &Vec<Box<Node>>) -> ValueResult {
        let mut results = Vec::new();
        for elem in elements.iter() {
            let res = self.eval(elem.clone())?;
            results.push(res);
        }
        Ok(ArrayV(RefCell::new(results)))
    }

    #[inline(always)]
    fn eval_map(&mut self, items: &Vec<MapNodeItem>) -> ValueResult {
        let mut value_map: BTreeMap<String, Value> = BTreeMap::new();
        for item in items.iter() {
            let k = self.eval(item.name.clone())?;
            let key = k.to_string();
            let val = self.eval(item.value.clone())?;
            value_map.insert(key, val);
        }
        Ok(MapV(RefCell::new(value_map)))
    }

    #[inline(always)]
    fn eval_neg(&mut self, node: Box<Node>) -> ValueResult {
        let pv = self.eval(node)?;
        match pv {
            NumberV(v) => Ok(NumberV(v.neg())),
            _ => return Err(EvalError::Op(format!("cannot neg {}", pv.data_type()))),
        }
    }

    // binary ops
    #[inline(always)]
    fn eval_binop(&mut self, op: String, left: Box<Node>, right: Box<Node>) -> ValueResult {
        let left_value = self.eval(left)?;
        let right_value = self.eval(right)?;
        match op.as_str() {
            "+" => ev_binop_add!(self, op, left_value, right_value),
            "-" => ev_binop_number!(self, op, left_value, right_value, -),
            "*" => ev_binop_number!(self, op, left_value, right_value, *),
            "/" => ev_binop_number!(self,op, left_value, right_value, /),
            ">" => ev_binop_comparation!(self, op, left_value, right_value, >),
            ">=" => ev_binop_comparation!(self, op, left_value, right_value, >=),
            "<" => ev_binop_comparation!(self, op, left_value, right_value, <),
            "<=" => ev_binop_comparation!(self, op, left_value, right_value, <=),
            "!=" => ev_binop_comparation!(self, op, left_value, right_value, !=),
            "=" => ev_binop_comparation!(self, op, left_value, right_value, ==),
            _ => return Err(EvalError::Op(format!("unknown op {}", op))),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::parse::parse;
    use core::assert_matches::assert_matches;
    use rust_decimal_macros::dec;

    #[test]
    fn test_number_parse() {
        let a = "2342404820143892034890".parse::<i64>();
        assert_matches!(a, Err(_));
    }

    #[test]
    fn test_parse_stateless() {
        let testcases = [
            ("2+ 4", "6"),
            ("2 -5", "-3"),
            ("8 - 2", "6"),
            ("7 / 2", "3.5"), // decimal display outputs normalized string
            ("10 / 3", "3.3333333333333333333333333333"), // precision is up to 28
            ("4 * 9 + 1", "37"),
            (r#""abc" + "def""#, r#""abcdef""#),
            ("2 < 3 - 1", "false"),
            (r#""abc" <= "abd""#, "true"),
            ("[2, 8,false,true]", "[2, 8, false, true]"),
            ("{a: 1, b: 2}", r#"{"a":1, "b":2}"#),
        ];

        let mut intp = super::Intepreter::new();
        for (input, output) in testcases {
            let node = parse(input).unwrap();
            let v = intp.eval(node).unwrap();
            assert_eq!(v.to_string(), output);
        }
    }

    #[test]
    fn test_def_vars() {
        let mut intp = super::Intepreter::new();
        intp.set_var("v1".to_owned(), super::NumberV(dec!(2.3)));
        let input = "v1 + 3";
        let node = parse(input).unwrap();
        let v = intp.eval(node).unwrap();
        assert_eq!(v.to_string(), "5.3");
    }
}
