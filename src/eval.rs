use std::backtrace;
use std::collections::btree_set::Intersection;

use crate::ast::{Node, Node::*};
use crate::value::{Value, Value::*, DECIMAL_PLACES};
// use std::fmt::format;
use rust_decimal_macros::dec;
use rust_decimal::prelude::*;
use crate::parse::parse;

pub type ValueResult = Result<Box<Value>, String>;

fn int_to_dec(iv: i64) -> Decimal {
    Decimal::from_str_exact(iv.to_string().as_str()).unwrap()
}

fn string_to_dec(sv: String) -> Decimal {
    Decimal::from_str_exact(sv.as_str()).unwrap()
}

#[derive(Clone)]
pub struct Intepreter {

}

impl Intepreter {
    pub fn new() -> Intepreter {
        Intepreter{}
    }

    pub fn eval(&mut self, node: Box<Node>) -> ValueResult {
        match *node {
            Number { value } => Ok(Box::new(NumberV(string_to_dec(value)))),
            Binop { op, left, right } => self.eval_binop(op, left, right),
            _ => Err(format!("eval not supported {}", *node)),
        }
    }

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
            "+" => self.eval_binop_add(left_value, right_value),
            "-" => self.eval_binop_sub(left_value, right_value),
            _ => return Err(format!("unknown op {}", op))
        }
    }

    fn eval_binop_add(&mut self, left_value: Box<Value>, right_value: Box<Value>) -> ValueResult {
        match *left_value {
            IntV(a) => {
                match *right_value {
                    IntV(b ) => Ok(Box::new(IntV(a + b))),
                    NumberV(b) => {
                        let dec_a = int_to_dec(a);
                        Ok(Box::new(NumberV(dec_a + b)))
                    }
                    _ => Err(format!("canot add int and {}", right_value.data_type())),
                }
            }
            NumberV(a) => {
                match *right_value {
                    IntV(b) => {
                        let dec_b = int_to_dec(b);
                        Ok(Box::new(NumberV(a + dec_b)))
                    }
                    NumberV(b) => {
                        Ok(Box::new(NumberV(a + b)))
                    }
                    _ => Err(format!("canot add number and {}", right_value.data_type())),
                }
            }
            StrV(a) => {
                if let StrV(b) = *right_value {
                    Ok(Box::new(StrV(a + &b)))
                } else {
                    Err(format!("canot add string and {}", right_value.data_type()))
                }
            },
            _ => Err(format!("canot add {} and {}", left_value.data_type(), right_value.data_type())),
        }
    }

    fn eval_binop_sub(&mut self, left_value: Box<Value>, right_value: Box<Value>) -> ValueResult {
        match *left_value {
            IntV(a) => {
                match *right_value {
                    IntV(b ) => Ok(Box::new(IntV(a - b))),
                    NumberV(b) => {
                        let dec_a = int_to_dec(a);
                        Ok(Box::new(NumberV(dec_a - b)))
                    }
                    _ => Err(format!("canot sub int and {}", right_value.data_type())),
                }
            }
            NumberV(a) => {
                match *right_value {
                    IntV(b ) => Ok(Box::new(NumberV(a - int_to_dec(b)))),
                    NumberV(b) => {
                        Ok(Box::new(NumberV(a - b)))
                    }
                    _ => Err(format!("canot sub int and {}", right_value.data_type())),
                }
            }
            _ => Err(format!("canot sub {} and {}", left_value.data_type(), right_value.data_type())),
        }
    }
}

#[test]
fn test_parse_and_eval() {
    let testcases = [
        ("2+ 4", "6"),
        ("2-5", "-3"),
    ];

    for (input, output) in testcases {
        let node = parse(input).unwrap();
        let mut intp = Intepreter::new();
        let v = intp.eval(node).unwrap();
        assert_eq!(v.to_string(), output);
    }    
}