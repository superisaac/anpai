use std::collections::HashMap;
use std::fmt;
use std::cmp;
use crate::eval::Intepreter;
use crate::values::value::Value;
use crate::eval::EvalError;
use crate::ast::Node;

// native func
pub type NativeFunc =
    fn(intp: &mut Intepreter, args: HashMap<String, Value>) -> Result<Value, EvalError>;

#[derive(Clone)]
pub struct NativeFuncT(pub NativeFunc);
impl fmt::Debug for NativeFuncT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "native func: {}", self.0 as usize)
    }
}

impl cmp::PartialEq for NativeFuncT {
    fn eq(&self, other: &Self) -> bool {
        self.0 as usize == other.0 as usize
    }
}

// macro
pub type MacroCb =
    fn(intp: &mut Intepreter, nodes: HashMap<String, Box<Node>>) -> Result<Value, EvalError>;

#[derive(Clone)]
pub struct MacroCbT(pub MacroCb);

impl fmt::Debug for MacroCbT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "macro {}", self.0 as usize)
    }
}

impl cmp::PartialEq for MacroCbT {
    fn eq(&self, other: &Self) -> bool {
        self.0 as usize == other.0 as usize
    }
}