use std::cell::RefCell;
use std::collections::HashMap;
use std::error;
use std::fmt;

use std::rc::Rc;

use super::ast::{FuncCallArg, MapNodeItem, Node, NodeSyntax::*};
use super::helpers::unescape;
use super::parse::ParseError;
use super::prelude::PRELUDE;
use super::values::context::Context;
use super::values::numeric::Numeric;
use super::values::temporal::parse_temporal;
use super::values::value::{TypeError, ValueError};

use super::values::func::{MacroT, NativeFunc};
use super::values::range::RangeT;
use super::values::value::Value::{self, *};

// EvalError
#[derive(Debug)]
pub enum EvalError {
    VarNotFound(String),
    KeyError,
    IndexError,
    TypeError(String),
    Runtime(String),
    Parse(ParseError),
    ValueError(String),
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::VarNotFound(name) => write!(f, "VarNotFound: `{}`", name),
            Self::KeyError => write!(f, "{}", "KeyError"),
            Self::TypeError(expect) => write!(f, "TypeError: expect {}", expect),
            Self::IndexError => write!(f, "{}", "IndexError"),
            Self::Runtime(message) => write!(f, "RuntimeError: {}", message),
            Self::ValueError(message) => write!(f, "ValueError: {}", message),
            Self::Parse(err) => write!(f, "{}", err),
        }
    }
}

impl error::Error for EvalError {}
//     fn source(&self) -> Option<&(dyn error::Error + 'static)> {
//         match self {
//             Self::Decimal(err) => Some(err),
//             _ => None,
//         }
//     }
// }

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
        Self::ValueError(err.0)
    }
}

impl From<TypeError> for EvalError {
    fn from(err: TypeError) -> EvalError {
        Self::TypeError(err.0)
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

pub struct Engine {
    scopes: Vec<RefCell<ScopeFrame>>,
}

impl Engine {
    pub fn new() -> Engine {
        let mut eng = Engine { scopes: Vec::new() };
        eng.push_frame(); // prelude frame
        eng
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

    /// set the value of a variable by look up the stack
    pub fn set_var(&mut self, name: String, value: Value) {
        if self.scopes.len() == 0 {
            self.push_frame();
        }

        for frame_ref in self.scopes.iter().rev() {
            let mut frame = frame_ref.borrow_mut();
            if frame.vars.contains_key(&name) {
                frame.vars.insert(name.clone(), value);
                return;
            }
        }

        // if the value not found then set it to the top bar
        self.bind_var(name, value)
    }

    /// bind a variable to the top of stack
    pub fn bind_var(&mut self, name: String, value: Value) {
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
                func_def: Node::new(FuncDef { arg_names, body }, node.start_pos),
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
        //let content = String::from(&value[1..(value.len() - 1)]);
        let content = unescape(&value[1..(value.len() - 1)]);
        Ok(StrV(content))
    }

    pub fn is_defined(&mut self, value_node: &Box<Node>) -> EvalResult {
        if let Var(v) = *value_node.syntax.clone() {
            return match self.resolve(v) {
                Some(_) => Ok(BoolV(true)),
                None => Ok(BoolV(false)),
            };
        }
        match self.eval(value_node.clone()) {
            Ok(_) => Ok(BoolV(true)),
            Err(EvalError::IndexError)
            | Err(EvalError::KeyError)
            | Err(EvalError::VarNotFound(_)) => Ok(BoolV(false)),
            Err(err) => Err(err),
        }
    }

    #[inline(always)]
    fn eval_number(&mut self, number_str: String) -> EvalResult {
        let d = Numeric::from_str(number_str.as_str())
            .ok_or(ValueError("fail to parse numger".to_owned()))?;
        Ok(NumberV(d))
    }

    #[inline(always)]
    fn eval_var(&mut self, name: String) -> EvalResult {
        if let Some(value) = self.resolve(name.clone()) {
            Ok(value)
        } else {
            Err(EvalError::VarNotFound(name))
        }
    }

    #[inline(always)]
    fn eval_array(&mut self, elements: &Vec<Box<Node>>) -> EvalResult {
        let mut results = Vec::new();
        for elem in elements.iter() {
            let res = self.eval(elem.clone())?;
            results.push(res);
        }
        Ok(ArrayV(Rc::new(RefCell::new(results))))
    }

    #[inline(always)]
    fn eval_map(&mut self, items: &Vec<MapNodeItem>) -> EvalResult {
        let mut value_map = Context::new();
        for item in items.iter() {
            let k = self.eval(item.name.clone())?;
            let key = k.expect_string(format!("context item {}", item.name).as_str())?;
            //let key = k.to_string();
            let val = self.eval(item.value.clone())?;
            value_map.insert(key, val);
        }
        Ok(ContextV(Rc::new(RefCell::new(value_map))))
    }

    #[inline(always)]
    fn eval_neg_op(&mut self, node: Box<Node>) -> EvalResult {
        let pv = self.eval(node)?;
        Ok((-pv)?)
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
        if start_value.data_type() != end_value.data_type() {
            return Err(EvalError::ValueError(format!(
                "range start type {} != end type {}",
                start_value.data_type(),
                end_value.data_type()
            )));
        }
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
                Ok(ArrayV(Rc::new(RefCell::new(results))))
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
                Ok(ArrayV(Rc::new(RefCell::new(results))))
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
            NativeFuncV {
                func,
                require_args,
                optional_args,
                var_arg,
            } => self.call_native_func(&func, require_args, optional_args, var_arg, call_args),
            FuncV { func_def } => self.call_func(func_def, call_args),
            MacroV {
                macro_,
                require_args,
            } => self.call_macro(&macro_, require_args, call_args),
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
        func: &NativeFunc,
        require_args: Vec<String>,
        optional_args: Vec<String>,
        var_arg: Option<String>,
        call_args: Vec<FuncCallArg>,
    ) -> EvalResult {
        let call_args_len = call_args.len();
        if require_args.len() > call_args_len {
            return Err(EvalError::Runtime(format!(
                "too few arguments, expect at least {} args, found {}",
                require_args.len(),
                call_args_len
            )));
        } else if var_arg.is_none() && require_args.len() + optional_args.len() < call_args.len() {
            return Err(EvalError::Runtime(format!(
                "too many arguments, expect at most {} args, found {}",
                require_args.len() + optional_args.len(),
                call_args_len
            )));
        }

        let mut named_args: HashMap<String, Value> = HashMap::new();
        let mut positional_arg_index = 0;
        // build args
        let mut var_arg_values: Vec<Value> = vec![];
        let mut use_var_arg = false;
        for call_arg in call_args {
            // resolve argument name
            let arg_name = match call_arg.arg_name.as_str() {
                "" => {
                    let implicit_arg_name = if positional_arg_index < require_args.len() {
                        require_args[positional_arg_index].as_str()
                    } else if positional_arg_index < require_args.len() + optional_args.len() {
                        optional_args[positional_arg_index - require_args.len()].as_str()
                    } else if let Some(ref var_arg_name) = var_arg {
                        use_var_arg = true;
                        var_arg_name.as_str()
                    } else {
                        return Err(EvalError::Runtime(format!(
                            "too many arguments, expect at most {} args, found {}",
                            require_args.len() + optional_args.len(),
                            call_args_len
                        )));
                    };
                    positional_arg_index += 1;
                    implicit_arg_name
                }
                a => a,
            };
            if named_args.contains_key(arg_name) {
                return Err(EvalError::ValueError(format!(
                    "argument {} already set",
                    arg_name
                )));
            }
            let arg_value = self.eval(call_arg.arg)?;
            if use_var_arg {
                var_arg_values.push(arg_value);
            } else {
                named_args.insert(arg_name.to_owned(), arg_value.clone());
            }
        }

        if var_arg.is_some() {
            // make var arg as an Array value
            let var_arg_name = var_arg.unwrap_or("_".to_string());
            let v = ArrayV(Rc::new(RefCell::new(var_arg_values)));
            named_args.insert(var_arg_name, v);
        }
        (func.body)(self, named_args)
    }

    fn call_macro(
        &mut self,
        macro_obj: &MacroT,
        require_args: Vec<String>,
        call_args: Vec<FuncCallArg>,
    ) -> EvalResult {
        if require_args.len() > call_args.len() {
            return Err(EvalError::Runtime(format!(
                "call macro {} expect {} args, found {}",
                macro_obj.name,
                require_args.len(),
                call_args.len()
            )));
        }

        let mut args: HashMap<String, Box<Node>> = HashMap::new();
        for (i, arg_name) in require_args.iter().enumerate() {
            args.insert(arg_name.clone(), call_args[i].arg.clone());
        }
        (macro_obj.body)(self, args)
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
        let left_bool_value = self.eval(left)?.bool_value();
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
            ContextV(a) => match right_value {
                StrV(k) => {
                    let m = a.borrow();
                    let v = m.get(k).ok_or(EvalError::KeyError)?;
                    Ok(v)
                }
                _ => Err(EvalError::runtime("map key not string")),
            },
            ArrayV(a) => match right_value {
                NumberV(idx) => {
                    // in FEEL language index starts from 1
                    let arr = a.borrow();
                    if !idx.is_integer()
                        || idx < Numeric::ONE
                        || idx > Numeric::from_usize(arr.len())
                    {
                        return Err(EvalError::IndexError);
                    }
                    let idx0 = idx.to_usize().unwrap();
                    let v = arr.get(idx0 - 1).ok_or(EvalError::IndexError)?;
                    Ok(v.clone())
                }
                _ => Err(EvalError::runtime("array index not integer")),
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
                let contains = rng.contains(&left_value);
                Ok(BoolV(contains))
            }
            ArrayV(a) => {
                let arr = a.borrow();
                for v in arr.iter() {
                    if *v == left_value {
                        return Ok(BoolV(true));
                    }
                }
                Ok(BoolV(false))
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
            ContextV(a) => {
                let m = a.borrow();
                let v = m.get(attr).ok_or(EvalError::KeyError)?;
                Ok(v)
            }
            _ => Err(EvalError::runtime("map is not indexable")),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{parse::parse, values::numeric::Numeric};
    use core::assert_matches::assert_matches;

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
            ("10 / 3", "3.333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333"), // precision is up to 28
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
            (r#""abc" + "de\\nf""#, r#""abcde\nf""#),
            ("2 < 3 - 1", "false"),
            (r#""abc" <= "abd""#, "true"),
            ("[6, 1, 2, -3][4]", "-3"),
            ("[2, 8,false,true]", "[2, 8, false, true]"),
            ("{a: 1, b: 2}", r#"{"a":1, "b":2}"#),
            // in operator over ranges and arrays
            ("5 in (5..8]", "false"),
            ("5 in [5..8)", "true"),
            ("8 in [5..8)", "false"),
            ("8 in [5..8]", "true"),
            (r#" "c" in ["a".."z"]"#, "true"),
            (r#" "f" in ["a".."f")"#, "false"),
            ("7 in [2, 7, 8]", "true"),
            ("7 in [3, 99, -1]", "false"),
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
            // test prelude functions
            ("not(2>1)", "false"),
            (r#"number("3000.888")"#, "3000.888"),
            (r#"string length("hello world")"#, "11"),
            (r#"string join(["hello", "world", "again"], ", ", ":")"#, r#"":hello, world, again""#),
            // boolean functions
            (r#"get or else("this", "default")"#, r#""this""#),
            (r#"get or else(null, "default")"#, r#""default""#),
            ("get or else(null, null)", "null"),
            // number functions
            ("decimal(1/3, 2)", "0.33"),
            ("decimal(1.5, 0)", "2"),
            (r#"decimal("1.56", 9)"#, "1.560000000"),

            ("floor(1.5)", "1"),
            ("floor(-1.5)", "-2"),
            ("floor(-1.56, 1)", "-1.6"),
            ("ceiling(1.5)", "2"),
            ("ceiling(-1.5)", "-1"),
            ("ceiling(-1.56, 1)", "-1.5"),

            // list functions
            ("list contains([2, 8, -1], 8)", "true"),
            (r#"list contains([2, 8, "hello"], "world")"#, "false"),
            ("count(1, 2, 4, 9, -3)", "5"),
            ("count()", "0"),
            ("min(31, -1, 9, 8, -1, -99)", "-99"),
            ("min(31, -1, 9, false, -1, -99)", "-99"),
            ("max(31, -1, 9, 8, -1, -99)", "31"),
            ("sum(31, -1, 9, false, -1, -99)", "-61"),  
            ("sort([3, -1, 2])", "[-1, 2, 3]"),
            ("sublist([1,2,3], 2)", "[2, 3]"),
            ("sublist([1,2,3], 1, 2)", "[1, 2]"),
            ("append([1], 2, 3)", "[1, 2, 3]"),
            ("append([1, 2, 3])", "[1, 2, 3]"),
            ("concatenate([1,2],[3])", "[1, 2, 3]"),
            ("concatenate([1],[2],[3])", "[1, 2, 3]"),
            ("insert before([1, 3], 1, 2)", "[2, 1, 3]"),
            ("remove([1,2,3], 2)", "[1, 3]"),
            ("reverse([1,2,3])", "[3, 2, 1]"),
            ("index of([1,2,3,2], 2)", "[2, 4]"),
            // test context functions
            (r#"get value({"a": 5, b: 9}, "b")"#, "9"),
            (r#"get value({"a": 5, b: {"c k": {m: 5}}}, ["b", "c k", "m"])"#, "5"),
            (r#"context put({"o":8}, ["a", "b", "c d"], 3)"#, r#"{"a":{"b":{"c d":3}}, "o":8}"#),
            (r#"context put({a: {b: {"c d":3}}, o:8}, ["a", "b", "c d"], 6)"#, r#"{"a":{"b":{"c d":6}}, "o":8}"#),
            ("context merge([{a:1}, {b:2}, {c:3}])", r#"{"a":1, "b":2, "c":3}"#),
            ("get entries({a: 2, b: 8})", r#"[{"key":"a", "value":2}, {"key":"b", "value":8}]"#),

            // test range functions
            ("before(1, 10)", "true"),
            ("before(10, 1)", "false"),
            ("before([1..5], 10)", "true"),
            ("before(1, [2..5])", "true"),
            ("before(3, [2..5])", "false"),

            ("before([1..5),[5..10])", "true"),
            ("before([1..5),(5..10])", "true"),
            ("before([1..5],[5..10])", "false"),
            ("before([1..5),(5..10])", "true"),

            ("after([5..10], [1..5))", "true"),
            ("after((5..10], [1..5))", "true"),
            ("after([5..10], [1..5])", "false"),
            ("after((5..10], [1..5))", "true"),

            ("meets([1..5], [5..10])", "true"),
            ("meets([1..3], [4..6])", "false"),
            ("meets([1..3], [3..5])", "true"),
            ("meets([1..5], (5..8])", "false"),

            ("met by([5..10], [1..5])", "true"),
            ("met by([3..4], [1..2])", "false"),
            ("met by([3..5], [1..3])", "true"),
            ("met by((5..8], [1..5))", "false"),
            ("met by([5..10], [1..5))", "false"),


            ("overlaps([5..10], [1..6])", "true"),
            ("overlaps((3..7], [1..4])", "true"),
            ("overlaps([1..3], (3..6])", "false"),
            ("overlaps((5..8], [1..5))", "false"),
            ("overlaps([4..10], [1..5))", "true"),

            ("overlaps before([1..5], [4..10])", "true"),
            ("overlaps before([3..4], [1..2])", "false"),
            ("overlaps before([1..3], (3..5])", "false"),
            ("overlaps before([1..5), (3..8])", "true"),
            ("overlaps before([1..5), [5..10])", "false"),

            ("overlaps after([4..10], [1..5])", "true"),
            ("overlaps after([3..4], [1..2])", "false"),
            ("overlaps after([3..5], [1..3))", "false"),
            ("overlaps after((5..8], [1..5))", "false"),
            ("overlaps after([4..10], [1..5))", "true"),

            ("finishes(5, [1..5])", "true"),
            ("finishes(10, [1..7])", "false"),
            ("finishes([3..5], [1..5])", "true"),
            ("finishes((1..5], [1..5))", "false"),
            ("finishes([5..10], [1..10))", "false"),

            ("finished by([5..10], 10)", "true"),
            ("finished by([3..4], 2)", "false"),

            ("finished by([1..5], [3..5])", "true"),
            ("finished by((5..8], [1..5))", "false"),
            ("finished by([5..10], (1..10))", "false"),

            ("includes([5..10], 6)", "true"),
            ("includes([3..4], 5)", "false"),
            ("includes([1..10], [4..6])", "true"),
            ("includes((5..8], [1..5))", "false"),
            ("includes([1..10], [1..5))", "true"),

            ("during(5, [1..10])", "true"),
            ("during(12, [1..10])", "false"),
            ("during(1, (1..10])", "false"),
            ("during([4..6], [1..10))", "true"),
            ("during((1..5], (1..10])", "true"),

            ("starts(1, [1..5])", "true"),
            ("starts(1, (1..8])", "false"),
            ("starts((1..5], [1..5])", "false"),
            ("starts([1..10], [1..10])", "true"),
            ("starts((1..10), (1..10))", "true"),

            ("started by([1..10], 1)", "true"),
            ("started by((1..10], 1)", "false"),
            ("started by([1..10], [1..5])", "true"),
            ("started by((1..10], [1..5))", "false"),
            ("started by([1..10], [1..10))", "true"),

            ("coincides([1..5], [1..5])", "true"),
            ("coincides((1..5], [1..5))", "false"),
            ("coincides([1..5], [2..6])", "false"),

        ];

        for (input, output) in testcases {
            let mut eng = super::Engine::new();
            let node = parse(input).unwrap();
            let v = eng.eval(node).unwrap();
            assert_eq!(v.to_string(), output, "output mismatch input: '{}'", input);
        }
    }

    #[test]
    fn test_def_vars() {
        let mut eng = super::Engine::new();
        eng.set_var(
            "v1".to_owned(),
            super::NumberV(Numeric::from_str("2.3").unwrap()),
        );
        let input = "v1 + 3";
        let node = parse(input).unwrap();
        let v = eng.eval(node).unwrap();
        assert_eq!(v.to_string(), "5.3");
    }

    #[test]
    fn test_native_func_set() {
        let mut eng = super::Engine::new();
        let input = r#"set("hi", 5)"#;
        let node = parse(input).unwrap();
        let _ = eng.eval(node).unwrap();

        let input1 = r#"hi + 3"#;
        let node1 = parse(input1).unwrap();
        let v = eng.eval(node1).unwrap();
        assert_eq!(v.to_string(), "8");
    }

    #[test]
    fn test_func_call() {
        let mut eng = super::Engine::new();
        let input = r#"set("add2", function(a, b) a+b)"#;
        let node = parse(input).unwrap();
        let _ = eng.eval(node).unwrap();

        let input1 = r#"add2(4.5, 9)"#;
        let node1 = parse(input1).unwrap();
        let v = eng.eval(node1).unwrap();
        assert_eq!(v.to_string(), "13.5");
    }
}
