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
        write!(f, "{}:{} at [{}..{}]", self.token, self.value, self.position, self.value.len() + self.position)
    }
}

#[derive(Clone)]
struct TokenMatcher {
    token: &'static str,
    reg: Option<Regex>,
}

struct TokenScanner {
    matchers : Vec<TokenMatcher>
}

impl TokenScanner {
    pub fn new<'a>() -> TokenScanner {
        let mut matchers: Vec<TokenMatcher> = Vec::new();
        matchers.push(TokenMatcher{
            token: "space",
            reg: Some(Regex::new(r"^\s+").unwrap()),
        });
        matchers.push(TokenMatcher{
            token: "comment_singleline",
            reg: Some(Regex::new(r"^//.*\n").unwrap()),
        });
        matchers.push(TokenMatcher{
            token: "comment_multiline",
            reg: Some(Regex::new(r"^/\*(.|\n)*\*/").unwrap()),
        });
        matchers.push(TokenMatcher{
            token: "keywords",
            reg: Some(Regex::new(r"^\b(true|false|and|or|null|function|if|then|else|loop|for|some|every|in|return|satisfies)\b").unwrap()),
        });
        matchers.push(TokenMatcher{
            token: "temporal",
            reg: Some(Regex::new(r#"^@"(\\.|[^"])*""#).unwrap()),
        });
        matchers.push(TokenMatcher{
            token: "string",
            reg: Some(Regex::new(r#"^"(\\.|[^"])*""#).unwrap()),
        });
        matchers.push(TokenMatcher{
            token: "number",
            reg: Some(Regex::new(r#"^\-?[0-9]+(\.[0-9]+)?\b"#).unwrap()),
        });

        let ops = [
            "?", "..", ".", ",", ";",
            ">=", ">", "=", "<=", "<", "!=", "!",
            "(", ")", "[", "]", "{", "}",
            ":=", ":",
            "+", "-", "*", "/", "%"];
        for op in ops {
            matchers.push(TokenMatcher{
                token: op,
                reg: None,
            });
        }
        
        matchers.push(TokenMatcher{
            token: "name",
            reg: Some(Regex::new(r"^[a-zA-Z_][a-zA-Z_0-9]*( +[a-zA-Z_][a-zA-Z_0-9]*)*").unwrap()),
        });

        return TokenScanner{
            matchers
        };
    }

    pub fn find_tokens(self, input: &str) -> Vec<Token> {
        let mut cursor = 0;
        let mut token_vecs: Vec<Token> = Vec::new();
        while cursor < input.len() {
            let rest = &input[cursor..];
            for matcher in self.matchers.iter() {
                if let Some(reg) = &matcher.reg {
                    if let Some(m) = reg.find(rest) {
                        assert_eq!(0, m.start());
                        let token = Token { token: String::from(matcher.token), value: m.as_str().to_string(), position: cursor };
                        token_vecs.push(token);
                        cursor += m.end();
                        break;
                    }
                } else if rest.starts_with(matcher.token) {
                    let token = Token { token: String::from(matcher.token), value: String::from(matcher.token), position: cursor };
                    token_vecs.push(token);
                    cursor += matcher.token.len();
                    break;
                }
            }

        }
        token_vecs
    }
}

pub fn parse_token() {
    let scanner = TokenScanner::new();
    let tokens = scanner.find_tokens("1 world");
    for token in tokens.iter() {
        println!("{}", token);
    }

    
}
