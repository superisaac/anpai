use crate::ast::{
    FuncCallArg, MapNodeItem,
    Node::{self, *},
};
use crate::scan::{ScanError, Scanner};
use std::error::Error;
use std::fmt;

// Parse error
#[derive(Debug)]
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
}

// shortcuts to go ahead one token
macro_rules! goahead {
    ($parser:ident) => {
        let _ = $parser.scanner.next_token()?;
        // if let Err(err) = $parser.scanner.next_token() {
        //     return Err(err);
        // }
    };
}

impl Parser<'_> {
    pub fn new(input: &str) -> Parser {
        let scanner = Scanner::new(input);
        Parser {
            scanner: Box::new(scanner),
        }
    }

    fn unexpect(&self, expects: &str) -> ParseError {
        ParseError::new(format!(
            "unexpected token {}, expect {}",
            self.scanner.unwrap_current_token().kind,
            expects
        ))
    }

    fn unexpect_keyword(&self, expects: &str) -> ParseError {
        ParseError::new(format!(
            "unexpected keyword {}, expect {}",
            self.scanner.unwrap_current_token().value,
            expects
        ))
    }

    pub fn parse(&mut self) -> NodeResult {
        let mut exprs: Vec<Node> = Vec::new();

        goahead!(self);
        while !self.scanner.expect("eof") {
            if self.scanner.expect(";") {
                goahead!(self);
            } else {
                let node = self.parse_multi_tests()?;
                exprs.push(*node);
            }
        }
        if exprs.len() == 1 {
            return Ok(Box::new(exprs[0].clone()));
        } else {
            return Ok(Box::new(ExprList { elements: exprs }));
        }
    }

    fn parse_multi_tests_element(&mut self) -> NodeResult {
        if self
            .scanner
            .expect_kinds(&[">", ">=", "<", "<=", "!=", "="])
        {
            // unary tests
            let op = self.scanner.unwrap_current_token().kind;
            goahead!(self); // skip op
            let right = self.parse_expression()?;
            let left = Box::new(Var("?".to_owned()));
            Ok(Box::new(Binop {
                op: op.to_string(),
                left,
                right,
            }))
        } else {
            self.parse_expression()
        }
    }

    fn parse_multi_tests(&mut self) -> NodeResult {
        let elem = self.parse_multi_tests_element()?;
        if self.scanner.expect(",") {
            let mut elements = Vec::new();
            elements.push(*elem);
            while self.scanner.expect(",") {
                goahead!(self); // skip ','
                let elem1 = self.parse_multi_tests_element()?;
                elements.push(*elem1);
            }
            Ok(Box::new(MultiTests { elements }))
        } else {
            Ok(elem)
        }
    }

    fn parse_expression(&mut self) -> NodeResult {
        self.parse_in_op()
    }

    // binary operators
    fn parse_binop_keywords(
        &mut self,
        keywords: &[&str],
        sub_func: fn(&mut Self) -> NodeResult,
    ) -> NodeResult {
        let mut left = sub_func(self)?;
        while self.scanner.expect_keywords(keywords) {
            let op = self.scanner.unwrap_current_token().value;
            goahead!(self);
            let right = sub_func(self)?;
            left = Box::new(Binop { op, left, right });
        }
        Ok(left)
    }

    fn parse_binop_kinds(
        &mut self,
        kinds: &[&str],
        sub_parse: fn(&mut Self) -> NodeResult,
    ) -> NodeResult {
        let mut left = sub_parse(self)?;
        while self.scanner.expect_kinds(kinds) {
            let op = self.scanner.unwrap_current_token().value;
            goahead!(self);
            let right = sub_parse(self)?;
            left = Box::new(Binop { op, left, right });
        }
        Ok(left)
    }

    fn parse_in_op(&mut self) -> NodeResult {
        self.parse_binop_keywords(&["in"], Parser::parse_logic_or)
    }

    fn parse_logic_or(&mut self) -> NodeResult {
        self.parse_binop_keywords(&["or"], Parser::parse_logic_and)
    }

    fn parse_logic_and(&mut self) -> NodeResult {
        self.parse_binop_keywords(&["and"], Parser::parse_compare)
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
            match self.scanner.unwrap_current_token().kind {
                "(" => {
                    node = self.parse_funccall_rest(node)?;
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

    fn parse_funccall_rest(&mut self, func_node: Box<Node>) -> NodeResult {
        goahead!(self); // skip "("
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
        Ok(Box::new(FuncCall {
            func_ref: func_node,
            args,
        }))
    }

    fn parse_funcall_arg(&mut self) -> Result<FuncCallArg, ParseError> {
        let arg = self.parse_expression()?;
        if self.scanner.expect(":") {
            goahead!(self);
            if let Var(name) = *arg {
                goahead!(self); // skip ":"
                let arg_value = self.parse_expression()?;
                return Ok(FuncCallArg {
                    arg_name: name,
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

        let at = self.parse_expression()?;
        if !self.scanner.expect("]") {
            return Err(self.unexpect("]"));
        }
        goahead!(self);
        return Ok(Box::new(Binop {
            op: "[]".to_owned(),
            left,
            right: at,
        }));
    }

    fn parse_dot_rest(&mut self, left: Box<Node>) -> NodeResult {
        goahead!(self); // skip "."
        let attr = self.parse_name(None)?;
        return Ok(Box::new(DotOp { left, attr }));
    }

    // single element
    fn parse_single_element(&mut self) -> NodeResult {
        match self.scanner.unwrap_current_token().kind {
            "number" => self.parse_number(),
            "name" => self.parse_var(),
            "string" => self.parse_string(),
            "temporal" => self.parse_temporal(),
            "-" => self.parse_neg(),
            "{" => self.parse_map(),
            "(" => self.parse_bracket_or_range(),
            "[" => self.parse_range_or_array(),
            "?" => Ok(Box::new(Var("?".to_owned()))),
            "keyword" => match self.scanner.unwrap_current_token().value.as_str() {
                "true" | "false" => self.parse_bool(),
                "null" => self.parse_null(),
                "if" => self.parse_if_expression(),
                "for" => self.parse_for_expression(),
                "some" | "every" => self.parse_some_or_every_expression(),
                "function" => self.parse_function_defination(),
                _ => return Err(self.unexpect_keyword("true, false")),
            },
            _ => return Err(self.unexpect("name, number")),
        }
    }

    fn parse_name(&mut self, stop_keywords: Option<&[&str]>) -> Result<String, ParseError> {
        let mut names: Vec<String> = Vec::new();

        while self.scanner.expect_kinds(&["name", "keyword"]) {
            let token = self.scanner.unwrap_current_token();
            if let ("keyword", Some(stop_keywords)) = (token.kind, stop_keywords) {
                let token_keyword = token.value.as_str();
                if stop_keywords.into_iter().any(|x| *x == token_keyword) {
                    break;
                }
            }
            names.push(token.value);
            goahead!(self);
        }
        if names.len() > 0 {
            let mut name_buffer = String::new();
            for (i, name) in names.iter().enumerate() {
                if i > 0 {
                    name_buffer.push_str(" ");
                }
                name_buffer.push_str(name.as_str());
            }
            Ok(name_buffer)
        } else {
            Err(self.unexpect("names"))
        }
    }

    fn parse_var(&mut self) -> NodeResult {
        let token = self.scanner.unwrap_current_token();
        goahead!(self);
        Ok(Box::new(Var(token.value)))
    }

    fn parse_number(&mut self) -> NodeResult {
        let token = self.scanner.unwrap_current_token();
        goahead!(self);
        Ok(Box::new(Number(token.value)))
    }

    fn parse_neg(&mut self) -> NodeResult {
        goahead!(self); // skip '-'
        let value = self.parse_expression()?;
        Ok(Box::new(Neg(value)))
    }

    fn parse_string(&mut self) -> NodeResult {
        let token = self.scanner.unwrap_current_token();
        goahead!(self);
        Ok(Box::new(Str(token.value)))
    }

    fn parse_temporal(&mut self) -> NodeResult {
        let token = self.scanner.unwrap_current_token();
        goahead!(self);
        Ok(Box::new(Temporal(token.value)))
    }

    fn parse_bool(&mut self) -> NodeResult {
        let bool_value = match self.scanner.unwrap_current_token().value.as_str() {
            "true" => true,
            "false" => false,
            _ => return Err(self.unexpect_keyword("true, false")),
        };
        goahead!(self);
        Ok(Box::new(Bool(bool_value)))
    }

    fn parse_null(&mut self) -> NodeResult {
        goahead!(self); // skip 'null'
        Ok(Box::new(Null))
    }

    // parse map/context defination
    fn parse_map(&mut self) -> NodeResult {
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
        Ok(Box::new(Map(items)))
    }

    fn parse_map_key(&mut self) -> NodeResult {
        if self.scanner.expect("name") {
            match self.parse_name(None) {
                Ok(name) => Ok(Box::new(Ident(name))),
                Err(err) => Err(err),
            }
        } else if self.scanner.expect("string") {
            self.parse_string()
        } else {
            return Err(self.unexpect("name or string"));
        }
    }

    fn parse_range_given_start(&mut self, start_open: bool, start_exp: Box<Node>) -> NodeResult {
        let end_exp = self.parse_expression()?;
        if self.scanner.expect(")") {
            // open end range
            goahead!(self); //skip ')'
            return Ok(Box::new(Range {
                start_open,
                start: start_exp,
                end_open: true,
                end: end_exp,
            }));
        } else if self.scanner.expect("]") {
            // close end range
            goahead!(self); //skip ')'
            return Ok(Box::new(Range {
                start_open,
                start: start_exp,
                end_open: false,
                end: end_exp,
            }));
        } else {
            return Err(self.unexpect("')', ']'"));
        }
    }

    fn parse_bracket_or_range(&mut self) -> NodeResult {
        goahead!(self); // skip '('
        let aexp = self.parse_expression()?;
        if self.scanner.expect("..") {
            // is range
            goahead!(self); // skip '..'
            return self.parse_range_given_start(true, aexp);
        } else if self.scanner.expect(")") {
            goahead!(self); // skip ')'
            return Ok(aexp);
        } else {
            return Err(self.unexpect("')', '..'"));
        }
    }

    fn parse_range_or_array(&mut self) -> NodeResult {
        goahead!(self); // skip '['
        if self.scanner.expect("]") {
            goahead!(self); // skip ']'
            return Ok(Box::new(Array(Vec::new())));
        }
        let aexp = self.parse_expression()?;
        if self.scanner.expect_kinds(&[",", "]"]) {
            return self.parse_array_given_first(aexp);
        }

        if !self.scanner.expect("..") {
            return Err(self.unexpect("'..'"));
        }
        goahead!(self); // skip '..'

        return self.parse_range_given_start(false, aexp);
    }

    fn parse_array_given_first(&mut self, first_element: Box<Node>) -> NodeResult {
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
        Ok(Box::new(Array(elements)))
    }

    // if expression
    fn parse_if_expression(&mut self) -> NodeResult {
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
        Ok(Box::new(IfExpr {
            condition: cond,
            then_branch,
            else_branch,
        }))
    }

    fn parse_for_expression(&mut self) -> NodeResult {
        goahead!(self); // skip 'for'
        let var_name = self.parse_name(Some(&["in", "for"]))?;
        if !self.scanner.expect_keyword("in") {
            return Err(self.unexpect_keyword("in"));
        }
        goahead!(self); // skip 'in'

        let list_expr = self.parse_expression()?;
        if self.scanner.expect(",") {
            // recursively call for parser
            let return_expr = self.parse_for_expression()?;
            return Ok(Box::new(ForExpr {
                var_name,
                list_expr,
                return_expr,
            }));
        }

        if !self.scanner.expect_keyword("return") {
            return Err(self.unexpect_keyword("return"));
        }
        goahead!(self); // skip 'return'

        let return_expr = self.parse_for_expression()?;
        Ok(Box::new(ForExpr {
            var_name,
            list_expr,
            return_expr,
        }))
    }

    fn parse_some_or_every_expression(&mut self) -> NodeResult {
        let cmd = self.scanner.unwrap_current_token().value;
        goahead!(self); // skip 'for'
        let var_name = self.parse_name(Some(&["in"]))?;

        if !self.scanner.expect_keyword("in") {
            return Err(self.unexpect_keyword("in"));
        }
        goahead!(self); // skip 'in'

        let list_expr = self.parse_expression()?;
        if !self.scanner.expect_keyword("satisfies") {
            return Err(self.unexpect_keyword("satisfies"));
        }
        goahead!(self); // skip 'satisfies'

        let filter_expr = self.parse_for_expression()?;
        if cmd == "some".to_owned() {
            Ok(Box::new(SomeExpr {
                var_name,
                list_expr,
                filter_expr,
            }))
        } else {
            Ok(Box::new(EveryExpr {
                var_name,
                list_expr,
                filter_expr,
            }))
        }
    }

    fn parse_function_defination(&mut self) -> NodeResult {
        goahead!(self); // skip 'function'
        if !self.scanner.expect("(") {
            return Err(self.unexpect("'('"));
        }
        goahead!(self); // skip '('

        let mut arg_names = Vec::new();
        while !self.scanner.expect(")") {
            let arg_name = self.parse_name(None)?;
            arg_names.push(arg_name);
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
        Ok(Box::new(FuncDef {
            arg_names,
            body: exp,
        }))
    }
}

pub fn parse(input: &str) -> NodeResult {
    let mut parser = Parser::new(input);
    parser.parse()
}

#[test]
fn test_parse_results() {
    let testcases = [
        ("a + b(4, 9)", "(+ a (call b [4, 9]))"),
        ("if a > 6 then true else false", "(if (> a 6) true false)"),
        ("{a: 1, \"bbb\": [2, 1]}", r#"{a: 1, "bbb": [2, 1]}"#),
        ("> 2, <= 1, a>8", "(> ? 2), (<= ? 1), (> a 8)"),
    ];

    for (input, output) in testcases {
        let node = parse(input).unwrap();
        assert_eq!(format!("{}", *node), output);
    }
}
