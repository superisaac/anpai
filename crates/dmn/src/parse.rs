extern crate sxd_document;
extern crate sxd_xpath;

use std::fs;
use sxd_document::parser;
use sxd_xpath::{Factory, Context, Value};
use sxd_xpath::nodeset::Node;

pub fn parse_file(path: &str) {
    let contents = fs::read_to_string(path).unwrap(); // .expect("fail to read the file");
    //let contents = XMLC.to_owned();
    //println!("contents {}", contents);
    //let contents = "<definitions><a>hello</a></definitions>".to_owned();
    let package = parser::parse(contents.as_str()).unwrap(); //  .expect("fail to parse xml contents");
    let doc =  package.as_document();
    let factory = Factory::new();
    let mut context = Context::new();
    context.set_namespace("ns", "https://www.omg.org/spec/DMN/20191111/MODEL/");

    let xpath = factory.build("/ns:definitions").unwrap().unwrap();
    //let value = evaluate_xpath(&doc, "/ns:definitions").unwrap(); //.expect("fail to evaluate xpath");
    let value = xpath.evaluate(&context, doc.root()).unwrap();
    println!("value {:?}", value);
    if let Value::Nodeset(nodeset) = value {
        for n in nodeset.into_iter() {
            if let Node::Element(e) = n {
                println!("elem {:?}", e.name());
            }
            //println!("node ns:{:?} ", n.namespace())
        }
    }
    // match(value) {
    //     Value::Nodeset(nodeset) => {
    //         for n in nodeset.into_iter() {
    //             n.t
    //         }
    //     }
    //     _ => ()
    // }
} 

#[cfg(test)]
mod test {

    #[test]
    fn test_parse_simple_dmn() {
        super::parse_file("src/fixtures/dmn/simpledish.dmn");
    }
}
