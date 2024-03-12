use crate::helpers::{fmt_iter, fmt_vec};
use crate::scan::TextPosition;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct FuncCallArg {
    pub arg_name: String,
    pub arg: Box<Node>,
}

impl fmt::Display for FuncCallArg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.arg_name == "" {
            write!(f, "{}", self.arg)
        } else {
            write!(f, "{}:{}", self.arg_name, self.arg)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct MapNodeItem {
    pub name: Box<Node>,
    pub value: Box<Node>,
}

impl fmt::Display for MapNodeItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum NodeSyntax {
    BinOp {
        op: String,
        left: Box<Node>,
        right: Box<Node>,
    },

    LogicOp {
        op: String,
        left: Box<Node>,
        right: Box<Node>,
    },

    DotOp {
        left: Box<Node>,
        attr: String,
    },

    // function calling
    FuncCall {
        func_ref: Box<Node>,
        args: Vec<FuncCallArg>,
    },

    // function defination
    FuncDef {
        arg_names: Vec<String>,
        body: Box<Node>,
    },

    // variable
    Var(String),

    // ident, used in map key
    Ident(String),

    Number(String),

    Bool(bool),

    Null,

    Str(String),

    Temporal(String),

    Neg(Box<Node>),

    Array(Vec<Box<Node>>),

    Map(Vec<MapNodeItem>),

    Range {
        start_open: bool,
        start: Box<Node>,
        end_open: bool,
        end: Box<Node>,
    },

    IfExpr {
        condition: Box<Node>,
        then_branch: Box<Node>,
        else_branch: Box<Node>,
    },

    ForExpr {
        var_name: String,
        list_expr: Box<Node>,
        return_expr: Box<Node>,
    },

    SomeExpr {
        var_name: String,
        list_expr: Box<Node>,
        filter_expr: Box<Node>,
    },

    EveryExpr {
        var_name: String,
        list_expr: Box<Node>,
        filter_expr: Box<Node>,
    },

    ExprList(Vec<Node>),

    MultiTests(Vec<Node>),
}

impl fmt::Display for NodeSyntax {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::BinOp { op, left, right } => write!(f, "({} {} {})", op, left, right),
            Self::LogicOp { op, left, right } => write!(f, "({} {} {})", op, left, right),
            Self::DotOp { left, attr } => write!(f, "(. {} {})", left, attr),
            Self::FuncCall { func_ref, args } => write!(f, "(call {} ", func_ref)
                .and_then(|_| fmt_vec(f, args.iter(), "[", "]"))
                .and_then(|_| write!(f, "{}", ")")),
            Self::FuncDef { arg_names, body } => write!(f, "(function ")
                .and_then(|_| fmt_vec(f, arg_names.iter(), "[", "]"))
                .and_then(|_| write!(f, " {})", body)),
            Self::Var(name) => write!(f, "{}", name),
            Self::Ident(name) => write!(f, "{}", name),
            Self::Number(value) => write!(f, "{}", value),
            Self::Bool(value) => write!(f, "{}", value),
            Self::Null => write!(f, "null"),
            Self::Str(value) => write!(f, "{}", value),
            Self::Temporal(value) => write!(f, "{}", value),
            Self::Neg(value) => write!(f, "(- {})", value),
            Self::Range {
                start_open,
                start,
                end_open,
                end,
            } => {
                let start_bra = if *start_open { "(" } else { "[" };
                let end_bra = if *end_open { ")" } else { "]" };
                write!(f, "{}{}..{}{}", start_bra, start, end, end_bra)
            }
            Self::Array(elements) => fmt_vec(f, elements.iter(), "[", "]"),
            Self::Map(items) => fmt_vec(f, items.iter(), "{", "}"),
            Self::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => write!(f, "(if {} {} {})", condition, then_branch, else_branch),
            Self::ForExpr {
                var_name,
                list_expr,
                return_expr,
            } => write!(f, "(for {} in {} {})", var_name, list_expr, return_expr),
            Self::SomeExpr {
                var_name,
                list_expr,
                filter_expr,
            } => write!(
                f,
                "(some {} in {} satisfies {})",
                var_name, list_expr, filter_expr
            ),
            Self::EveryExpr {
                var_name,
                list_expr,
                filter_expr,
            } => write!(
                f,
                "(every {} in {} satisfies {})",
                var_name, list_expr, filter_expr
            ),
            Self::ExprList(elements) => fmt_iter(f, elements.iter(), " ", "(expr-list ", ")"),
            Self::MultiTests(elements) => fmt_iter(f, elements.iter(), " ", "(multi-tests ", ")"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Node {
    pub syntax: Box<NodeSyntax>,
    pub start_pos: TextPosition,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.syntax)
    }
}

impl Node {
    pub fn new(syntax: NodeSyntax, start_pos: TextPosition) -> Box<Node> {
        Box::new(Node {
            syntax: Box::new(syntax),
            start_pos,
        })
    }
}
