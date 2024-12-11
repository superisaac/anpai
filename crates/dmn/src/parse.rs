extern crate sxd_document;
extern crate sxd_xpath;

use std::fs;
use sxd_document::parser;
use sxd_xpath::evaluate_xpath;

pub fn parse_file(path: &str) {
    let contents = fs::read_to_string(path).expect("fail to read the file");
    let package = parser::parse(contents.as_str()).expect("fail to parse xml contents");
    let doc =  package.as_document();

    let value = evaluate_xpath(&doc, "//decisionTable").expect("fail to evaluate xpath");
    println!("value {:?}", value);
} 

#[cfg(test)]
mod test {

    #[test]
    fn test_parse_simple_dmn() {
        super::parse_file("src/fixtures/dmn/simpledish.dmn");
    }
}
