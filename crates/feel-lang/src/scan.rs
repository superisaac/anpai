use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

// Scan error
#[derive(Debug, Clone)]
pub struct ScanError {
    pub message: String,
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ScanError: {}", self.message)
    }
}

impl Error for ScanError {}

impl ScanError {
    pub fn from_str(message: &str) -> ScanError {
        ScanError {
            message: message.to_owned(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct TextPosition {
    pub chars: usize,
    pub lines: usize,
    pub cols: usize,
}

impl fmt::Display for TextPosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "chars: {}, lines: {}, cols: {}",
            self.chars, self.lines, self.cols
        )
    }
}

impl TextPosition {
    pub fn zero() -> TextPosition {
        TextPosition {
            chars: 0,
            lines: 0,
            cols: 0,
        }
    }

    pub fn is_zero(&self) -> bool {
        return self.chars == 0 && self.lines == 0 && self.cols == 0;
    }

    pub fn increase(&self, chunk: &str) -> TextPosition {
        let lines: Vec<&str> = chunk.split("\n").collect();
        if lines.len() == 0 {
            return self.clone();
        }
        let delta_cols = if lines.len() == 1 {
            // the same line with the original pos
            self.cols
        } else {
            0
        };
        TextPosition {
            chars: self.chars + chunk.len(),
            lines: self.lines + lines.len() - 1,
            cols: delta_cols + lines.last().unwrap().len(),
        }
    }

    pub fn line_pointers(&self, full_text: &str) -> String {
        let lines: Vec<&str> = full_text.split("\n").collect();
        let spaces = if self.cols > 0 {
            " ".repeat(self.cols - 1)
        } else {
            "".to_string()
        };
        format!("{}\n{}^\n", lines[self.lines], spaces)
    }
}

// Token struct
#[derive(Clone, Debug)]
pub struct Token {
    pub kind: &'static str,
    pub value: String,
    pub position: TextPosition,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{}..{}] {} '{}'",
            self.position.chars,
            self.value.len() + self.position.chars,
            self.kind,
            self.value
        )
    }
}

impl Token {
    pub fn expect(&self, kind: &str) -> bool {
        self.kind == kind
    }

    pub fn expect_kinds(&self, kinds: &[&str]) -> bool {
        kinds.into_iter().any(|kind| self.expect(*kind))
    }

    pub fn expect_keyword(&self, keyword: &str) -> bool {
        if self.kind != "keyword" {
            return false;
        }
        self.value.as_str() == keyword
    }

    pub fn expect_keywords(&self, keywords: &[&str]) -> bool {
        if self.kind != "keyword" {
            return false;
        }
        let self_keyword = self.value.as_str();
        keywords.into_iter().any(|kw| self_keyword == *kw)
    }
}

#[test]
fn test_token_expect() {
    let token = Token {
        kind: "abc",
        value: "xyz".to_owned(),
        position: TextPosition::zero(),
    };
    assert!(token.expect_kinds(&["abc", "kkk"]));
    assert!(!token.expect_kinds(&["abcdef", "kkk"]));
}

#[test]
fn test_token_expect_keywords() {
    let token = Token {
        kind: "keyword",
        value: "xyz".to_owned(),
        position: TextPosition::zero(),
    };
    assert!(!token.expect_keywords(&["abc", "kkk"]));
    assert!(token.expect_keywords(&["xyz", "kkk"]));
}

#[test]
fn test_value_ahead_01() {
    let text = r#"
    abc 
    def ghi
    ok"#;
    let cursor = TextPosition::zero().increase(text);
    assert_eq!(cursor.chars, text.len());
    assert_eq!(cursor.lines, 3);
    assert_eq!(cursor.cols, 6); // "    ok".len()
}

#[test]
fn test_value_ahead_02() {
    let text = "2 + +";
    let cursor = TextPosition::zero().increase(text);
    assert_eq!(cursor.chars, text.len());
    assert_eq!(cursor.lines, 0);
    assert_eq!(cursor.cols, 5);
}

#[test]
fn test_value_ahead_03() {
    let text = "\n\n2 + +";
    let cursor = TextPosition::zero().increase(text);
    assert_eq!(cursor.chars, text.len());
    assert_eq!(cursor.lines, 2);
    assert_eq!(cursor.cols, 5);
}

#[derive(Clone)]
struct TokenPattern {
    token: &'static str,
    reg: Option<Regex>,
}

lazy_static! {
    static ref TOKEN_PATTERNS: Vec<TokenPattern> = {
        let mut patterns: Vec<TokenPattern> = Vec::new();
        patterns.push(TokenPattern {
            token: "space",
            reg: Some(Regex::new(r"^\s+").unwrap()),
        });
        patterns.push(TokenPattern {
            token: "comment_singleline",
            reg: Some(Regex::new(r"^//.*\n").unwrap()),
        });
        patterns.push(TokenPattern {
            token: "comment_multiline",
            reg: Some(Regex::new(r"^/\*(.|\n)*\*/").unwrap()),
        });
        patterns.push(TokenPattern{
            token: "keyword",
            reg: Some(Regex::new(r"^\b(true|false|and|or|null|function|if|then|else|loop|for|some|every|in|return|satisfies)\b").unwrap()),
        });
        patterns.push(TokenPattern {
            token: "temporal",
            reg: Some(Regex::new(r#"^@"(\\.|[^"])*""#).unwrap()),
        });
        patterns.push(TokenPattern {
            token: "string",
            reg: Some(Regex::new(r#"^"(\\.|[^"])*""#).unwrap()),
        });

        let ops = [
            "..", ".", ",", ";", ">=", ">", "=", "<=", "<", "!=", "!", "(", ")", "[", "]",
            "{", "}", ":=", ":", "+", "-", "*", "/", "%",
        ];
        for op in ops {
            patterns.push(TokenPattern {
                token: op,
                reg: None,
            });
        }

        patterns.push(TokenPattern {
            token: "number",
            reg: Some(Regex::new(r#"^[0-9]+(\.[0-9]+)?\b"#).unwrap()),
        });

        patterns.push(TokenPattern{
            token: "name",
            //reg: Some(Regex::new(r"^[a-zA-Z_][a-zA-Z_0-9]*( +[a-zA-Z_][a-zA-Z_0-9]*)*").unwrap()),
            reg: Some(Regex::new(r"(?x)
            ^[a-zA-Z_\$\?\%\p{Han}\p{Greek}\p{Bopomofo}\p{Hangul}][a-zA-Z_\$\?\%0-9\p{Han}\p{Greek}\p{Bopomofo}\p{Hangul}]*
            ").unwrap()),
        });

        patterns
    };
}

//#[derive(Clone, Copy)]
pub struct Scanner<'a> {
    // input text
    input: &'a str,
    // current scan position
    cursor: TextPosition,

    // current abtained token
    current: Option<Token>,
}

impl Scanner<'_> {
    // constructor
    pub fn new(input: &str) -> Scanner {
        Scanner {
            cursor: TextPosition::zero(),
            current: None,
            input,
        }
    }

    // if the scan reached the end of input
    pub fn is_eof(&self) -> bool {
        self.cursor.chars >= self.input.len()
    }

    pub fn text_range(&self, start: usize, end: usize) -> String {
        self.input[start..end].to_string()
    }

    // returns current token
    // pub fn current_token(&self) -> Option<Token> {
    //     self.current.clone()
    // }

    // returns the current token unwrapped
    pub fn current_token(&self) -> Token {
        self.current.clone().unwrap()
    }

    // expect the current token to be kind
    pub fn expect(&self, kind: &str) -> bool {
        self.current
            .clone()
            .map(|t| t.expect(kind))
            .unwrap_or(false)
    }

    // expect the current token to be one of the kinds
    pub fn expect_kinds(&self, kinds: &[&str]) -> bool {
        self.current
            .clone()
            .map(|t| t.expect_kinds(kinds))
            .unwrap_or(false)
    }

    pub fn expect_keyword(&self, keyword: &str) -> bool {
        self.current
            .clone()
            .map(|t| t.expect_keyword(keyword))
            .unwrap_or(false)
    }

    pub fn expect_keywords(&self, keywords: &[&str]) -> bool {
        self.current
            .clone()
            .map(|t| t.expect_keywords(keywords))
            .unwrap_or(false)
    }

    pub fn next_token(&mut self) -> Result<(), ScanError> {
        match self.find_next_token() {
            Ok(token) => {
                if token.kind == "comment_singleline"
                    || token.kind == "comment_multiline"
                    || token.kind == "space"
                {
                    return self.next_token();
                }
                self.current = Some(token.clone());
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn find_next_token(&mut self) -> Result<Token, ScanError> {
        if self.is_eof() {
            return Ok(Token {
                kind: "eof",
                value: "".to_owned(),
                position: self.cursor.clone(),
            });
        }
        let rest = &self.input[(self.cursor.chars)..];
        for pattern in TOKEN_PATTERNS.iter() {
            if let Some(reg) = &pattern.reg {
                if let Some(m) = reg.find(rest) {
                    assert_eq!(0, m.start());
                    let token = Token {
                        kind: pattern.token,
                        value: m.as_str().to_owned(),
                        position: self.cursor.clone(),
                    };
                    //self.cursor += token.value.len();
                    self.cursor = self.cursor.increase(&token.value);
                    return Ok(token);
                }
            } else if rest.starts_with(pattern.token) {
                let token = Token {
                    kind: pattern.token,
                    value: String::from(pattern.token),
                    position: self.cursor.clone(),
                };
                //self.cursor += token.value.len();
                self.cursor = self.cursor.increase(&token.value);
                return Ok(token);
            }
        }
        Err(ScanError::from_str("fail to find token"))
    }

    pub fn rewind(&mut self, token: Token) {
        self.cursor = token.position.increase(&token.value);
        self.current = Some(token.clone());
    }

    // pub fn find_tokens(&mut self) -> Result<Vec<Token>, ScanError> {
    //     let mut token_vecs: Vec<Token> = Vec::new();
    //     while !self.is_eof() {
    //         if let Err(err) = self.next_token() {
    //             return Err(err);
    //         }
    //         token_vecs.push(self.current_token());
    //     }
    //     Ok(token_vecs)
    // }
}
