mod ast;
mod token;

mod parse;

mod value;

mod eval;

fn main() {
    token::parse_token();
    let n = ast::Node::Number {
        value: "123".to_owned(),
    };
    println!("{}", n);

    let mut p = parse::Parser::new("a(5,9)");
    match p.parse() {
        Ok(node) => println!("P: {}", node),
        Err(err) => panic!("{}", err),
    }
}
