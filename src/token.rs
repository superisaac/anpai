use lazy_static::lazy_static;
use regex::Regex;
use std::fmt;

#[derive(Clone)]
pub struct Token {
    kind: String,
    value: String,
    position: usize,
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
    pub fn expect(&self, kinds: &[&str]) -> bool {
        let self_kind = self.kind.as_str();
        kinds.into_iter().any(|kind| self_kind == *kind)
    }

    pub fn expect_keywords(&self, keywords: &[&str]) -> bool {
        if self.kind != "keyword".to_owned() {
            return false;
        }
        let self_keyword = self.value.as_str();
        keywords.into_iter().any(|kw| self_keyword == *kw)
    }
}

#[test]
fn test_token_expect() {
    let token = Token {
        kind: "abc".to_owned(),
        value: "xyz".to_owned(),
        position: 0,
    };
    assert!(token.expect(&["abc", "kkk"]));
    assert!(!token.expect(&["abcdef", "kkk"]));
}

#[test]
fn test_token_expect_keywords() {
    let token = Token {
        kind: "keyword".to_owned(),
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
struct TokenScanner<'a> {
    // input text
    input: &'a str,
    // current scan position
    cursor: usize,

    // current abtained token
    current: Option<Token>,
}

impl TokenScanner<'_> {
    // constructor
    pub fn new<'a>(input: &str) -> TokenScanner {
        return TokenScanner {
            cursor: 0,
            current: None,
            input: input.clone(),
        };
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

    // expect the current token to be one of the kinds
    pub fn expect(&self, kinds: &[&str]) -> bool {
        self.current
            .clone()
            .map(|t| t.expect(kinds))
            .unwrap_or(false)
    }

    pub fn expect_keywords(&self, keywords: &[&str]) -> bool {
        self.current
            .clone()
            .map(|t| t.expect_keywords(keywords))
            .unwrap_or(false)
    }

    pub fn next_token(&mut self) -> Result<(), &'static str> {
        match self.find_next_token() {
            Ok(token) => {
                self.current = Some(token.clone());
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn find_next_token(&mut self) -> Result<Token, &'static str> {
        if self.is_eof() {
            return Ok(Token {
                kind: "eof".to_owned(),
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
                        kind: String::from(pattern.token),
                        value: m.as_str().to_owned(),
                        position: self.cursor,
                    };
                    self.cursor += token.value.len();
                    return Ok(token);
                }
            } else if rest.starts_with(pattern.token) {
                let token = Token {
                    kind: String::from(pattern.token),
                    value: String::from(pattern.token),
                    position: self.cursor,
                };
                self.cursor += token.value.len();
                return Ok(token);
            }
        }
        Err("fail to find token")
    }

    pub fn find_tokens(&mut self) -> Result<Vec<Token>, &'static str> {
        let mut token_vecs: Vec<Token> = Vec::new();
        while !self.is_eof() {
            match self.next_token() {
                Ok(_) => {
                    token_vecs.push(self.unwrap_current_token());
                }
                Err(msg) => return Err(msg),
            }
        }
        Ok(token_vecs)
    }
}

pub fn parse_token() {
    let input = "1 world我 but(a+b) true";
    let mut scanner = TokenScanner::new(input);
    match scanner.find_tokens() {
        Ok(tokens) => {
            for token in tokens {
                println!("{}", token);
            }
        }
        Err(err) => {
            panic!("{}", err);
        }
    }
}
