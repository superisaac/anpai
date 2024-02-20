use std::collections::BTreeMap;
use std::ops::Neg;

use crate::ast::{MapNodeItem, Node, Node::*};

use crate::value::Value::{self, *};
use rust_decimal::prelude::*;

pub type ValueResult = Result<Value, String>;

#[derive(Clone)]
pub struct Intepreter {}

macro_rules! ev_binop_add {
    ($self:ident, $op:expr, $left_value:expr, $right_value:expr) => {
        match $left_value {
            NumberV(a) => match $right_value {
                NumberV(b) => Ok(NumberV(a + b)),
                _ => Err(format!(
                    "canot {} number and {}",
                    $op,
                    $right_value.data_type()
                )),
            },
            StrV(a) => match $right_value {
                StrV(b) => Ok(StrV(a + &b)),
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
        match $left_value {
            NumberV(numa) => match $right_value {
                NumberV(numb) => Ok(NumberV(numa $numop numb)),
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
        match $left_value {
            NumberV(a) => match $right_value {
                NumberV(b) => Ok(BoolV(a $nativeop b)),
                _ => Err(format!(
                    "canot {} number and {}",
                    $op,
                    $right_value.data_type()
                )),
            },
            StrV(a) => match $right_value {
                StrV(b) => Ok(BoolV(a $nativeop b)),
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
            Null => Ok(NullV),
            Bool(value) => Ok(BoolV(value)),
            Number(value) => self.eval_number(value),
            Str(value) => self.eval_string(value),
            Ident(value) => Ok(StrV(value)),
            Neg(value) => self.eval_neg(value),
            Binop { op, left, right } => self.eval_binop(op, left, right),
            Array(elements) => self.eval_array(&elements),
            Map(items) => self.eval_map(&items),
            _ => Err(format!("eval not supported {}", *node)),
        }
    }

    #[inline(always)]
    fn eval_string(&mut self, value: String) -> ValueResult {
        let content = String::from(&value[1..(value.len() - 1)]);
        Ok(StrV(content))
    }

    #[inline(always)]
    fn eval_number(&mut self, number_str: String) -> ValueResult {
        match Decimal::from_str_exact(number_str.as_str()) {
            Ok(d) => Ok(NumberV(d)),
            Err(err) => return Err(err.to_string()),
        }
    }

    #[inline(always)]
    fn eval_array(&mut self, elements: &Vec<Node>) -> ValueResult {
        let mut results = Vec::new();
        for elem in elements.iter() {
            let res = match self.eval(Box::new(elem.clone())) {
                Ok(v) => v,
                Err(err) => return Err(err),
            };
            results.push(res);
        }
        Ok(ArrayV(results))
    }

    #[inline(always)]
    fn eval_map(&mut self, items: &Vec<MapNodeItem>) -> ValueResult {
        let mut value_map: BTreeMap<String, Value> = BTreeMap::new();
        for item in items.iter() {
            let key = match self.eval(item.name.clone()) {
                Ok(k) => k.to_string(),
                Err(err) => return Err(err),
            };
            let val = match self.eval(item.value.clone()) {
                Ok(v) => v,
                Err(err) => return Err(err),
            };
            value_map.insert(key, val);
        }
        Ok(MapV(value_map))
    }

    #[inline(always)]
    fn eval_neg(&mut self, node: Box<Node>) -> ValueResult {
        let pv = match self.eval(node) {
            Ok(v) => v,
            Err(err) => return Err(err),
        };
        match pv {
            NumberV(v) => Ok(NumberV(v.neg())),
            _ => return Err(format!("cannot neg {}", pv.data_type())),
        }
    }

    // binary ops
    #[inline(always)]
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

#[cfg(test)]
mod test {
    use crate::parse::parse;
    use core::assert_matches::assert_matches;
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
            ("7 / 2", "3.5"), // decimal display outputs normalized string
            ("10 / 3", "3.3333333333333333333333333333"), // precision is up to 28
            ("4 * 9 + 1", "37"),
            (r#""abc" + "def""#, r#""abcdef""#),
            ("2 < 3 - 1", "false"),
            (r#""abc" <= "abd""#, "true"),
            ("[2, 8,false,true]", "[2, 8, false, true]"),
            ("{a: 1, b: 2}", r#"{"a":1, "b":2}"#),
        ];

        for (input, output) in testcases {
            let node = parse(input).unwrap();
            let mut intp = super::Intepreter::new();
            let v = intp.eval(node).unwrap();
            assert_eq!(v.to_string(), output);
        }
    }
}
