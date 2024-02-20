use core::assert_matches::assert_matches;
use std::ops::Neg;

use crate::ast::{Node, Node::*};
use crate::parse::parse;
use crate::value::Value::{self, *};
use rust_decimal::prelude::*;

pub type ValueResult = Result<Box<Value>, String>;

#[derive(Clone)]
pub struct Intepreter {}

macro_rules! ev_binop_add {
    ($self:ident, $op:expr, $left_value:expr, $right_value:expr) => {
        match *$left_value {
            NumberV(a) => match *$right_value {
                NumberV(b) => Ok(Box::new(NumberV(a + b))),
                _ => Err(format!(
                    "canot {} number and {}",
                    $op,
                    $right_value.data_type()
                )),
            },
            StrV(a) => match *$right_value {
                StrV(b) => Ok(Box::new(StrV(a + &b))),
                _ => Err(format!(
                    "canot {} string and {}",
                    $op,
                    $right_value.data_type()
                )),
            },
            _ => Err(format!(
                "canot {} {} and {}",
                $op,
                $left_value.data_type(),
                $right_value.data_type()
            )),
        }
    };
}

macro_rules! ev_binop_number {
    ($self:ident, $op:expr, $left_value:expr, $right_value:expr, $numop:tt) => {
        match *$left_value {
            NumberV(numa) => match *$right_value {
                NumberV(numb) => Ok(Box::new(NumberV(numa $numop numb))),
                _ => Err(format!(
                    "canot {} number and {}",
                    $op,
                    $right_value.data_type()
                )),
            },
            _ => Err(format!(
                "canot {} {} and {}",
                $op,
                $left_value.data_type(),
                $right_value.data_type()
            )),
        }
    };
}

macro_rules! ev_binop_comparation {
    ($self:ident, $op:expr, $left_value:expr, $right_value:expr, $nativeop:tt) => {
        match *$left_value {
            NumberV(a) => match *$right_value {
                NumberV(b) => Ok(Box::new(BoolV(a $nativeop b))),
                _ => Err(format!(
                    "canot {} number and {}",
                    $op,
                    $right_value.data_type()
                )),
            },
            StrV(a) => match *$right_value {
                StrV(b) => Ok(Box::new(BoolV(a $nativeop b))),
                _ => Err(format!(
                    "canot {} string and {}",
                    $op,
                    $right_value.data_type()
                )),
            },
            _ => Err(format!(
                "canot {} {} and {}",
                $op,
                $left_value.data_type(),
                $right_value.data_type()
            )),
        }
    };
}

impl Intepreter {
    pub fn new() -> Intepreter {
        Intepreter {}
    }

    pub fn eval(&mut self, node: Box<Node>) -> ValueResult {
        match *node {
            Null => Ok(Box::new(NullV)),
            Bool(value) => Ok(Box::new(BoolV(value))),
            Number(value) => self.eval_number(value),
            Str(value) => self.eval_string(value),
            Neg(value) => self.eval_neg(value),
            Binop { op, left, right } => self.eval_binop(op, left, right),
            _ => Err(format!("eval not supported {}", *node)),
        }
    }

    fn eval_string(&mut self, value: String) -> ValueResult {
        let content = String::from(&value[1..(value.len() - 1)]);
        Ok(Box::new(StrV(content)))
    }

    fn eval_number(&mut self, number_str: String) -> ValueResult {
        match Decimal::from_str_exact(number_str.as_str()) {
            Ok(d) => Ok(Box::new(NumberV(d))),
            Err(err) => return Err(err.to_string()),
        }
    }
    fn eval_neg(&mut self, node: Box<Node>) -> ValueResult {
        let pv = match self.eval(node) {
            Ok(v) => v,
            Err(err) => return Err(err),
        };
        match *pv {
            NumberV(v) => Ok(Box::new(NumberV(v.neg()))),
            _ => return Err(format!("cannot neg {}", pv.data_type())),
        }
    }

    // binary ops
    fn eval_binop(&mut self, op: String, left: Box<Node>, right: Box<Node>) -> ValueResult {
        let left_value = match self.eval(left) {
            Ok(v) => v,
            Err(err) => return Err(err),
        };
        let right_value = match self.eval(right) {
            Ok(v) => v,
            Err(err) => return Err(err),
        };
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
            _ => return Err(format!("unknown op {}", op)),
        }
    }
}

#[test]
fn test_number_parse() {
    let a = "2342404820143892034890".parse::<i64>();
    assert_matches!(a, Err(_));
}

#[test]
fn test_parse_and_eval() {
    let testcases = [
        ("2+ 4", "6"),
        ("2 -5", "-3"),
        ("8 - 2", "6"),
        ("7 / 2", "3.50"),
        ("4 * 9 + 1", "37"),
        (r#""abc" + "def""#, r#""abcdef""#),
        ("2 < 3 - 1", "false"),
        (r#""abc" <= "abd""#, "true"),
    ];

    for (input, output) in testcases {
        let node = parse(input).unwrap();
        let mut intp = Intepreter::new();
        let v = intp.eval(node).unwrap();
        assert_eq!(v.to_string(), output);
    }
}
