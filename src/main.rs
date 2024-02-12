mod ast;
mod token;
fn main() {
    token::parse_token();
    let n = ast::Node::Number {
        value: "123".to_owned(),
    };
    println!("{}", n);
}
