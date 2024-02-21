use crate::ast::Node::*;
use crate::helpers::fmt_vec;
use std::fmt;

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct MapNodeItem {
    pub name: Box<Node>,
    pub value: Box<Node>,
}

impl fmt::Display for MapNodeItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.value)
    }
}

#[derive(Clone, Debug)]
pub enum Node {
    Binop {
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

    ExprList {
        elements: Vec<Node>,
    },

    MultiTests {
        elements: Vec<Node>,
    },
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Binop { op, left, right } => write!(f, "({} {} {})", op, left, right),
            DotOp { left, attr } => write!(f, "(. {} {})", left, attr),
            FuncCall { func_ref, args } => write!(f, "(call {} ", func_ref)
                .and_then(|_| fmt_vec(f, args.iter(), "[", "]"))
                .and_then(|_| write!(f, "{}", ")")),
            FuncDef { arg_names, body } => write!(f, "(function ")
                .and_then(|_| fmt_vec(f, arg_names.iter(), "[", "]"))
                .and_then(|_| write!(f, " {})", body)),
            Var(name) => write!(f, "{}", name),
            Ident(name) => write!(f, "{}", name),
            Number(value) => write!(f, "{}", value),
            Bool(value) => write!(f, "{}", value),
            Null => write!(f, "null"),
            Str(value) => write!(f, "{}", value),
            Temporal(value) => write!(f, "{}", value),
            Neg(value) => write!(f, "(- {})", value),
            Range {
                start_open,
                start,
                end_open,
                end,
            } => {
                let start_bra = if *start_open { "(" } else { "[" };
                let end_bra = if *end_open { ")" } else { "]" };
                write!(f, "{}{}..{}{}", start_bra, start, end, end_bra)
            }
            Array(elements) => fmt_vec(f, elements.iter(), "[", "]"),
            Map(items) => fmt_vec(f, items.iter(), "{", "}"),
            IfExpr {
                condition,
                then_branch,
                else_branch,
            } => write!(f, "(if {} {} {})", condition, then_branch, else_branch),
            ForExpr {
                var_name,
                list_expr,
                return_expr,
            } => write!(f, "(for {} in {} {})", var_name, list_expr, return_expr),
            SomeExpr {
                var_name,
                list_expr,
                filter_expr,
            } => write!(
                f,
                "(some {} in {} satisfies {})",
                var_name, list_expr, filter_expr
            ),
            EveryExpr {
                var_name,
                list_expr,
                filter_expr,
            } => write!(
                f,
                "(every {} in {} satisfies {})",
                var_name, list_expr, filter_expr
            ),
            ExprList { elements } => fmt_vec(f, elements.iter(), "", ""),
            MultiTests { elements } => fmt_vec(f, elements.iter(), "", ""),
        }
    }
}
