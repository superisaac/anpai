use crate::ast::{FuncCallArg, MapNodeItem, Node, NodeSyntax::*, VarValue};
use crate::eval::Engine;
use crate::helpers::find_duplicate;
use crate::scan::{ScanError, Scanner, TextPosition, Token};

use std::backtrace::Backtrace;
use std::error::Error;
use std::fmt;

/// parse FEEL refer to https://www.omg.org/spec/DMN/1.2/PDF

#[derive(Default)]
pub enum ParseTop {
    #[default]
    Expression,
    UnaryTests,
}

// Parse error
#[derive(Debug, Clone)]
pub enum ParseError {
    Parse(String),
    Scan(ScanError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::Parse(message) => write!(f, "ParseError: {}", message),
            ParseError::Scan(err) => write!(f, "{}", err),
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Parse(_) => None,
            Self::Scan(ref err) => Some(err),
        }
    }
}

impl From<ScanError> for ParseError {
    fn from(err: ScanError) -> ParseError {
        Self::Scan(err)
    }
}

impl ParseError {
    pub fn new(message: String) -> ParseError {
        Self::Parse(message)
    }
}

type NodeResult = Result<Box<Node>, ParseError>;

pub struct Parser<'a> {
    scanner: Box<Scanner<'a>>,
    engine: Box<Engine>,
}

// shortcuts to go ahead one token
macro_rules! goahead {
    ($parser:ident) => {
        let _ = $parser.scanner.next_token()?;
    };
}

impl Parser<'_> {
    pub fn new<'a>(input: &str, engine: Box<Engine>) -> Parser {
        let scanner = Scanner::new(input);
        Parser {
            scanner: Box::new(scanner),
            engine,
        }
    }

    fn unexpect(&self, expects: &str) -> ParseError {
        let bt = Backtrace::force_capture();
        let mut stack_str = String::new();
        for (i, frame) in bt.frames().iter().enumerate() {
            if i > 0 {
                stack_str.push_str("\n");
            }
            stack_str.push_str(format!("{:?}", frame).as_str());
        }
        //let stack_str = format!("{:?}", bt);

        ParseError::new(format!(
            "unexpected token {}, expect {}, stack {}",
            self.scanner.current_token().kind,
            expects,
            stack_str,
        ))
    }

    fn unexpect_keyword(&self, expects: &str) -> ParseError {
        ParseError::new(format!(
            "unexpected keyword {}, expect {}",
            self.scanner.current_token().value,
            expects
        ))
    }

    pub fn parse(&mut self, top: ParseTop) -> NodeResult {
        goahead!(self);
        match top {
            ParseTop::Expression => self.parse_expression(),
            ParseTop::UnaryTests => self.parse_unary_tests(),
        }
    }

    fn parse_unary_tests(&mut self) -> NodeResult {
        let start_pos = self.scanner.current_token().position;
        let elem = self.parse_unary_test()?;
        let mut elements = Vec::new();
        elements.push(elem);

        if self.scanner.expect(",") {
            while self.scanner.expect(",") {
                goahead!(self); // skip ','
                let elem1 = self.parse_unary_test()?;
                elements.push(elem1);
            }
            //Ok(Node::new(UnaryTests(elements), start_pos))
        }
        Ok(Node::new(UnaryTests(elements), start_pos))
    }

    fn parse_unary_test(&mut self) -> NodeResult {
        if self
            .scanner
            .expect_kinds(&[">", ">=", "<", "<=", "!=", "="])
        {
            let op = self.scanner.current_token().kind;
            goahead!(self); // skip op
            let start_pos = self.scanner.current_token().position;
            let right = self.parse_expression()?;
            //let left = Node::new(Var(VarValue::Name("?".to_owned())), start_pos.clone());
            Ok(Node::new(
                UnaryTest {
                    op: op.to_string(),
                    right,
                },
                start_pos,
            ))
        } else {
            let start_pos = self.scanner.current_token().position;
            let right = self.parse_expression()?;
            match *right.syntax {
                Var(_) | Number(_) | Str(_) | Ident(_) | Null | Bool(_) | Temporal(_) | Neg(_) => {
                    Ok(Node::new(
                        UnaryTest {
                            op: "=".to_string(),
                            right,
                        },
                        start_pos,
                    ))
                }
                _ => Ok(right),
            }
        }
    }

    fn parse_expression(&mut self) -> NodeResult {
        self.parse_in_op(Parser::parse_logic_or)
    }

    fn parse_in_op(&mut self, sub_func: fn(&mut Self) -> NodeResult) -> NodeResult {
        let mut start_pos = self.scanner.current_token().position;
        let mut left = sub_func(self)?;
        while self.scanner.expect_keyword("in") {
            goahead!(self);
            let right = sub_func(self)?;
            left = Node::new(InOp { left, right }, start_pos.clone());
            start_pos = self.scanner.current_token().position;
        }
        Ok(left)
    }

    fn parse_binop_kinds(
        &mut self,
        kinds: &[&str],
        sub_parse: fn(&mut Self) -> NodeResult,
    ) -> NodeResult {
        let mut start_pos = self.scanner.current_token().position;
        let mut left = sub_parse(self)?;
        while self.scanner.expect_kinds(kinds) {
            let op = self.scanner.current_token().value;
            goahead!(self);
            let right = sub_parse(self)?;
            left = Node::new(BinOp { op, left, right }, start_pos);
            start_pos = self.scanner.current_token().position;
        }
        Ok(left)
    }

    // logic ops
    fn parse_logicop_keywords(
        &mut self,
        keywords: &[&str],
        sub_func: fn(&mut Self) -> NodeResult,
    ) -> NodeResult {
        let mut start_pos = self.scanner.current_token().position;
        let mut left = sub_func(self)?;
        while self.scanner.expect_keywords(keywords) {
            let op = self.scanner.current_token().value;
            goahead!(self);
            let right = sub_func(self)?;
            left = Node::new(LogicOp { op, left, right }, start_pos);
            start_pos = self.scanner.current_token().position;
        }
        Ok(left)
    }

    fn parse_logic_or(&mut self) -> NodeResult {
        self.parse_logicop_keywords(&["or"], Parser::parse_logic_and)
    }

    fn parse_logic_and(&mut self) -> NodeResult {
        self.parse_logicop_keywords(&["and"], Parser::parse_compare)
    }

    fn parse_compare(&mut self) -> NodeResult {
        self.parse_binop_kinds(&[">", ">=", "<", "<=", "!=", "="], Parser::parse_add_or_sub)
    }

    fn parse_add_or_sub(&mut self) -> NodeResult {
        self.parse_binop_kinds(&["+", "-"], Parser::parse_mul_or_div)
    }

    fn parse_mul_or_div(&mut self) -> NodeResult {
        self.parse_binop_kinds(&["*", "/", "%"], Parser::parse_funccall_or_index_or_dot)
    }

    fn parse_funccall_or_index_or_dot(&mut self) -> NodeResult {
        let mut node = self.parse_single_element()?;
        loop {
            match self.scanner.current_token().kind {
                "(" => {
                    node = self.parse_func_call_rest(node)?;
                }
                "[" => {
                    node = self.parse_index_rest(node)?;
                }
                "." => {
                    node = self.parse_dot_rest(node)?;
                }
                _ => break,
            }
        }
        Ok(node)
    }

    fn parse_func_call_rest(&mut self, func_node: Box<Node>) -> NodeResult {
        goahead!(self); // skip "("
        let start_pos = func_node.clone().start_pos;
        let mut args: Vec<FuncCallArg> = Vec::new();
        while !self.scanner.expect(")") {
            let arg = self.parse_funcall_arg()?;
            args.push(arg);
            if self.scanner.expect(",") {
                goahead!(self);
            } else if !self.scanner.expect(")") {
                return Err(self.unexpect(") and ,"));
            }
        }
        if self.scanner.expect(")") {
            goahead!(self);
        }
        Ok(Node::new(
            FuncCall {
                func_ref: func_node,
                args,
            },
            start_pos,
        ))
    }

    fn parse_funcall_arg(&mut self) -> Result<FuncCallArg, ParseError> {
        let arg = self.parse_expression()?;
        if self.scanner.expect(":") {
            goahead!(self);
            if let Var(v) = *arg.syntax {
                goahead!(self); // skip ":"
                let arg_value = self.parse_expression()?;
                return Ok(FuncCallArg {
                    arg_name: v.value(),
                    arg: arg_value,
                });
            } else {
                return Err(self.unexpect("'var'"));
            }
        } else {
            return Ok(FuncCallArg {
                arg_name: "".to_owned(),
                arg,
            });
        }
    }

    fn parse_index_rest(&mut self, left: Box<Node>) -> NodeResult {
        goahead!(self); // skip "["
        let start_pos = left.clone().start_pos;
        let at = self.parse_expression()?;
        if !self.scanner.expect("]") {
            return Err(self.unexpect("]"));
        }
        goahead!(self);
        return Ok(Node::new(
            BinOp {
                op: "[]".to_owned(),
                left,
                right: at,
            },
            start_pos,
        ));
    }

    fn parse_dot_rest(&mut self, left: Box<Node>) -> NodeResult {
        goahead!(self); // skip "."
        let start_pos = left.clone().start_pos;
        let attr = self.parse_name(None)?;
        return Ok(Node::new(DotOp { left, attr }, start_pos));
    }

    // single element
    fn parse_single_element(&mut self) -> NodeResult {
        match self.scanner.current_token().kind {
            "number" => self.parse_number(),
            "name" => self.parse_var(),
            "backtick" => self.parse_backtick(),
            "string" => self.parse_string(),
            "temporal" => self.parse_temporal(),
            "-" => self.parse_neg(),
            "{" => self.parse_map(),
            "(" => self.parse_bracket_or_range(),
            "[" => self.parse_range_or_array(),
            ">" | ">=" | "<" | "<=" | "!=" | "=" => self.parse_unary_test(),
            "keyword" => match self.scanner.current_token().value.as_str() {
                "true" | "false" => self.parse_bool(),
                "null" => self.parse_null(),
                "if" => self.parse_if_expression(),
                "for" => self.parse_for_expression(),
                "some" | "every" => self.parse_some_or_every_expression(),
                "function" => self.parse_function_defination(),
                _ => {
                    return Err(self.unexpect_keyword("true, false, if, for, some, every, function"))
                }
            },
            _ => return Err(self.unexpect("name, number")),
        }
    }

    fn parse_name(&mut self, stop_keywords: Option<&[&str]>) -> Result<String, ParseError> {
        let mut token_stack: Vec<Token> = Vec::new();

        while self
            .scanner
            .expect_kinds(&["name", "keyword", "+", "-", "*", "/"])
        {
            let token = self.scanner.current_token();
            if let ("keyword", Some(stop_keywords)) = (token.kind, stop_keywords) {
                let token_keyword = token.value.as_str();
                if stop_keywords.into_iter().any(|x| *x == token_keyword) {
                    break;
                }
            }
            token_stack.push(token);
            goahead!(self);
        }
        while token_stack.len() > 0 {
            let mut name_buffer = String::new();
            let mut found_op = false;
            for (i, t) in token_stack.iter().enumerate() {
                if t.kind != "keyword" && t.kind != "name" {
                    found_op = true;
                }
                if i > 0
                    && (token_stack[i - 1].position.chars + token_stack[i - 1].value.len()
                        < t.position.chars)
                {
                    name_buffer.push_str(" ");
                }
                name_buffer.push_str(t.value.as_str());
            }
            if !found_op || self.engine.has_name(name_buffer.clone()) {
                return Ok(name_buffer);
            }
            if let Some(token) = token_stack.pop() {
                self.scanner.rewind(token);
            }
        }

        Err(self.unexpect("names"))
    }

    fn parse_var_name(&mut self, stop_keywords: Option<&[&str]>) -> Result<String, ParseError> {
        let mut token_stack: Vec<Token> = Vec::new();
        if self.scanner.expect("backtick") {
            let t = self.scanner.current_token();
            goahead!(self);
            return Ok(t.value);
        }
        while self
            .scanner
            .expect_kinds(&["name", "keyword", "+", "-", "*", "/"])
        {
            let token = self.scanner.current_token();
            if let ("keyword", Some(stop_keywords)) = (token.kind, stop_keywords) {
                let token_keyword = token.value.as_str();
                if stop_keywords.into_iter().any(|x| *x == token_keyword) {
                    break;
                }
            }
            token_stack.push(token);
            goahead!(self);
        }
        if token_stack.len() > 0 {
            let mut name_buffer = String::new();
            for (i, t) in token_stack.iter().enumerate() {
                if i > 0
                    && (token_stack[i - 1].position.chars + token_stack[i - 1].value.len()
                        < t.position.chars)
                {
                    name_buffer.push_str(" ");
                }
                name_buffer.push_str(t.value.as_str());
            }
            return Ok(name_buffer);
        }

        Err(self.unexpect("names"))
    }

    fn parse_var(&mut self) -> NodeResult {
        let start_pos = self.scanner.current_token().position;
        let var_name = self.parse_name(None)?;
        // let token = self.scanner.current_token();
        // goahead!(self);
        Ok(Node::new(Var(VarValue::Name(var_name)), start_pos))
    }

    fn parse_backtick(&mut self) -> NodeResult {
        let token = self.scanner.current_token();
        goahead!(self);
        Ok(Node::new(
            Var(VarValue::Backtick(token.value)),
            token.position,
        ))
        //Ok(Node::new(Str(token.value), token.position))
    }

    fn parse_number(&mut self) -> NodeResult {
        let token = self.scanner.current_token();
        goahead!(self);
        Ok(Node::new(Number(token.value), token.position))
    }

    fn parse_neg(&mut self) -> NodeResult {
        goahead!(self); // skip '-'
        let start_pos = self.scanner.current_token().position;
        let node = self.parse_expression()?;
        Ok(Node::new(Neg(node), start_pos))
    }

    fn parse_string(&mut self) -> NodeResult {
        let token = self.scanner.current_token();
        goahead!(self);
        Ok(Node::new(Str(token.value), token.position))
    }

    fn parse_temporal(&mut self) -> NodeResult {
        let token = self.scanner.current_token();
        goahead!(self);
        Ok(Node::new(Temporal(token.value), token.position))
    }

    fn parse_bool(&mut self) -> NodeResult {
        let token = self.scanner.current_token();
        let bool_value = match token.value.as_str() {
            "true" => true,
            "false" => false,
            _ => return Err(self.unexpect_keyword("true, false")),
        };
        goahead!(self);
        Ok(Node::new(Bool(bool_value), token.position))
    }

    fn parse_null(&mut self) -> NodeResult {
        let start_pos = self.scanner.current_token().position;
        goahead!(self); // skip 'null'
        Ok(Node::new(Null, start_pos))
    }

    // parse map/context defination
    fn parse_map(&mut self) -> NodeResult {
        let start_pos = self.scanner.current_token().position;
        goahead!(self); // skip '{'
        let mut items = Vec::new();
        while !self.scanner.expect("}") {
            let mapkey = self.parse_map_key()?;

            if !self.scanner.expect(":") {
                return Err(self.unexpect(":"));
            }
            goahead!(self); // skip ':'

            let exp = self.parse_expression()?;
            items.push(MapNodeItem {
                name: mapkey,
                value: exp,
            });

            if self.scanner.expect(",") {
                goahead!(self); // skip ','
            } else if !self.scanner.expect("}") {
                return Err(self.unexpect("'}', ','"));
            }
        }

        if self.scanner.expect("}") {
            goahead!(self); // skip '}'
        }
        Ok(Node::new(Map(items), start_pos))
    }

    fn parse_map_key(&mut self) -> NodeResult {
        if self.scanner.expect_kinds(&["name", "backtick"]) {
            let start_pos = self.scanner.current_token().position;
            match self.parse_var_name(None) {
                Ok(name) => Ok(Node::new(Ident(name), start_pos)),
                Err(err) => Err(err),
            }
        } else if self.scanner.expect("string") {
            self.parse_string()
        } else {
            return Err(self.unexpect("name or string"));
        }
    }

    fn parse_range_given_start(
        &mut self,
        start_open: bool,
        start_exp: Box<Node>,
        start_pos: TextPosition,
    ) -> NodeResult {
        let end_exp = self.parse_expression()?;
        if self.scanner.expect(")") {
            // open end range
            goahead!(self); //skip ')'
            return Ok(Node::new(
                Range {
                    start_open,
                    start: start_exp,
                    end_open: true,
                    end: end_exp,
                },
                start_pos,
            ));
        } else if self.scanner.expect("]") {
            // close end range
            goahead!(self); //skip ')'
            return Ok(Node::new(
                Range {
                    start_open,
                    start: start_exp,
                    end_open: false,
                    end: end_exp,
                },
                start_pos,
            ));
        } else {
            return Err(self.unexpect("')', ']'"));
        }
    }

    fn parse_bracket_or_range(&mut self) -> NodeResult {
        let start_pos = self.scanner.current_token().position;
        goahead!(self); // skip '('
        let aexp = self.parse_expression()?;
        if self.scanner.expect("..") {
            // is range
            goahead!(self); // skip '..'
            return self.parse_range_given_start(true, aexp, start_pos);
        } else if self.scanner.expect(")") {
            goahead!(self); // skip ')'
            return Ok(aexp);
        } else if self.scanner.expect(",") {
            //goahead!(self);
            return self.parse_expr_list_given_first(aexp, start_pos);
        } else {
            return Err(self.unexpect("')', ',', '..'"));
        }
    }

    fn parse_range_or_array(&mut self) -> NodeResult {
        let start_pos = self.scanner.current_token().position;
        goahead!(self); // skip '['
        if self.scanner.expect("]") {
            goahead!(self); // skip ']'
            return Ok(Node::new(Array(Vec::new()), start_pos));
        }
        let aexp = self.parse_expression()?;
        if self.scanner.expect_kinds(&[",", "]"]) {
            return self.parse_array_given_first(aexp, start_pos);
        }

        if !self.scanner.expect("..") {
            return Err(self.unexpect("'..'"));
        }
        goahead!(self); // skip '..'

        return self.parse_range_given_start(false, aexp, start_pos);
    }

    fn parse_expr_list_given_first(
        &mut self,
        first_element: Box<Node>,
        start_pos: TextPosition,
    ) -> NodeResult {
        let mut elements = Vec::new();
        elements.push(first_element);

        while self.scanner.expect(",") {
            goahead!(self); // skip ','
            let elem = self.parse_expression()?;
            elements.push(elem);
        }
        if !self.scanner.expect(")") {
            return Err(self.unexpect("')'"));
        }
        goahead!(self); // skip ')'
        if elements.len() <= 1 {
            Ok(elements[0].clone())
        } else {
            Ok(Node::new(ExprList(elements), start_pos))
        }
    }

    fn parse_array_given_first(
        &mut self,
        first_element: Box<Node>,
        start_pos: TextPosition,
    ) -> NodeResult {
        let mut elements = Vec::new();
        elements.push(first_element);

        while self.scanner.expect(",") {
            goahead!(self); // skip ','
            let elem = self.parse_expression()?;
            elements.push(elem);
        }
        if !self.scanner.expect("]") {
            return Err(self.unexpect("']'"));
        }
        goahead!(self); // skip ']'
        Ok(Node::new(Array(elements), start_pos))
    }

    // if expression
    fn parse_if_expression(&mut self) -> NodeResult {
        let start_pos = self.scanner.current_token().position;
        goahead!(self); // skip 'if'
        let cond = self.parse_expression()?;
        if !self.scanner.expect_keyword("then") {
            return Err(self.unexpect_keyword("then"));
        }
        goahead!(self); // skip 'then'

        let then_branch = self.parse_expression()?;
        if !self.scanner.expect_keyword("else") {
            return Err(self.unexpect_keyword("else"));
        }
        goahead!(self); // skip 'else'

        let else_branch = self.parse_expression()?;
        Ok(Node::new(
            IfExpr {
                condition: cond,
                then_branch,
                else_branch,
            },
            start_pos,
        ))
    }

    fn parse_for_expression(&mut self) -> NodeResult {
        let start_pos = self.scanner.current_token().position;
        goahead!(self); // skip 'for'
        let var_name = self.parse_var_name(Some(&["in", "for"]))?;
        if !self.scanner.expect_keyword("in") {
            return Err(self.unexpect_keyword("in"));
        }
        goahead!(self); // skip 'in'

        let list_expr = self.parse_expression()?;
        if self.scanner.expect(",") {
            // recursively call for parser
            let return_expr = self.parse_for_expression()?;
            return Ok(Node::new(
                ForExpr {
                    var_name,
                    list_expr,
                    return_expr,
                },
                start_pos,
            ));
        }

        if !self.scanner.expect_keyword("return") {
            return Err(self.unexpect_keyword("return"));
        }
        goahead!(self); // skip 'return'

        let return_expr = self.parse_expression()?;
        Ok(Node::new(
            ForExpr {
                var_name,
                list_expr,
                return_expr,
            },
            start_pos,
        ))
    }

    fn parse_some_or_every_expression(&mut self) -> NodeResult {
        let start_pos = self.scanner.current_token().position;
        let cmd = self.scanner.current_token().value;
        goahead!(self); // skip 'some'|'every'
        let var_name = self.parse_var_name(Some(&["in"]))?;

        if !self.scanner.expect_keyword("in") {
            return Err(self.unexpect_keyword("in"));
        }
        goahead!(self); // skip 'in'

        let list_expr = self.parse_expression()?;
        if !self.scanner.expect_keyword("satisfies") {
            return Err(self.unexpect_keyword("satisfies"));
        }
        goahead!(self); // skip 'satisfies'

        let filter_expr = self.parse_expression()?;
        if cmd == "some".to_owned() {
            Ok(Node::new(
                SomeExpr {
                    var_name,
                    list_expr,
                    filter_expr,
                },
                start_pos,
            ))
        } else {
            Ok(Node::new(
                EveryExpr {
                    var_name,
                    list_expr,
                    filter_expr,
                },
                start_pos,
            ))
        }
    }

    fn parse_function_defination(&mut self) -> NodeResult {
        let start_pos = self.scanner.current_token().position;
        goahead!(self); // skip 'function'
        if !self.scanner.expect("(") {
            return Err(self.unexpect("'('"));
        }
        goahead!(self); // skip '('

        let mut arg_names = Vec::new();
        while !self.scanner.expect(")") {
            let arg_name = self.parse_name(None)?;
            arg_names.push(arg_name);

            if let Some(dup_arg_name) = find_duplicate(&arg_names) {
                return Err(ParseError::Parse(format!(
                    "function has duplication arg name `{}`",
                    dup_arg_name
                )));
            }
            if self.scanner.expect(",") {
                goahead!(self); // skip ','
            } else if !self.scanner.expect(")") {
                return Err(self.unexpect("')'"));
            }
        }
        // TODO: check duplicate names
        if !self.scanner.expect(")") {
            return Err(self.unexpect("')'"));
        }
        goahead!(self); // skip ')'

        let exp = self.parse_expression()?;
        let end_pos = self.scanner.current_token().position;
        let func_code = self.scanner.text_range(start_pos.chars, end_pos.chars);
        Ok(Node::new(
            FuncDef {
                arg_names,
                body: exp,
                code: func_code.to_owned(),
            },
            start_pos,
        ))
    }
}

pub fn parse(
    input: &str,
    engine: Box<Engine>,
    top: ParseTop,
) -> Result<Box<Node>, (ParseError, TextPosition)> {
    let mut parser = Parser::new(input, engine);
    match parser.parse(top) {
        Ok(n) => Ok(n),
        Err(err) => Err((err, parser.scanner.current_token().position)),
    }
}

#[cfg(test)]
mod test {
    use crate::eval::Engine;
    use core::assert_matches::assert_matches;
    #[test]
    fn test_parse_expressions() {
        let testcases = [
            ("a + b(4, 9)", "(+ a (call b [4, 9]))"),
            ("if a > 6 then true else false", "(if (> a 6) true false)"),
            ("{a: 1, \"bbb\": [2, 1]}", r#"{a: 1, "bbb": [2, 1]}"#),
            //("> 2, <= 1, a>8", "(unary-tests (> ? 2) (<= ? 1) (> a 8))"),
            //("2>8; 9; true", "(expr-list (> 2 8) 9 true)"),
        ];

        for (input, output) in testcases {
            let engine = Box::new(Engine::new());
            let node = super::parse(input, engine, Default::default()).unwrap();
            assert_eq!(
                format!("{}", *node),
                output,
                "output {} mismatch input {}",
                output,
                input
            );
        }
    }

    #[test]
    fn test_parse_unary_tests() {
        let testcases = [
            ("> 2, <= 1, a>8", "(unary-tests (> 2) (<= 1) (> a 8))"),
            //("2>8; 9; true", "(expr-list (> 2 8) 9 true)"),
        ];

        for (input, output) in testcases {
            let engine = Box::new(Engine::new());
            let node = super::parse(input, engine, super::ParseTop::UnaryTests).unwrap();
            assert_eq!(
                format!("{}", *node),
                output,
                "output {} mismatch input {}",
                output,
                input
            );
        }
    }

    #[test]
    fn test_parse_func_def() {
        let input = "function(a, b) a + b   ";
        let engine = Box::new(Engine::new());
        let node = super::parse(input, engine, Default::default()).unwrap();
        assert_matches!(
            *(node.syntax),
            crate::ast::NodeSyntax::FuncDef {
                arg_names: _,
                body: _,
                code: _
            }
        );
        if let crate::ast::NodeSyntax::FuncDef {
            arg_names: _,
            body: _,
            code: c,
        } = *node.syntax
        {
            assert_eq!(c.as_str(), "function(a, b) a + b   ");
        }
    }

    #[test]
    fn test_parse_dup_arg_name() {
        let engine = Box::new(Engine::new());
        let res = super::parse("function(a, b, a) a+ b", engine, Default::default());
        assert_matches!(res, Err((super::ParseError::Parse(x), _)) if x == "function has duplication arg name `a`".to_owned());
    }
}
