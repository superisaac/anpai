use super::super::ast::Node;
use super::super::eval::Engine;
use super::super::eval::EvalError;
use super::value::Value;

use std::cmp;
use std::collections::HashMap;
use std::fmt;

// native func
pub type NativeFuncBody =
    fn(eng: &mut Engine, args: HashMap<String, Value>) -> Result<Value, EvalError>;

#[derive(Clone, Eq)]
pub struct NativeFunc {
    pub name: String,
    pub body: NativeFuncBody,
}

impl fmt::Debug for NativeFunc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "native func `{}`", self.name)
    }
}

impl cmp::PartialEq for NativeFunc {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

// macro
pub type MacroBody =
    fn(eng: &mut Engine, nodes: HashMap<String, Box<Node>>) -> Result<Value, EvalError>;

#[derive(Clone, Eq)]
pub struct MacroT {
    pub name: String,
    pub body: MacroBody,
}

impl fmt::Debug for MacroT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "macro {}", self.name)
    }
}

impl cmp::PartialEq for MacroT {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
