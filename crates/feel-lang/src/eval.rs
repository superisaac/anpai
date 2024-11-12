use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error;
use std::fmt;

use std::rc::Rc;

use crate::scan::TextPosition;

use self::EvalErrorKind::*;

use super::ast::{FuncCallArg, MapNodeItem, Node, NodeSyntax::*};
use super::helpers::unescape;
use super::parse::{parse, ParseError};
use super::prelude::PRELUDE;
use super::values::context::Context;
use super::values::numeric::Numeric;
use super::values::temporal::parse_temporal;
use super::values::value::{TypeError, ValueError};

use super::values::func::{MacroT, NativeFunc};
use super::values::range::RangeT;
use super::values::value::Value::{self, *};

// EvalError
#[derive(Debug, Clone)]
pub enum EvalErrorKind {
    VarNotFound(String),
    KeyError,
    IndexError,
    TypeError(String),
    Runtime(String),
    Parse(ParseError),
    ValueError(String),
}

impl fmt::Display for EvalErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::VarNotFound(name) => write!(f, "VarNotFound: `{}`", name),
            Self::KeyError => write!(f, "{}", "KeyError"),
            Self::TypeError(expect) => write!(f, "TypeError: expect {}", expect),
            Self::IndexError => write!(f, "{}", "IndexError"),
            Self::Runtime(message) => write!(f, "RuntimeError: {}", message),
            Self::ValueError(message) => write!(f, "ValueError: {}", message),
            Self::Parse(parse_err) => write!(f, "{}", parse_err),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EvalError {
    pub kind: EvalErrorKind,
    pub pos: TextPosition,
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} at {}", self.kind, self.pos)
    }
}

impl error::Error for EvalError {}

impl From<String> for EvalError {
    fn from(err: String) -> EvalError {
        Self::new(Runtime(err))
    }
}

impl From<ParseError> for EvalError {
    fn from(err: ParseError) -> EvalError {
        Self::new(Parse(err))
    }
}

impl From<(ParseError, TextPosition)> for EvalError {
    fn from(err: (ParseError, TextPosition)) -> EvalError {
        Self::new_with_pos(Parse(err.0), err.1)
    }
}

impl From<ValueError> for EvalError {
    fn from(err: ValueError) -> EvalError {
        Self::new(EvalErrorKind::ValueError(err.0))
    }
}

impl From<TypeError> for EvalError {
    fn from(err: TypeError) -> EvalError {
        Self::new(EvalErrorKind::TypeError(err.0))
    }
}

impl EvalError {
    pub fn new(kind: EvalErrorKind) -> EvalError {
        EvalError {
            kind,
            pos: TextPosition::zero(),
        }
    }

    pub fn new_with_pos(kind: EvalErrorKind, pos: TextPosition) -> EvalError {
        EvalError { kind, pos }
    }

    pub fn runtime(message: &str) -> EvalError {
        Self::new(Runtime(String::from(message)))
    }

    pub fn value_error(message: &str) -> EvalError {
        Self::new(EvalErrorKind::ValueError(String::from(message)))
    }

    pub fn type_error(message: &str) -> EvalError {
        Self::new(EvalErrorKind::TypeError(String::from(message)))
    }

    pub fn index_error() -> EvalError {
        Self::new(EvalErrorKind::IndexError)
    }

    // pub fn var_not_found(varname: &str) -> EvalError {
    //     Self::new(VarNotFound(String::from(varname)))
    // }

    pub fn with_pos(&self, pos: TextPosition) -> EvalError {
        EvalError {
            kind: self.kind.clone(),
            pos,
        }
    }

    pub fn with_pos_if_zero(&self, pos: TextPosition) -> EvalError {
        if pos.is_zero() {
            EvalError {
                kind: self.kind.clone(),
                pos,
            }
        } else {
            self.clone()
        }
    }
}

pub type EvalResult = Result<Value, EvalError>;

#[derive(Clone)]
pub struct ScopeFrame {
    vars: HashMap<String, Value>,
}

#[derive(Clone)]
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

    pub fn has_name(&self, name: String) -> bool {
        for scope in self.scopes.iter().rev() {
            if let Some(_v) = scope.borrow().vars.get(&name) {
                return true;
            }
        }
        PRELUDE.has_name(name)
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

    pub fn as_box(&self) -> Box<Engine> {
        return Box::new(self.clone());
    }

    pub fn load_context(&mut self, ctx_input: &str) -> EvalResult {
        let node = parse(ctx_input, Box::new(self.clone()))?;
        let ctx_value = self.eval(node)?;
        return match ctx_value {
            ContextV(m) => {
                self.push_frame();
                let ctx_entries = m.as_ref().borrow().entries();
                for (k, v) in ctx_entries {
                    self.set_var(k, v);
                }
                Ok(BoolV(true))
            }
            _ => Err(EvalError::new(EvalErrorKind::ValueError(
                "context/map required".to_owned(),
            ))),
        };
    }

    pub fn eval(&mut self, node: Box<Node>) -> EvalResult {
        let start_pos = node.start_pos;
        let res = match *node.syntax {
            Null => Ok(NullV),
            Bool(value) => Ok(BoolV(value)),
            Number(value) => self.eval_number(value),
            Str(value) => self.eval_string(value),
            Temporal(value) => Ok(parse_temporal(value.as_str())?),
            Ident(value) => Ok(StrV(value)),
            Var(name) => self.eval_var(name),
            Neg(value) => self.eval_neg_op(value),
            BinOp { op, left, right } => self.eval_binop(op, left, right),
            InOp { left, right } => self.eval_in_op(left, right),
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
            FuncDef {
                arg_names,
                body,
                code,
            } => Ok(FuncV {
                func_def: Node::new(
                    FuncDef {
                        arg_names,
                        body,
                        code: code.clone(),
                    },
                    start_pos.clone(),
                ),
                code,
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
        };
        return match res {
            Ok(v) => Ok(v),
            Err(err) => Err(err.with_pos_if_zero(start_pos)),
        };
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
        self.push_frame();
        let r = match self.eval(value_node.clone()) {
            Ok(_) => Ok(BoolV(true)),
            Err(EvalError {
                kind: IndexError,
                pos: _,
            })
            | Err(EvalError {
                kind: KeyError,
                pos: _,
            })
            | Err(EvalError {
                kind: VarNotFound(_),
                pos: _,
            }) => Ok(BoolV(false)),
            Err(err) => Err(err),
        };
        self.pop_frame();
        r
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
            Err(EvalError::new(VarNotFound(name)))
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
        let end_value = self.eval(end_node.clone())?;
        if start_value.data_type() != end_value.data_type() {
            return Err(EvalError::new_with_pos(
                EvalErrorKind::ValueError(format!(
                    "range start type {} != end type {}",
                    start_value.data_type(),
                    end_value.data_type()
                )),
                end_node.start_position(),
            ));
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
                let refarr: &RefCell<Vec<Value>> = items.borrow();
                for item in refarr.borrow().iter() {
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
                let refarr: &RefCell<Vec<Value>> = items.borrow();
                for item in refarr.borrow().iter() {
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
                let refarr: &RefCell<Vec<Value>> = items.borrow();
                for item in refarr.borrow().iter() {
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
    fn eval_expr_list_in(&mut self, exprs: Vec<Box<Node>>) -> EvalResult {
        let left_value = self
            .resolve("?".to_owned())
            .ok_or(EvalError::new(VarNotFound("?".to_owned())))?;
        for expr in exprs.iter() {
            let res = self.eval(expr.clone())?;
            if let BoolV(true) = res {
                return Ok(BoolV(true));
            } else if left_value == res {
                return Ok(BoolV(true));
            }
        }
        Ok(BoolV(false))
    }

    #[inline(always)]
    fn eval_expr_list(&mut self, exprs: Vec<Box<Node>>) -> EvalResult {
        let mut last_value: Option<Value> = None;
        for expr in exprs.iter() {
            let res = self.eval(expr.clone())?;
            last_value = Some(res);
        }
        if let Some(v) = last_value {
            Ok(v)
        } else {
            Ok(NullV)
        }
    }

    #[inline(always)]
    fn eval_multi_tests(&mut self, exprs: Vec<Box<Node>>) -> EvalResult {
        self.eval_expr_list_in(exprs)
        // //let input_value = self.resolve("?".to_owned()).ok_or(EvalError::VarNotFound)?;
        // for expr in exprs.iter() {
        //     let res = self.eval(expr.clone())?;
        //     if res.bool_value() {
        //         return Ok(BoolV(true));
        //     }
        // }
        // Ok(BoolV(false))
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
            FuncV { func_def, code: _ } => self.call_func(func_def, call_args),
            MacroV {
                macro_,
                require_args,
            } => self.call_macro(&macro_, require_args, call_args),
            _ => {
                return Err(EvalError::runtime(
                    format!("cannot call non function {}", fref.data_type()).as_str(),
                ))
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
            return Err(EvalError::new(Runtime(format!(
                "too few arguments, expect at least {} args, found {}",
                require_args.len(),
                call_args_len
            ))));
        } else if var_arg.is_none() && require_args.len() + optional_args.len() < call_args.len() {
            return Err(EvalError::new(Runtime(format!(
                "too many arguments, expect at most {} args, found {}",
                require_args.len() + optional_args.len(),
                call_args_len
            ))));
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
                        return Err(EvalError::new(Runtime(format!(
                            "too many arguments, expect at most {} args, found {}",
                            require_args.len() + optional_args.len(),
                            call_args_len
                        ))));
                    };
                    positional_arg_index += 1;
                    implicit_arg_name
                }
                a => a,
            };
            if named_args.contains_key(arg_name) {
                return Err(EvalError::new(EvalErrorKind::ValueError(format!(
                    "argument {} already set",
                    arg_name
                ))));
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
            return Err(EvalError::new(Runtime(format!(
                "call macro {} expect {} args, found {}",
                macro_obj.name,
                require_args.len(),
                call_args.len()
            ))));
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

        if let FuncDef {
            arg_names,
            body,
            code: _,
        } = *func_def.syntax
        {
            if arg_names.len() > arg_values.len() {
                return Err(EvalError::new(Runtime(
                    "func call with too few arguments".to_owned(),
                )));
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
            Err(EvalError::new(Runtime(format!(
                "cannot call non funct {}",
                func_def
            ))))
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
            _ => Err(EvalError::new(Runtime(format!(
                "un expected logic op {}",
                op
            )))),
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
            //"in" => self.eval_binop_in(left_value, right_value),
            _ => return Err(EvalError::new(Runtime(format!("unknown op {}", op)))),
        }
    }

    #[inline(always)]
    fn eval_binop_index(&mut self, left_value: Value, right_value: Value) -> EvalResult {
        match left_value {
            ContextV(a) => match right_value {
                StrV(k) => {
                    let refctx: &RefCell<Context> = a.borrow();
                    //let m = a.borrow();
                    let v = refctx.borrow().get(k).ok_or(EvalError::new(KeyError))?;
                    Ok(v)
                }
                _ => Err(EvalError::new(Runtime("map key not string".to_owned()))),
            },
            ArrayV(a) => match right_value {
                NumberV(idx) => {
                    // in FEEL language index starts from 1
                    let refarr: &RefCell<Vec<Value>> = a.borrow();
                    let arr = refarr.borrow();
                    if !idx.is_integer()
                        || idx < Numeric::ONE
                        || idx > Numeric::from_usize(arr.len())
                    {
                        return Err(EvalError::new(IndexError));
                    }
                    let idx0 = idx.to_usize().unwrap();

                    let v = arr.get(idx0 - 1).ok_or(EvalError::new(IndexError))?;
                    Ok(v.clone())
                }
                _ => Err(EvalError::runtime("array index not integer")),
            },
            _ => Err(EvalError::new(Runtime(format!(
                "value {} is not indexable",
                left_value.data_type()
            )))),
        }
    }

    #[inline(always)]
    fn eval_in_op(&mut self, left: Box<Node>, right: Box<Node>) -> EvalResult {
        let left_value = self.eval(left)?;
        match *right.syntax {
            ExprList(items) => {
                self.push_frame();
                self.bind_var("?".to_owned(), left_value.clone());
                let res = self.eval_expr_list_in(items);
                self.pop_frame();
                return res;
            }
            _ => (),
        }
        self.push_frame();
        let right_res = self.eval(right);
        self.pop_frame();
        let right_value = right_res?;
        match right_value {
            RangeV(rng) => {
                let contains = rng.contains(&left_value);
                Ok(BoolV(contains))
            }
            ArrayV(a) => {
                let refarr: &RefCell<Vec<Value>> = a.borrow();
                for v in refarr.borrow().iter() {
                    if *v == left_value {
                        return Ok(BoolV(true));
                    }
                }
                Ok(BoolV(false))
            }
            x => Ok(BoolV(x == left_value)), // _ => Err(EvalError::Runtime(format!(
                                             //     "cannot perform in op on {}",
                                             //     right_value.data_type(),
                                             // ))),
        }
    }

    #[inline(always)]
    fn eval_dotop(&mut self, left: Box<Node>, attr: String) -> EvalResult {
        let left_value = self.eval(left)?;
        match left_value {
            ContextV(a) => {
                let refctx: &RefCell<Context> = a.borrow();
                //let m = a.borrow();
                let v = refctx.borrow().get(attr).ok_or(EvalError::new(KeyError))?;
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
            (None, "2+ 4", "6"),
            (None, "2 -5", "-3"),
            (None, "8 - 2", "6"),
            (None, "7 / 2", "3.5"), // decimal display outputs normalized string
            (None, "10 / 3", "3.333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333"), // precision is up to 28
            (None, "4 * 9 + 1", "37"),
            (None, "8 % 5", "3"),
            (None, "8 / 5", "1.6"),
            (None, "true and false", "false"),
            (None, "false or 2", "true"),
            (None, "not (false or 2)", "false"),
            (
                None,
                r#"@"2023-06-01T10:33:20+01:00" + @"P3Y11M""#,
                r#"date and time("2027-05-01T10:33:20+01:00")"#,
            ),
            (
                None,
                r#"@"2023-06-01T10:33:20+01:00" - @"P1Y2M""#,
                r#"date and time("2022-04-01T10:33:20+01:00")"#,
            ),
            (
                None,
                r#" @"2023-06-01T10:33:20+01:00" - @"2022-04-01T10:33:20+01:00" "#,
                r#"duration("P426DT0.2446661632S")"#,
            ),
            (None, r#"@"2023-09-17" < @"2023-10-02""#, "true"),
            (None, r#""abc" + "de\\nf""#, r#""abcde\nf""#),
            (None, "2 < 3 - 1", "false"),
            (None, r#""abc" <= "abd""#, "true"),
            (None, "[6, 1, 2, -3][4]", "-3"),
            (None, "[2, 8,false,true]", "[2, 8, false, true]"),
            (None, "{a: 1, b: 2}", r#"{"a":1, "b":2}"#),
            // in operator over ranges and arrays
            (None, "5 in (5..8]", "false"),
            (None, "5 in [5..8)", "true"),
            (None, "8 in [5..8)", "false"),
            (None, "8 in [5..8]", "true"),
            (None, r#" "c" in ["a".."z"]"#, "true"),
            (None, r#" "f" in ["a".."f")"#, "false"),
            (None, "7 in [2, 7, 8]", "true"),
            (None, "7 in [3, 99, -1]", "false"),
            // if expr
            (None, "if 2 > 3 then 6 else 8", "8"),
            (None, "for a in [2, 3, 4] return a * 2", "[4, 6, 8]"), // simple for loop
            (None, r#"for `a&b-c` in [2, 3, 4] return `a&b-c` * 2"#, "[4, 6, 8]"), // simple for loop
            (
                None,
                "for a in [2, 3, 4], b in [8, 1, 2] return a + b",
                "[[10, 3, 4], [11, 4, 5], [12, 5, 6]]",
            ),
            (None, "some a in [2, 8, 3, 6] satisfies a > 4", "8"),
            (None, "every a in [2, 8, 3, 6] satisfies a > 4", "[8, 6]"),
            //("2 * 8; true; null; 9 / 3", "3"),
            (None, "2 in (>=5, <3)", "true"),

            (Some("{a: 5}"), r#"a + 10.3"#, "15.3"), // expression list
            (Some(r#"{"?": 5}"#), r#">6, =8, < 3"#, "false"), // multi tests
            (Some(r#"{"?": 5}"#), r#">6, <8, < 3"#, "true"),
            (Some(r#"{"???": 5}"#), r#"??? + 6"#, "11"),
            (Some(r#"{a+b: 9}"#), "a+b*2", "18"),
            (None, r#"{a: function(x,y) x+y}["a"](3, 5)"#, "8"),

            //(Some(r#"{"?": 5}"#), r#"?>6, ?<8, < 3"#, "true"),
            (None, r#"is defined(a)"#, "false"),
            (None, r#"is defined([1, 2][1])"#, "true"),
            (None, r#"is defined([1, 2][-1])"#, "false"),
            (None, r#"is defined([1, 2][6])"#, "false"),
            // test prelude functions
            (None, "not(2>1)", "false"),
            (None, r#"number("3000.888")"#, "3000.888"),
            (None, r#"string length("hello world")"#, "11"),
            (None, r#"string join(["hello", "world", "again"], ", ", ":")"#, r#"":hello, world, again""#),
            // boolean functions
            (None, r#"get or else("this", "default")"#, r#""this""#),
            (None, r#"get or else(null, "default")"#, r#""default""#),
            (None, "get or else(null, null)", "null"),
            // number functions
            (None, "decimal(1/3, 2)", "0.33"),
            (None, "decimal(1.5, 0)", "2"),
            (None, "decimal(1.5)", "1.5"),
            (None, r#"decimal("1.56", 9)"#, "1.560000000"),

            (None, "floor(1.5)", "1"),
            (None, "floor(-1.5)", "-2"),
            (None, "floor(-1.56, 1)", "-1.6"),
            (None, "ceiling(1.5)", "2"),
            (None, "ceiling(-1.5)", "-1"),
            (None, "ceiling(-1.56, 1)", "-1.5"),
            (None, "decimal(log(10), 12)", "2.302585092994"),
            (None, "odd(5)", "true"),
            (None, "odd(2)", "false"),
            (None, "even(5)", "false"),
            (None, "even(2)", "true"),

            // list functions
            (None, "list contains([2, 8, -1], 8)", "true"),
            (None, r#"list contains([2, 8, "hello"], "world")"#, "false"),
            (None, "count(1, 2, 4, 9, -3)", "5"),
            (None, "count()", "0"),
            (None, "min(31, -1, 9, 8, -1, -99)", "-99"),
            (None, "min(31, -1, 9, false, -1, -99)", "-99"),
            (None, "max(31, -1, 9, 8, -1, -99)", "31"),
            (None, "sum(31, -1, 9, false, -1, -99)", "-61"),  
            (None, "sort([3, -1, 2])", "[-1, 2, 3]"),
            (None, "sublist([1,2,3], 2)", "[2, 3]"),
            (None, "sublist([1,2,3], 1, 2)", "[1, 2]"),
            (None, "append([1], 2, 3)", "[1, 2, 3]"),
            (None, "append([1, 2, 3])", "[1, 2, 3]"),
            (None, "concatenate([1,2],[3])", "[1, 2, 3]"),
            (None, "concatenate([1],[2],[3])", "[1, 2, 3]"),
            (None, "insert before([1, 3], 1, 2)", "[2, 1, 3]"),
            (None, "remove([1,2,3], 2)", "[1, 3]"),
            (None, "reverse([1,2,3])", "[3, 2, 1]"),
            (None, "index of([1,2,3,2], 2)", "[2, 4]"),
            // test context functions
            (None, r#"get value({"a": 5, b: 9}, "b")"#, "9"),
            (None, r#"get value({"a": 5, b: {"c k": {m: 5}}}, ["b", "c k", "m"])"#, "5"),
            (None, r#"context put({"o":8}, ["a", "b", "c d"], 3)"#, r#"{"a":{"b":{"c d":3}}, "o":8}"#),
            (None, r#"context put({a: {b: {"c d":3}}, o:8}, ["a", "b", "c d"], 6)"#, r#"{"a":{"b":{"c d":6}}, "o":8}"#),
            (None, "context merge([{a:1}, {b:2}, {c:3}])", r#"{"a":1, "b":2, "c":3}"#),
            (None, "get entries({a: 2, b: 8})", r#"[{"key":"a", "value":2}, {"key":"b", "value":8}]"#),

            // test range functions
            (None, "before(1, 10)", "true"),
            (None, "before(10, 1)", "false"),
            (None, "before([1..5], 10)", "true"),
            (None, "before(1, [2..5])", "true"),
            (None, "before(3, [2..5])", "false"),

            (None, "before([1..5),[5..10])", "true"),
            (None, "before([1..5),(5..10])", "true"),
            (None, "before([1..5],[5..10])", "false"),
            (None, "before([1..5),(5..10])", "true"),

            (None, "after([5..10], [1..5))", "true"),
            (None, "after((5..10], [1..5))", "true"),
            (None, "after([5..10], [1..5])", "false"),
            (None, "after((5..10], [1..5))", "true"),

            (None, "meets([1..5], [5..10])", "true"),
            (None, "meets([1..3], [4..6])", "false"),
            (None, "meets([1..3], [3..5])", "true"),
            (None, "meets([1..5], (5..8])", "false"),

            (None, "met by([5..10], [1..5])", "true"),
            (None, "met by([3..4], [1..2])", "false"),
            (None, "met by([3..5], [1..3])", "true"),
            (None, "met by((5..8], [1..5))", "false"),
            (None, "met by([5..10], [1..5))", "false"),


            (None, "overlaps([5..10], [1..6])", "true"),
            (None, "overlaps((3..7], [1..4])", "true"),
            (None, "overlaps([1..3], (3..6])", "false"),
            (None, "overlaps((5..8], [1..5))", "false"),
            (None, "overlaps([4..10], [1..5))", "true"),

            (None, "overlaps before([1..5], [4..10])", "true"),
            (None, "overlaps before([3..4], [1..2])", "false"),
            (None, "overlaps before([1..3], (3..5])", "false"),
            (None, "overlaps before([1..5), (3..8])", "true"),
            (None, "overlaps before([1..5), [5..10])", "false"),

            (None, "overlaps after([4..10], [1..5])", "true"),
            (None, "overlaps after([3..4], [1..2])", "false"),
            (None, "overlaps after([3..5], [1..3))", "false"),
            (None, "overlaps after((5..8], [1..5))", "false"),
            (None, "overlaps after([4..10], [1..5))", "true"),

            (None, "finishes(5, [1..5])", "true"),
            (None, "finishes(10, [1..7])", "false"),
            (None, "finishes([3..5], [1..5])", "true"),
            (None, "finishes((1..5], [1..5))", "false"),
            (None, "finishes([5..10], [1..10))", "false"),

            (None, "finished by([5..10], 10)", "true"),
            (None, "finished by([3..4], 2)", "false"),

            (None, "finished by([1..5], [3..5])", "true"),
            (None, "finished by((5..8], [1..5))", "false"),
            (None, "finished by([5..10], (1..10))", "false"),

            (None, "includes([5..10], 6)", "true"),
            (None, "includes([3..4], 5)", "false"),
            (None, "includes([1..10], [4..6])", "true"),
            (None, "includes((5..8], [1..5))", "false"),
            (None, "includes([1..10], [1..5))", "true"),

            (None, "during(5, [1..10])", "true"),
            (None, "during(12, [1..10])", "false"),
            (None, "during(1, (1..10])", "false"),
            (None, "during([4..6], [1..10))", "true"),
            (None, "during((1..5], (1..10])", "true"),

            (None, "starts(1, [1..5])", "true"),
            (None, "starts(1, (1..8])", "false"),
            (None, "starts((1..5], [1..5])", "false"),
            (None, "starts([1..10], [1..10])", "true"),
            (None, "starts((1..10), (1..10))", "true"),

            (None, "started by([1..10], 1)", "true"),
            (None, "started by((1..10], 1)", "false"),
            (None, "started by([1..10], [1..5])", "true"),
            (None, "started by((1..10], [1..5))", "false"),
            (None, "started by([1..10], [1..10))", "true"),

            (None, "coincides([1..5], [1..5])", "true"),
            (None, "coincides((1..5], [1..5))", "false"),
            (None, "coincides([1..5], [2..6])", "false"),

            // temporal functions
            (None, r#"date and time("2018-04-29T09:30:00+07:00")"#, r#"date and time("2018-04-29T09:30:00+07:00")"# ),
        ];

        for (ctx, input, output) in testcases {
            let mut eng = super::Engine::new();
            //println!("parse input {input}");
            if let Some(ctx_input) = ctx {
                eng.load_context(ctx_input).unwrap();
            }
            let node = parse(input, Box::new(eng.clone())).unwrap();
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
        let node = parse(input, Box::new(eng.clone())).unwrap();
        let v = eng.eval(node).unwrap();
        assert_eq!(v.to_string(), "5.3");
    }

    #[test]
    fn test_native_func_set() {
        let mut eng = super::Engine::new();
        eng.load_context("{hi: 5}").unwrap();

        let input1 = r#"hi + 3"#;
        let node1 = parse(input1, Box::new(eng.clone())).unwrap();
        let v = eng.eval(node1).unwrap();
        assert_eq!(v.to_string(), "8");
    }

    #[test]
    fn test_func_call() {
        let mut eng = super::Engine::new();
        eng.load_context(r#"{add2: (function(a, b) a+b)}"#).unwrap();

        let input1 = r#"add2(4.5, 9)"#;
        let node1 = parse(input1, Box::new(eng.clone())).unwrap();
        let v = eng.eval(node1).unwrap();
        assert_eq!(v.to_string(), "13.5");
    }
}
