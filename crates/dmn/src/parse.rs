extern crate sxd_document;
extern crate sxd_xpath;

use std::fs;
use sxd_document::parser;
use sxd_xpath::{Factory, Context, Value};
//use sxd_document::dom::Element;
use sxd_xpath::nodeset::Node;

#[derive(Clone, Debug)]
pub struct InputExpression {
    pub id: String,
    pub type_ref: String,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Input {
    pub id: String,
    pub label: String,
    pub expression: InputExpression
}

#[derive(Clone, Debug)]
pub struct Output {
    pub id: String,
    pub name: String,
    pub type_ref: String,
}

#[derive(Clone, Debug)]
pub struct DicisionTable {
    pub id: String,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}

pub struct Parser<'a> {
    factory: Factory,
    context: Context<'a>,
}

impl Parser<'_> {
    pub fn new<'a>() -> Parser<'a> {
        let factory = Factory::new();
        let mut context = Context::new();
        context.set_namespace("ns", "https://www.omg.org/spec/DMN/20191111/MODEL/");
        Parser {
            factory, context
        }
    }

    fn get_text(&self, node: Node, xpath: &str) -> String {
        let b = self.factory.build(xpath).unwrap().unwrap();
        match b.evaluate(&self.context, node).unwrap() {
            Value::String(s) => s,
            Value::Nodeset(nodeset) => {
                let mut buf:String = String::new();
                for n in nodeset.iter() {
                    buf.push_str(n.string_value().as_str());
                }
                return buf;
            }
            Value::Boolean(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
        }
    }

    fn get_attribute(&self, node: Node, attr_name: &str) -> String {
        if let Node::Element(e) = node {
            return e.attribute(attr_name).unwrap().value().to_owned();
        }
        return "".to_owned();
    }

    // fn evaluate_xpath<'a>(&'a self, node: Node<'a>, xpath: &str) -> Value<'_> {
    //     let p = self.factory.build(xpath).unwrap().unwrap();
    //     let r = p.evaluate(&self.context, node).unwrap();
    //     r.clone()        
    // }

    // fn get_first_element<'a>(&'a self, node: Node<'a>, xpath: &str) -> Option<Element> {
    //     let b = self.factory.build(xpath).unwrap().unwrap();
    //     match b.evaluate(&self.context, node).unwrap() {
    //         Value::Nodeset(nodeset ) => {
    //             for n in nodeset.iter() {
    //                 if let Node::Element(e) = n {
    //                     return Some(e.clone());
    //                 }
    //             }
    //             None
    //         }
    //         _ => None,
    //     }
    // }

    fn get_first_element_node<'a>(&'a self, node: Node<'a>, xpath: &str) -> Option<Node<'a>> {
        let b = self.factory.build(xpath).unwrap().unwrap();
        match b.evaluate(&self.context, node).unwrap() {
            Value::Nodeset(nodeset ) => {
                for n in nodeset.iter() {
                    if let Node::Element(_) = n {
                        return Some(n.clone());
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn get_element_nodes<'a>(&'a self, node: Node<'a>, xpath: &str) -> Vec<Node<'a>> {
        let b = self.factory.build(xpath).unwrap().unwrap();
        let mut nodes: Vec<Node> = vec![];
        let value = b.evaluate(&self.context, node).unwrap();
        println!("node html {} {:?}", xpath, node);
        println!("element nodes {:?}", value);
        match value {
            Value::Nodeset(nodeset ) => {
                for n in nodeset.iter() {
                    if let Node::Element(_) = n {
                        nodes.push(n.clone());
                    }
                }
            }
            _ => (),
        }
        nodes
    }

    // fn get_elements<'a>(&'a self, node: Node<'a>, xpath: &str) -> Vec<Element> {
    //     let b = self.factory.build(xpath).unwrap().unwrap();
    //     let mut elements: Vec<Element> = vec![];
    //     match b.evaluate(&self.context, node).unwrap() {
    //         Value::Nodeset(nodeset ) => {
    //             for n in nodeset.iter() {
    //                 if let Node::Element(e) = n {
    //                     elements.push(e.clone());
    //                 }
    //             }
    //         }
    //         _ => (),
    //     }
    //     elements
    // }

    fn parse_input(&self, n: Node) -> Input {
        if let Node::Element(_) = n {
            let id = self.get_attribute(n, "id");
            let label = self.get_attribute(n, "label");

            let expr_node = self.get_first_element_node(n, "ns:inputExpression").unwrap();
            let input_expr = InputExpression {
                id: self.get_attribute(expr_node, "id"),
                type_ref: self.get_attribute(expr_node, "typeRef"),
                text: self.get_text(expr_node, "ns:text"),
            };

            Input {
                id, label, expression: input_expr
            }
        } else {
            panic!("not input");
        }
    }

    fn parse_output(&self, n: Node) -> Output {
        let id = self.get_attribute(n, "id");
        let type_ref = self.get_attribute(n, "typeRef");
        let name = self.get_attribute(n, "name");
        Output {
            id, type_ref, name,
        }
    }

    fn parse_decision_table(&self, node: Node) -> DicisionTable {
        if let Node::Element(_) = node  {
            let id = self.get_attribute(node, "id");

            let mut inputs: Vec<Input> = vec![];
            for input_node in self.get_element_nodes(node, "ns:input") {
                let input = self.parse_input(input_node);
                inputs.push(input);
            }

            let mut outputs: Vec<Output> = vec![];
            for output_node in self.get_element_nodes(node, "ns:output") {
                let output = self.parse_output(output_node);
                outputs.push(output);
            }

            DicisionTable{id: id.to_owned(), inputs, outputs}
        } else {
            panic!("bad element");
        }
    }

    fn parse_file(&self, path: &str) -> DicisionTable {
        let contents = fs::read_to_string(path).unwrap();
        let package = parser::parse(contents.as_str()).unwrap();
        let doc = package.as_document();
        let dicision_table_node = self.get_first_element_node(
            doc.root().into(),
            "/ns:definitions/ns:decision/ns:decisionTable").unwrap();
        self.parse_decision_table(dicision_table_node)
    }
}

pub fn parse_file(path: &str) {
    let parser = Parser::new();
    let table = parser.parse_file(path);
    println!("{:?}", table);
} 

#[cfg(test)]
mod test {

    #[test]
    fn test_parse_simple_dmn() {
        super::parse_file("src/fixtures/dmn/simpledish.dmn");
    }
}
