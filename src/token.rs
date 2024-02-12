use regex::Regex;
use std::fmt;

#[derive(Clone)]
struct Token {
    token: String,
    value: String,
    position: usize,
}

impl fmt::Display for Token {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(
            f,
            "{}:{} at [{}..{}]",
            self.token,
            self.value,
            self.position,
            self.value.len() + self.position
        )
    }
}

#[derive(Clone)]
struct TokenMatcher {
    token: &'static str,
    reg: Option<Regex>,
}

//#[derive(Clone)]
struct TokenScanner<'a> {
    input: &'a str,
    cursor: usize,
    matchers: Vec<TokenMatcher>,
}

impl TokenScanner<'_> {
    pub fn new<'a>(input: &str) -> TokenScanner {
        let mut matchers: Vec<TokenMatcher> = Vec::new();
        matchers.push(TokenMatcher {
            token: "space",
            reg: Some(Regex::new(r"^\s+").unwrap()),
        });
        matchers.push(TokenMatcher {
            token: "comment_singleline",
            reg: Some(Regex::new(r"^//.*\n").unwrap()),
        });
        matchers.push(TokenMatcher {
            token: "comment_multiline",
            reg: Some(Regex::new(r"^/\*(.|\n)*\*/").unwrap()),
        });
        matchers.push(TokenMatcher{
            token: "keywords",
            reg: Some(Regex::new(r"^\b(true|false|and|or|null|function|if|then|else|loop|for|some|every|in|return|satisfies)\b").unwrap()),
        });
        matchers.push(TokenMatcher {
            token: "temporal",
            reg: Some(Regex::new(r#"^@"(\\.|[^"])*""#).unwrap()),
        });
        matchers.push(TokenMatcher {
            token: "string",
            reg: Some(Regex::new(r#"^"(\\.|[^"])*""#).unwrap()),
        });
        matchers.push(TokenMatcher {
            token: "number",
            reg: Some(Regex::new(r#"^\-?[0-9]+(\.[0-9]+)?\b"#).unwrap()),
        });

        let ops = [
            "?", "..", ".", ",", ";", ">=", ">", "=", "<=", "<", "!=", "!", "(", ")", "[", "]",
            "{", "}", ":=", ":", "+", "-", "*", "/", "%",
        ];
        for op in ops {
            matchers.push(TokenMatcher {
                token: op,
                reg: None,
            });
        }

        matchers.push(TokenMatcher{
            token: "name",
            //reg: Some(Regex::new(r"^[a-zA-Z_][a-zA-Z_0-9]*( +[a-zA-Z_][a-zA-Z_0-9]*)*").unwrap()),
            reg: Some(Regex::new(r"[a-zA-Z_\$\p{Han}\p{Greek}\p{Bopomofo}\p{Hangul}][a-zA-Z_\$0-9\p{Han}\p{Greek}}\p{Bopomofo}\p{Hangul}]*").unwrap()),
        });

        return TokenScanner {
            matchers,
            cursor: 0,
            input: input.clone(),
        };
    }

    pub fn is_eof(&self) -> bool {
        self.cursor >= self.input.len()
    }

    pub fn next_token(&mut self) -> Result<Token, &'static str> {
        if self.is_eof() {
            return Ok(Token {
                token: "eof".to_owned(),
                value: "".to_owned(),
                position: self.cursor,
            });
        }
        let rest = &self.input[self.cursor..];
        for matcher in self.matchers.iter() {
            if let Some(reg) = &matcher.reg {
                if let Some(m) = reg.find(rest) {
                    assert_eq!(0, m.start());
                    let token = Token {
                        token: String::from(matcher.token),
                        value: m.as_str().to_string(),
                        position: self.cursor,
                    };
                    self.cursor += token.value.len();
                    return Ok(token);
                }
            } else if rest.starts_with(matcher.token) {
                let token = Token {
                    token: String::from(matcher.token),
                    value: String::from(matcher.token),
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
                Ok(token) => {
                    token_vecs.push(token);
                }
                Err(msg) => return Err(msg),
            }
        }
        Ok(token_vecs)
    }
}

pub fn parse_token() {
    let input = "1 world我";
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
