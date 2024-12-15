extern crate sxd_document;
extern crate sxd_xpath;


use std::fs;

use sxd_document::parser;
use sxd_xpath::{Context, Factory, Value};
use sxd_xpath::nodeset::Node;

use crate::types::*;

pub struct Parser<'a> {
    factory: Factory,
    context: Context<'a>,
}

impl Parser<'_> {
    pub fn new<'a>() -> Parser<'a> {
        let factory = Factory::new();
        let mut context = Context::new();
        context.set_namespace("ns", "https://www.omg.org/spec/DMN/20191111/MODEL/");
        Parser { factory, context }
    }

    fn get_text(&self, node: Node, xpath: &str) -> Result<String, DmnError> {
        let b = self.factory.build(xpath)?;
        if let Some(xpath) = b {
            match xpath.evaluate(&self.context, node)? {
                Value::String(s) => Ok(s),
                Value::Nodeset(nodeset) => {
                    let mut buf: String = String::new();
                    for n in nodeset.iter() {
                        buf.push_str(n.string_value().as_str());
                    }
                    return Ok(buf);
                }
                Value::Boolean(b) => Ok(b.to_string()),
                Value::Number(n) => Ok(n.to_string()),
            }
        } else {
            Ok("".to_owned())
        }
    }

    fn get_attribute(&self, node: Node, attr_name: &str) -> Result<String, DmnError> {
        if let Node::Element(e) = node {
            if let Some(v) = e.attribute(attr_name) {
                return Ok(v.value().to_owned());
            }
        }
        return Err(DmnError::NoAttribute(attr_name.to_owned()));
    }

    fn get_first_element_node<'a>(
        &'a self,
        node: Node<'a>,
        xpath: &str,
    ) -> Result<Node<'a>, DmnError> {
        let b = self.factory.build(xpath)?;
        if let Some(xpath) = b {
            match xpath.evaluate(&self.context, node)? {
                Value::Nodeset(nodeset) => {
                    for n in nodeset.iter() {
                        if let Node::Element(_) = n {
                            return Ok(n.clone());
                        }
                    }
                    Err(DmnError::NoElement)
                }
                _ => Err(DmnError::NoElement),
            }
        } else {
            Err(DmnError::NoElement)
        }
    }

    fn get_element_nodes<'a>(
        &'a self,
        node: Node<'a>,
        xpath: &str,
    ) -> Result<Vec<Node<'a>>, DmnError> {
        let b = self.factory.build(xpath)?;
        let mut nodes: Vec<Node> = vec![];
        if let Some(xpath) = b {
            let value = xpath.evaluate(&self.context, node)?;
            match value {
                Value::Nodeset(nodeset) => {
                    for n in nodeset.iter() {
                        if let Node::Element(_) = n {
                            nodes.push(n.clone());
                        }
                    }
                }
                _ => (),
            }
        }
        Ok(nodes)
    }

    fn parse_input(&self, n: Node) -> Result<Input, DmnError> {
        if let Node::Element(_) = n {
            let id = self.get_attribute(n, "id")?;
            let label = self.get_attribute(n, "label").unwrap_or_default();

            let expr_node = self.get_first_element_node(n, "ns:inputExpression")?;
            let input_expr = InputExpression {
                id: self.get_attribute(expr_node, "id")?,
                type_ref: self
                    .get_attribute(expr_node, "typeRef")
                    .unwrap_or("".to_owned()),
                text: self.get_text(expr_node, "ns:text")?,
            };

            Ok(Input {
                id,
                label,
                expression: input_expr,
            })
        } else {
            Err(DmnError::InvalidElement("input".to_owned()))
        }
    }

    fn parse_rule(&self, n: Node) -> Result<Rule, DmnError> {
        let id: String = self.get_attribute(n, "id")?;
        let description = self.get_text(n, "ns:description").unwrap_or("".to_owned());

        let mut input_entries: Vec<RuleInputEntry> = vec![];
        for input_node in self.get_element_nodes(n, "ns:inputEntry")? {
            let input_entry_id = self.get_attribute(input_node, "id")?;
            let text = self.get_text(input_node, "text").unwrap_or("".to_owned());
            input_entries.push(RuleInputEntry {
                id: input_entry_id,
                text,
            });
        }

        let mut output_entries: Vec<RuleOutputEntry> = vec![];
        for output_node in self.get_element_nodes(n, "ns:outputEntry")? {
            let output_entry_id = self.get_attribute(output_node, "id")?;
            let text = self.get_text(output_node, "text").unwrap_or("".to_owned());
            output_entries.push(RuleOutputEntry {
                id: output_entry_id,
                text,
            });
        }

        Ok(Rule {
            id,
            description,
            input_entries,
            output_entries,
        })
    }

    fn parse_output(&self, n: Node) -> Result<Output, DmnError> {
        let id = self.get_attribute(n, "id")?;
        let type_ref = self.get_attribute(n, "typeRef").unwrap_or_default();
        let name = self.get_attribute(n, "name").unwrap_or_default();
        Ok(Output { id, type_ref, name })
    }

    fn parse_decision_table(&self, node: Node) -> Result<DicisionTable, DmnError> {
        if let Node::Element(_) = node {
            let id = self.get_attribute(node, "id")?;
            let hit_policy = self
                .get_attribute(node, "hitPolicy")
                .unwrap_or("UNIQUE".to_owned());

            let mut inputs: Vec<Input> = vec![];
            for input_node in self.get_element_nodes(node, "ns:input")? {
                let input = self.parse_input(input_node)?;
                inputs.push(input);
            }

            let mut outputs: Vec<Output> = vec![];
            for output_node in self.get_element_nodes(node, "ns:output")? {
                let output = self.parse_output(output_node)?;
                outputs.push(output);
            }

            let mut rules: Vec<Rule> = vec![];
            for rule_node in self.get_element_nodes(node, "ns:rule")? {
                let rule = self.parse_rule(rule_node)?;
                rules.push(rule);
            }
            Ok(DicisionTable {
                id,
                hit_policy,
                inputs,
                outputs,
                rules,
            })
        } else {
            Err(DmnError::InvalidElement("decisionTable".to_owned()))
        }
    }

    fn parse_file(&self, path: &str) -> Result<DicisionTable, DmnError> {
        let contents =
            fs::read_to_string(path).or_else(|e| Err(DmnError::IOError(e.to_string())))?;
        let package =
            parser::parse(contents.as_str()).or_else(|e| Err(DmnError::XMLError(e.to_string())))?;
        let doc = package.as_document();
        let dicision_table_node = self.get_first_element_node(
            doc.root().into(),
            "/ns:definitions/ns:decision/ns:decisionTable",
        )?;
        self.parse_decision_table(dicision_table_node)
    }
}

pub fn parse_file(path: &str) {
    let parser = Parser::new();
    let table = parser.parse_file(path).unwrap();
    println!("{:?}", table);
}

#[cfg(test)]
mod test {

    #[test]
    fn test_parse_simple_dmn() {
        super::parse_file("src/fixtures/dmn/simpledish.dmn");
    }
}
