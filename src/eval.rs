use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::error;
use std::fmt;

use std::rc::Rc;

use crate::ast::{FuncCallArg, MapNodeItem, Node, NodeSyntax::*};
use crate::parse::ParseError;
use crate::prelude::PRELUDE;
use crate::temporal::parse_temporal;
use crate::value::{NativeFunc, ValueError};

use crate::value::MacroCbT;
use crate::value::RangeT;
use crate::value::Value::{self, *};

use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::{Decimal, Error as DecimalError};
use rust_decimal_macros::dec;

// EvalError
#[derive(Debug)]
pub enum EvalError {
    VarNotFound,
    KeyError,
    IndexError,
    Runtime(String),
    Decimal(DecimalError),
    Parse(ParseError),
    Value(String),
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::VarNotFound => write!(f, "{}", "VarNotFound"),
            Self::KeyError => write!(f, "{}", "KeyError"),
            Self::IndexError => write!(f, "{}", "IndexError"),
            Self::Runtime(message) => write!(f, "RuntimeError: {}", message),
            Self::Value(message) => write!(f, "ValueError: {}", message),
            Self::Decimal(err) => write!(f, "DecimalError: {}", err),
            Self::Parse(err) => write!(f, "{}", err),
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

impl From<String> for EvalError {
    fn from(err: String) -> EvalError {
        Self::Runtime(err)
    }
}

impl From<ParseError> for EvalError {
    fn from(err: ParseError) -> EvalError {
        Self::Parse(err)
    }
}

impl From<ValueError> for EvalError {
    fn from(err: ValueError) -> EvalError {
        Self::Value(err.0)
    }
}

impl EvalError {
    pub fn runtime(message: &str) -> EvalError {
        Self::Runtime(String::from(message))
    }
}

pub type EvalResult = Result<Value, EvalError>;

pub struct ScopeFrame {
    vars: HashMap<String, Value>,
}

pub struct Intepreter {
    scopes: Vec<RefCell<ScopeFrame>>,
}

impl Intepreter {
    pub fn new() -> Intepreter {
        let mut intp = Intepreter { scopes: Vec::new() };
        intp.push_frame(); // prelude frame
        intp
    }

    fn push_frame(&mut self) {
        let frame = ScopeFrame {
            vars: HashMap::new(),
        };
        self.scopes.push(RefCell::new(frame));
    }

    fn pop_frame(&mut self) {
        self.scopes.pop();
    }

    pub fn resolve(&self, name: String) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.borrow().vars.get(&name) {
                return Some(v.clone());
            }
        }
        PRELUDE.resolve(name)
    }

    pub fn set_var(&mut self, name: String, value: Value) {
        if self.scopes.len() == 0 {
            self.push_frame();
        }
        self.scopes
            .last()
            .unwrap()
            .borrow_mut()
            .vars
            .insert(name, value);
    }

    pub fn eval(&mut self, node: Box<Node>) -> EvalResult {
        match *node.syntax {
            Null => Ok(NullV),
            Bool(value) => Ok(BoolV(value)),
            Number(value) => self.eval_number(value),
            Str(value) => self.eval_string(value),
            Temporal(value) => Ok(parse_temporal(value.as_str())?),
            Ident(value) => Ok(StrV(value)),
            Var(name) => self.eval_var(name),
            Neg(value) => self.eval_neg_op(value),
            Not(value) => self.eval_not_op(value),
            BinOp { op, left, right } => self.eval_binop(op, left, right),
            LogicOp { op, left, right } => self.eval_logicop(op, left, right),
            DotOp { left, attr } => self.eval_dotop(left, attr),
            Range {
                start_open,
                start,
                end_open,
                end,
            } => self.eval_range(start_open, start, end, end_open),
            Array(elements) => self.eval_array(&elements),
            Map(items) => self.eval_map(&items),
            FuncDef { arg_names, body } => Ok(FuncV {
                func_def: Node::new(FuncDef { arg_names, body }),
            }),
            FuncCall { func_ref, args } => self.eval_func_call(func_ref, args),
            IfExpr {
                condition,
                then_branch,
                else_branch,
            } => self.eval_if_expr(condition, then_branch, else_branch),
            ForExpr {
                var_name,
                list_expr,
                return_expr,
            } => self.eval_for_expr(var_name, list_expr, return_expr),
            SomeExpr {
                var_name,
                list_expr,
                filter_expr,
            } => self.eval_some_expr(var_name, list_expr, filter_expr),
            EveryExpr {
                var_name,
                list_expr,
                filter_expr,
            } => self.eval_every_expr(var_name, list_expr, filter_expr),
            ExprList(exprs) => self.eval_expr_list(exprs),
            MultiTests(exprs) => self.eval_multi_tests(exprs),
        }
    }

    #[inline(always)]
    fn eval_string(&mut self, value: String) -> EvalResult {
        let content = String::from(&value[1..(value.len() - 1)]);
        Ok(StrV(content))
    }

    pub fn is_defined(&mut self, value_node: Box<Node>) -> EvalResult {
        if let Var(v) = *value_node.syntax.clone() {
            return match self.resolve(v) {
                Some(_) => Ok(BoolV(true)),
                None => Ok(BoolV(false)),
            };
        }
        match self.eval(value_node.clone()) {
            Ok(_) => Ok(BoolV(true)),
            Err(EvalError::IndexError) | Err(EvalError::KeyError) | Err(EvalError::VarNotFound) => {
                Ok(BoolV(false))
            }
            Err(err) => Err(err),
        }
    }

    #[inline(always)]
    fn eval_number(&mut self, number_str: String) -> EvalResult {
        let d = Decimal::from_str_exact(number_str.as_str())?;
        Ok(NumberV(d))
    }

    #[inline(always)]
    fn eval_var(&mut self, name: String) -> EvalResult {
        if let Some(value) = self.resolve(name) {
            Ok(value)
        } else {
            Err(EvalError::VarNotFound)
        }
    }

    #[inline(always)]
    fn eval_array(&mut self, elements: &Vec<Box<Node>>) -> EvalResult {
        let mut results = Vec::new();
        for elem in elements.iter() {
            let res = self.eval(elem.clone())?;
            results.push(res);
        }
        Ok(ArrayV(RefCell::new(Rc::new(results))))
    }

    #[inline(always)]
    fn eval_map(&mut self, items: &Vec<MapNodeItem>) -> EvalResult {
        let mut value_map: BTreeMap<String, Value> = BTreeMap::new();
        for item in items.iter() {
            let k = self.eval(item.name.clone())?;
            let key = k.to_string();
            let val = self.eval(item.value.clone())?;
            value_map.insert(key, val);
        }
        Ok(MapV(RefCell::new(Rc::new(value_map))))
    }

    #[inline(always)]
    fn eval_neg_op(&mut self, node: Box<Node>) -> EvalResult {
        let pv = self.eval(node)?;
        Ok((-pv)?)
    }

    #[inline(always)]
    fn eval_not_op(&mut self, node: Box<Node>) -> EvalResult {
        let pv = self.eval(node)?;
        Ok(!pv)
    }

    #[inline(always)]
    fn eval_if_expr(
        &mut self,
        condition: Box<Node>,
        then_branch: Box<Node>,
        else_branch: Box<Node>,
    ) -> EvalResult {
        let cond_value = self.eval(condition)?;
        if cond_value.bool_value() {
            self.eval(then_branch)
        } else {
            self.eval(else_branch)
        }
    }

    fn eval_range(
        &mut self,
        start_open: bool,
        start_node: Box<Node>,
        end_node: Box<Node>,
        end_open: bool,
    ) -> EvalResult {
        let start_value = self.eval(start_node)?;
        let end_value = self.eval(end_node)?;
        Ok(RangeV(RangeT {
            start_open,
            start: Rc::new(start_value),
            end: Rc::new(end_value),
            end_open,
        }))
    }

    fn eval_for_expr(
        &mut self,
        var_name: String,
        list_expr: Box<Node>,
        return_expr: Box<Node>,
    ) -> EvalResult {
        let list_value = self.eval(list_expr)?;
        match list_value {
            ArrayV(items) => {
                let mut results: Vec<Value> = vec![];
                for item in items.borrow().iter() {
                    self.push_frame();
                    self.set_var(var_name.clone(), item.clone());
                    let result = self.eval(return_expr.clone());
                    self.pop_frame();
                    match result {
                        Ok(v) => results.push(v),
                        Err(err) => return Err(err),
                    }
                }
                Ok(ArrayV(RefCell::new(Rc::new(results))))
            }
            _ => Err(EvalError::runtime("for loop require a list")),
        }
    }

    fn eval_some_expr(
        &mut self,
        var_name: String,
        list_expr: Box<Node>,
        filter_expr: Box<Node>,
    ) -> EvalResult {
        let list_value = self.eval(list_expr)?;
        match list_value {
            ArrayV(items) => {
                for item in items.borrow().iter() {
                    self.push_frame();
                    self.set_var(var_name.clone(), item.clone());
                    let result = self.eval(filter_expr.clone());
                    self.pop_frame();
                    match result {
                        Ok(v) => {
                            if v.bool_value() {
                                return Ok(item.clone());
                            }
                        }
                        Err(err) => return Err(err),
                    }
                }
                Ok(NullV)
            }
            _ => Err(EvalError::runtime("for loop require a list")),
        }
    }

    fn eval_every_expr(
        &mut self,
        var_name: String,
        list_expr: Box<Node>,
        filter_expr: Box<Node>,
    ) -> EvalResult {
        let list_value = self.eval(list_expr)?;
        match list_value {
            ArrayV(items) => {
                let mut results: Vec<Value> = vec![];
                for item in items.borrow().iter() {
                    self.push_frame();
                    self.set_var(var_name.clone(), item.clone());
                    let result = self.eval(filter_expr.clone());
                    self.pop_frame();
                    match result {
                        Ok(v) => {
                            if v.bool_value() {
                                results.push(item.clone());
                            }
                        }
                        Err(err) => return Err(err),
                    }
                }
                Ok(ArrayV(RefCell::new(Rc::new(results))))
            }
            _ => Err(EvalError::runtime("for loop require a list")),
        }
    }

    #[inline(always)]
    fn eval_expr_list(&mut self, exprs: Vec<Node>) -> EvalResult {
        let mut last_result: Option<Value> = None;
        for expr in exprs.iter() {
            let res = self.eval(Box::new(expr.clone()))?;
            last_result = Some(res);
        }
        if let Some(v) = last_result {
            Ok(v)
        } else {
            Ok(NullV)
        }
    }

    #[inline(always)]
    fn eval_multi_tests(&mut self, exprs: Vec<Node>) -> EvalResult {
        //let input_value = self.resolve("?".to_owned()).ok_or(EvalError::VarNotFound)?;
        for expr in exprs.iter() {
            let res = self.eval(Box::new(expr.clone()))?;
            if res.bool_value() {
                return Ok(BoolV(true));
            }
        }
        Ok(BoolV(false))
    }

    #[inline(always)]
    fn eval_func_call(&mut self, func_ref: Box<Node>, call_args: Vec<FuncCallArg>) -> EvalResult {
        let fref = self.eval(func_ref)?;
        match fref {
            NativeFuncV { func, arg_names } => self.call_native_func(func.0, arg_names, call_args),
            FuncV { func_def } => self.call_func(func_def, call_args),
            MacroV {
                callback,
                arg_names,
            } => self.call_macro(callback, arg_names, call_args),
            _ => {
                return Err(EvalError::Runtime(format!(
                    "cannot call non function {}",
                    fref.data_type()
                )))
            }
        }
    }

    fn call_native_func(
        &mut self,
        func: NativeFunc,
        arg_names: Vec<String>,
        call_args: Vec<FuncCallArg>,
    ) -> EvalResult {
        let mut arg_values: Vec<Value> = Vec::new();
        for a in call_args {
            let v = self.eval(a.arg)?;
            arg_values.push(v);
        }

        if arg_names.len() > arg_values.len() {
            return Err(EvalError::runtime(
                "native func call with too few arguments",
            ));
        }
        let mut args: HashMap<String, Value> = HashMap::new();
        for (i, arg_name) in arg_names.iter().enumerate() {
            let value = &arg_values[i];
            args.insert(arg_name.clone(), value.clone());
        }
        func(self, args)
    }

    fn call_macro(
        &mut self,
        callback: MacroCbT,
        arg_names: Vec<String>,
        call_args: Vec<FuncCallArg>,
    ) -> EvalResult {
        if arg_names.len() > call_args.len() {
            return Err(EvalError::runtime("call macro with too few arguments"));
        }

        let mut args: HashMap<String, Box<Node>> = HashMap::new();
        for (i, arg_name) in arg_names.iter().enumerate() {
            args.insert(arg_name.clone(), call_args[i].arg.clone());
        }
        callback.0(self, args)
    }

    fn call_func(&mut self, func_def: Box<Node>, call_args: Vec<FuncCallArg>) -> EvalResult {
        let mut arg_values: Vec<Value> = Vec::new();
        for a in call_args {
            let v = self.eval(a.arg)?;
            arg_values.push(v);
        }

        if let FuncDef { arg_names, body } = *func_def.syntax {
            if arg_names.len() > arg_values.len() {
                return Err(EvalError::runtime("func call with too few arguments"));
            }
            self.push_frame();
            for (i, arg_name) in arg_names.iter().enumerate() {
                let value = &arg_values[i];
                self.set_var(arg_name.clone(), value.clone());
            }
            let result = self.eval(body);
            self.pop_frame();
            result
        } else {
            Err(EvalError::Runtime(format!(
                "cannot call non funct {}",
                func_def
            )))
        }
    }

    // logic ops
    #[inline(always)]
    fn eval_logicop(&mut self, op: String, left: Box<Node>, right: Box<Node>) -> EvalResult {
        let left_value = self.eval(left)?;
        let left_bool_value = left_value.bool_value();
        match op.as_str() {
            "and" => {
                let right_value = self.eval(right)?;
                Ok(BoolV(left_bool_value && right_value.bool_value()))
            }
            "or" => {
                if left_bool_value {
                    return Ok(BoolV(true));
                } else {
                    let right_value = self.eval(right)?;
                    return Ok(BoolV(right_value.bool_value()));
                }
            }
            _ => Err(EvalError::Runtime(format!("un expected logic op {}", op))),
        }
    }

    // binary ops
    #[inline(always)]
    fn eval_binop(&mut self, op: String, left: Box<Node>, right: Box<Node>) -> EvalResult {
        let left_value = self.eval(left)?;
        let right_value = self.eval(right)?;
        match op.as_str() {
            "+" => Ok((left_value + right_value)?),
            "-" => Ok((left_value - right_value)?),
            "*" => Ok((left_value * right_value)?),
            "/" => Ok((left_value / right_value)?),
            "%" => Ok((left_value % right_value)?),
            ">" => Ok(BoolV(left_value > right_value)),
            ">=" => Ok(BoolV(left_value >= right_value)),
            "<" => Ok(BoolV(left_value < right_value)),
            "<=" => Ok(BoolV(left_value <= right_value)),
            "!=" => Ok(BoolV(left_value != right_value)),
            "=" => Ok(BoolV(left_value == right_value)),
            "[]" => self.eval_binop_index(left_value, right_value),
            "in" => self.eval_binop_in(left_value, right_value),
            _ => return Err(EvalError::Runtime(format!("unknown op {}", op))),
        }
    }

    #[inline(always)]
    fn eval_binop_index(&mut self, left_value: Value, right_value: Value) -> EvalResult {
        match left_value {
            MapV(a) => match right_value {
                StrV(k) => {
                    let m = a.borrow();
                    let v = m.get(&k).ok_or(EvalError::KeyError)?;
                    Ok(v.clone())
                }
                _ => Err(EvalError::runtime("map key not string")),
            },
            ArrayV(a) => match right_value {
                NumberV(idx) => {
                    // in FEEL language index starts from 1
                    let arr = a.borrow();
                    if !idx.is_integer()
                        || idx < dec!(1)
                        || idx > Decimal::from_usize(arr.len()).unwrap()
                    {
                        return Err(EvalError::IndexError);
                    }
                    let idx0 = idx.to_usize().unwrap();
                    let v = arr.get(idx0 - 1).ok_or(EvalError::IndexError)?;
                    Ok(v.clone())
                }
                _ => Err(EvalError::runtime("array index not string")),
            },
            _ => Err(EvalError::Runtime(format!(
                "value {} is not indexable",
                left_value.data_type()
            ))),
        }
    }

    #[inline(always)]
    fn eval_binop_in(&mut self, left_value: Value, right_value: Value) -> EvalResult {
        match right_value {
            RangeV(rng) => {
                let contains = rng.contains(left_value);
                Ok(BoolV(contains))
            }
            _ => Err(EvalError::Runtime(format!(
                "cannot perform in op on {}",
                right_value.data_type(),
            ))),
        }
    }

    #[inline(always)]
    fn eval_dotop(&mut self, left: Box<Node>, attr: String) -> EvalResult {
        let left_value = self.eval(left)?;
        match left_value {
            MapV(a) => {
                let m = a.borrow();
                let v = m.get(&attr).ok_or(EvalError::KeyError)?;
                Ok(v.clone())
            }
            _ => Err(EvalError::runtime("map is not indexable")),
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
            ("8 % 5", "3"),
            ("8 / 5", "1.6"),
            ("true and false", "false"),
            ("false or 2", "true"),
            ("not (false or 2)", "false"),
            (
                r#"@"2023-06-01T10:33:20+01:00" + @"P3Y11M""#,
                "2027-05-01T10:33:20+01:00",
            ),
            (
                r#"@"2023-06-01T10:33:20+01:00" - @"P1Y2M""#,
                "2022-04-01T10:33:20+01:00",
            ),
            (
                r#" @"2023-06-01T10:33:20+01:00" - @"2022-04-01T10:33:20+01:00" "#,
                "P426DT0.2446661632S",
            ),
            (r#"@"2023-09-17" < @"2023-10-02""#, "true"),
            (r#""abc" + "def""#, r#""abcdef""#),
            ("2 < 3 - 1", "false"),
            (r#""abc" <= "abd""#, "true"),
            ("[6, 1, 2, -3][4]", "-3"),
            ("[2, 8,false,true]", "[2, 8, false, true]"),
            ("{a: 1, b: 2}", r#"{"a":1, "b":2}"#),
            // ranges
            ("5 in (5..8]", "false"),
            ("5 in [5..8)", "true"),
            ("8 in [5..8)", "false"),
            ("8 in [5..8]", "true"),
            // if expr
            ("if 2 > 3 then 6 else 8", "8"),
            ("for a in [2, 3, 4] return a * 2", "[4, 6, 8]"), // simple for loop
            (
                "for a in [2, 3, 4], b in [8, 1, 2] return a + b",
                "[[10, 3, 4], [11, 4, 5], [12, 5, 6]]",
            ),
            ("some a in [2, 8, 3, 6] satisfies a > 4", "8"),
            ("every a in [2, 8, 3, 6] satisfies a > 4", "[8, 6]"),
            ("2 * 8; true; null; 9 / 3", "3"),
            (r#"set("a", 5); a + 10.3"#, "15.3"), // expression list
            (r#"set("?", 5); >6, =8, < 3"#, "false"), // multi tests
            (r#"set("?", 5); >6, <8, < 3"#, "true"),
            (r#"is defined(a)"#, "false"),
            (r#"is defined([1, 2][1])"#, "true"),
            (r#"is defined([1, 2][-1])"#, "false"),
            (r#"is defined([1, 2][6])"#, "false"),
        ];

        for (input, output) in testcases {
            println!("eval {}", input);
            let mut intp = super::Intepreter::new();
            let node = parse(input).unwrap();
            let v = intp.eval(node).unwrap();
            assert_eq!(v.to_string(), output, "output mismatch input: '{}'", input);
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

    #[test]
    fn test_native_func_set() {
        let mut intp = super::Intepreter::new();
        let input = r#"set("hi", 5)"#;
        let node = parse(input).unwrap();
        let _ = intp.eval(node).unwrap();

        let input1 = r#"hi + 3"#;
        let node1 = parse(input1).unwrap();
        let v = intp.eval(node1).unwrap();
        assert_eq!(v.to_string(), "8");
    }

    #[test]
    fn test_func_call() {
        let mut intp = super::Intepreter::new();
        let input = r#"set("add2", function(a, b) a+b)"#;
        let node = parse(input).unwrap();
        let _ = intp.eval(node).unwrap();

        let input1 = r#"add2(4.5, 9)"#;
        let node1 = parse(input1).unwrap();
        let v = intp.eval(node1).unwrap();
        assert_eq!(v.to_string(), "13.5");
    }
}
