use std::fmt;

#[derive(Clone)]
struct FuncCallArg {
    arg_name: String,
    arg: Box<Node>,
}

impl fmt::Display for FuncCallArg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.arg_name, self.arg)
    }
}

fn fmt_vec<T: fmt::Display>(vec: &Vec<T>, f: &mut fmt::Formatter) -> fmt::Result {
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
    Ok(())
}

#[derive(Clone)]
struct MapItem {
    name: String,
    value: Box<Node>,
}
impl fmt::Display for MapItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.name, self.value)
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
    Var {
        name: String,
    },

    Number {
        value: String,
    },

    Bool {
        value: bool,
    },

    Null,

    String {
        value: String,
    },

    Temporal {
        value: String,
    },

    Array {
        elements: Vec<Node>,
    },

    Map {
        items: Vec<MapItem>,
    },

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
            Node::Binop { op, left, right } => write!(f, "({} {} {})", op, left, right),
            Node::DotOp { left, attr } => write!(f, "(. {} {})", left, attr),
            Node::FuncCall { func_ref, args } => {
                write!(f, "(call {} ", func_ref).and_then(|_| fmt_vec(args, f))
            }
            Node::FuncDef { args, body } => write!(f, "(function [")
                .and_then(|_| fmt_vec(args, f))
                .and_then(|_| write!(f, "] {})", body)),
            Node::Var { name } => write!(f, "{}", name),
            Node::Number { value } => write!(f, "{}", value),
            Node::Bool { value } => write!(f, "{}", value),
            Node::Null => write!(f, "null"),
            Node::String { value } => write!(f, "\"{}\"", value),
            Node::Temporal { value } => write!(f, "{}", value),
            Node::Range {
                start_open,
                start,
                end_open,
                end,
            } => {
                let start_bra = if *start_open { "(" } else { "[" };
                let end_bra = if *end_open { ")" } else { "]" };
                write!(f, "{}{}..{}{}", start_bra, start, end, end_bra)
            }
            Node::Array { elements } => fmt_vec(elements, f),
            Node::Map { items } => fmt_vec(items, f),
            Node::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => write!(f, "(if {} {} {})", condition, then_branch, else_branch),
            Node::ForExpr {
                var_name,
                list_expr,
                return_expr,
            } => write!(f, "(for {} in {} {})", var_name, list_expr, return_expr),
            Node::SomeExpr {
                var_name,
                list_expr,
                filter_expr,
            } => write!(
                f,
                "(some {} in {} satisfies {})",
                var_name, list_expr, filter_expr
            ),
            Node::EveryExpr {
                var_name,
                list_expr,
                filter_expr,
            } => write!(
                f,
                "(every {} in {} satisfies {})",
                var_name, list_expr, filter_expr
            ),
            Node::ExprList { elements } => fmt_vec(elements, f),
            Node::MultiTests { elements } => fmt_vec(elements, f),
        }
    }
}
