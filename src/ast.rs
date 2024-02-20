use crate::ast::Node::*;
use std::fmt;

#[derive(Clone)]
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

fn fmt_vec<T: fmt::Display>(
    f: &mut fmt::Formatter,
    vec: &Vec<T>,
    prefix: &str,
    suffix: &str,
) -> fmt::Result {
    match write!(f, "{}", prefix) {
        Err(err) => return Err(err),
        _ => (),
    }
    for (i, arg) in vec.iter().enumerate() {
        if i > 0 {
            match write!(f, ", {}", arg) {
                Err(err) => return Err(err),
                _ => (),
            }
        } else {
            match write!(f, "{}", arg) {
                Err(err) => return Err(err),
                _ => (),
            }
        }
    }
    match write!(f, "{}", suffix) {
        Err(err) => return Err(err),
        _ => (),
    }
    Ok(())
}

#[derive(Clone)]
pub struct MapNodeItem {
    pub name: Box<Node>,
    pub value: Box<Node>,
}

impl fmt::Display for MapNodeItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.value)
    }
}

#[derive(Clone)]
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
        args: Vec<String>,
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

    Array(Vec<Node>),

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
                .and_then(|_| fmt_vec(f, args, "[", "]"))
                .and_then(|_| write!(f, "{}", ")")),
            FuncDef { args, body } => write!(f, "(function ")
                .and_then(|_| fmt_vec(f, args, "[", "]"))
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
            Array(elements) => fmt_vec(f, elements, "[", "]"),
            Map(items) => fmt_vec(f, items, "{", "}"),
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
            ExprList { elements } => fmt_vec(f, elements, "", ""),
            MultiTests { elements } => fmt_vec(f, elements, "", ""),
        }
    }
}
