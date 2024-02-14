use crate::ast::{Node, Node::*};
use crate::token::Scanner;

type NodeResult = Result<Box<Node>, &'static str>;

pub struct Parser<'a> {
    scanner: Box<Scanner<'a>>,
}

// shortcuts to go ahead one token
macro_rules! goahead {
    ($parser:ident) => {
        if let Err(err) = $parser.scanner.next_token() {
            return Err(err);
        }
    };
}

impl Parser<'_> {
    pub fn new(input: &str) -> Parser {
        let scanner = Scanner::new(input);
        Parser {
            scanner: Box::new(scanner),
        }
    }

    pub fn parse(&mut self) -> NodeResult {
        let mut exprs: Vec<Node> = Vec::new();

        goahead!(self);
        while !self.scanner.expect("eof") {
            if self.scanner.expect(";") {
                goahead!(self);
            } else {
                match self.parse_expression() {
                    Ok(node) => exprs.push(*node),
                    Err(err) => return Err(err),
                }
            }
        }
        if exprs.len() == 1 {
            return Ok(Box::new(exprs[0].clone()));
        } else {
            return Ok(Box::new(ExprList { elements: exprs }));
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
        let mut left = match sub_func(self) {
            Ok(node) => node,
            Err(err) => return Err(err),
        };

        while self.scanner.expect_keywords(keywords) {
            let op = self.scanner.unwrap_current_token().value;
            goahead!(self);

            let right = match sub_func(self) {
                Ok(node) => node,
                Err(err) => return Err(err),
            };

            left = Box::new(Binop { op, left, right });
        }
        Ok(left)
    }

    fn parse_binop_kinds(
        &mut self,
        kinds: &[&str],
        sub_func: fn(&mut Self) -> NodeResult,
    ) -> NodeResult {
        let mut left = match sub_func(self) {
            Ok(node) => node,
            Err(err) => return Err(err),
        };

        while self.scanner.expect_kinds(kinds) {
            let op = self.scanner.unwrap_current_token().value;
            goahead!(self);

            let right = match sub_func(self) {
                Ok(node) => node,
                Err(err) => return Err(err),
            };

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
        self.parse_binop_kinds(&["*", "/", "%"], Parser::parse_single_element)
    }

    // single element
    fn parse_single_element(&mut self) -> NodeResult {
        match self.scanner.unwrap_current_token().kind {
            "number" => self.parse_number(),
            "name" => self.parse_var(),
            _ => {
                println!("unexpected => {}", self.scanner.unwrap_current_token());
                return Err("unexpected token");
            }
        }
    }

    fn parse_var(&mut self) -> NodeResult {
        let token = self.scanner.unwrap_current_token();
        goahead!(self);
        Ok(Box::new(Var { name: token.value }))
    }

    fn parse_number(&mut self) -> NodeResult {
        let token = self.scanner.unwrap_current_token();
        goahead!(self);
        Ok(Box::new(Number { value: token.value }))
    }
}
