use lazy_static::lazy_static;
use regex::Regex;
use std::fmt;

#[derive(Clone)]
pub struct Token {
    pub kind: &'static str,
    pub value: String,
    pub position: usize,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{}..{}] {} '{}'",
            self.position,
            self.value.len() + self.position,
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
        position: 0,
    };
    assert!(token.expect_kinds(&["abc", "kkk"]));
    assert!(!token.expect_kinds(&["abcdef", "kkk"]));
}

#[test]
fn test_token_expect_keywords() {
    let token = Token {
        kind: "keyword",
        value: "xyz".to_owned(),
        position: 0,
    };
    assert!(!token.expect_keywords(&["abc", "kkk"]));
    assert!(token.expect_keywords(&["xyz", "kkk"]));
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
        patterns.push(TokenPattern {
            token: "number",
            reg: Some(Regex::new(r#"^\-?[0-9]+(\.[0-9]+)?\b"#).unwrap()),
        });

        let ops = [
            "?", "..", ".", ",", ";", ">=", ">", "=", "<=", "<", "!=", "!", "(", ")", "[", "]",
            "{", "}", ":=", ":", "+", "-", "*", "/", "%",
        ];
        for op in ops {
            patterns.push(TokenPattern {
                token: op,
                reg: None,
            });
        }

        patterns.push(TokenPattern{
            token: "name",
            //reg: Some(Regex::new(r"^[a-zA-Z_][a-zA-Z_0-9]*( +[a-zA-Z_][a-zA-Z_0-9]*)*").unwrap()),
            reg: Some(Regex::new(r"(?x)
            ^[a-zA-Z_\$\p{Han}\p{Greek}\p{Bopomofo}\p{Hangul}][a-zA-Z_\$0-9\p{Han}\p{Greek}}\p{Bopomofo}\p{Hangul}]*
            (\s+[a-zA-Z_\$\p{Han}\p{Greek}\p{Bopomofo}\p{Hangul}][a-zA-Z_\$0-9\p{Han}\p{Greek}}\p{Bopomofo}\p{Hangul}]*)*
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
    cursor: usize,

    // current abtained token
    current: Option<Token>,
}

impl Scanner<'_> {
    // constructor
    pub fn new(input: &str) -> Scanner {
        Scanner {
            cursor: 0,
            current: None,
            input: input.clone(),
        }
    }

    // if the scan reached the end of input
    pub fn is_eof(&self) -> bool {
        self.cursor >= self.input.len()
    }

    // returns current token
    pub fn current_token(&self) -> Option<Token> {
        self.current.clone()
    }

    // returns the current token unwrapped
    pub fn unwrap_current_token(&self) -> Token {
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

    pub fn next_token(&mut self) -> Result<(), String> {
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

    fn find_next_token(&mut self) -> Result<Token, String> {
        if self.is_eof() {
            return Ok(Token {
                kind: "eof",
                value: "".to_owned(),
                position: self.cursor,
            });
        }
        let rest = &self.input[self.cursor..];
        for pattern in TOKEN_PATTERNS.iter() {
            if let Some(reg) = &pattern.reg {
                if let Some(m) = reg.find(rest) {
                    assert_eq!(0, m.start());
                    let token = Token {
                        kind: pattern.token,
                        value: m.as_str().to_owned(),
                        position: self.cursor,
                    };
                    self.cursor += token.value.len();
                    return Ok(token);
                }
            } else if rest.starts_with(pattern.token) {
                let token = Token {
                    kind: pattern.token,
                    value: String::from(pattern.token),
                    position: self.cursor,
                };
                self.cursor += token.value.len();
                return Ok(token);
            }
        }
        Err("fail to find token".to_owned())
    }

    pub fn find_tokens(&mut self) -> Result<Vec<Token>, String> {
        let mut token_vecs: Vec<Token> = Vec::new();
        while !self.is_eof() {
            if let Err(err) = self.next_token() {
                return Err(err);
            }
            token_vecs.push(self.unwrap_current_token());
        }
        Ok(token_vecs)
    }
}

pub fn parse_token() {
    let input = "1 world我 but(a+b) true";
    let mut scanner = Scanner::new(input);
    if let Ok(tokens) = scanner.find_tokens() {
        for token in tokens {
            println!("{}", token);
        }
    }
}
