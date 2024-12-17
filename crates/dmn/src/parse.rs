extern crate sxd_document;
extern crate sxd_xpath;

use std::fs;

use sxd_document::parser;
use sxd_xpath::nodeset::Node;
use sxd_xpath::{Context, Factory, Value};

use crate::types::*;

pub struct Parser<'a> {
    factory: Factory,
    context: Context<'a>,
}

static DEFAULT_NAMESPACE: &str = "https://www.omg.org/spec/DMN/20191111/MODEL/";

impl Parser<'_> {
    pub fn new<'a>() -> Parser<'a> {
        let factory = Factory::new();
        let mut context = Context::new();
        context.set_namespace("ns", DEFAULT_NAMESPACE);
        Parser { factory, context }
    }

    fn get_text(&self, node: Node, xpath: &str) -> Result<String, DmnError> {
        let b = self.factory.build(xpath)?;
        if let Some(xpath) = b {
            match xpath.evaluate(&self.context, node)? {
                Value::String(s) => Ok(s.trim().to_owned()),
                Value::Nodeset(nodeset) => {
                    let mut buf: String = String::new();
                    for n in nodeset.iter() {
                        buf.push_str(n.string_value().as_str());
                    }
                    return Ok(buf.trim().to_owned());
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

    fn parse_child_elements<ElemType>(
        &self,
        node: Node,
        local_name: &str,
        child_fn: fn(&Self, node: Node) -> Result<ElemType, DmnError>,
    ) -> Result<Vec<ElemType>, DmnError> {
        let mut elements: Vec<ElemType> = vec![];
        for child_node in self.get_child_element_nodes(node, local_name) {
            elements.push(child_fn(self, child_node)?);
        }
        Ok(elements)
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

    fn get_child_element_nodes<'a>(&'a self, node: Node<'a>, local_name: &str) -> Vec<Node<'a>> {
        let mut nodes: Vec<Node> = vec![];
        for child_node in node.children() {
            if let Node::Element(elem) = child_node {
                if elem.name().local_part() == local_name {
                    nodes.push(child_node);
                }
            }
        }
        nodes
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
    fn parse_rule_input_entry(&self, node: Node) -> Result<RuleInputEntry, DmnError> {
        let id = self.get_attribute(node, "id")?;
        let text = self
                .get_text(node, "ns:text")
                .unwrap_or("".to_owned());
        Ok(RuleInputEntry {
            id,
            text,
        })
    }

    fn parse_rule_output_entry(&self, node: Node) -> Result<RuleOutputEntry, DmnError> {
        let id = self.get_attribute(node, "id")?;
        let text = self
                .get_text(node, "ns:text")
                .unwrap_or("".to_owned());
        Ok(RuleOutputEntry {
            id,
            text,
        })
    }

    fn parse_rule(&self, n: Node) -> Result<Rule, DmnError> {
        let id: String = self.get_attribute(n, "id")?;
        let description = self.get_text(n, "ns:description").unwrap_or("".to_owned());

        let input_entries = self.parse_child_elements(
            n, 
            "inputEntry", 
            Parser::parse_rule_input_entry)?;

        let output_entries = self.parse_child_elements(
            n, 
            "outputEntry", 
            Parser::parse_rule_output_entry)?;

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

            let inputs = self.parse_child_elements(node, "input", Parser::parse_input)?;
            let outputs = self.parse_child_elements(node, "output", Parser::parse_output)?;

            let mut rules: Vec<Rule> = vec![];
            for (i, rule_node) in self
                .get_child_element_nodes(node, "rule")
                .iter()
                .enumerate()
            {
                let rule = self.parse_rule(*rule_node)?;
                if rule.input_entries.len() != inputs.len() {
                    return Err(DmnError::InvalidElement(format!(
                        "rule({}).inputEntries.len({}) != inputs.len({})",
                        i,
                        rule.input_entries.len(),
                        inputs.len()
                    )));
                }
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

    fn parse_requirements(&self, parent_node: Node) -> Result<Requirements, DmnError> {
        let mut requirements = Requirements {
            required_inputs: vec![],
            required_authorities: vec![],
            required_dicisions: vec![],
        };

        for node in self.get_element_nodes(parent_node, "ns:informationRequirement/ns:requiredDecision")? {
            requirements.required_dicisions.push(self.get_attribute(node, "href")?);
        }

        for node in self.get_element_nodes(parent_node, "ns:informationRequirement/ns:requiredInput")? {
            requirements.required_inputs.push(self.get_attribute(node, "href")?);
        }

        for node in self.get_element_nodes(parent_node, "ns:authorityRequirement/ns:requiredAuthority")? {
            requirements.required_authorities.push(self.get_attribute(node, "href")?);
        }
        
        Ok(requirements)
    }

    fn parse_dicision(&self, node: Node) -> Result<Dicision, DmnError> {
        if let Node::Element(_) = node {
            let id = self.get_attribute(node, "id")?;

            let dicision_table = match self.get_first_element_node(node, "ns:decisionTable") {
                Ok(n) => Some(self.parse_decision_table(n)?),
                Err(DmnError::NoElement) => None,
                Err(err) => return Err(err),
            };

            let requirements = self.parse_requirements(node)?;
            Ok(Dicision {
                id,
                dicision_table,
                requirements,
            })
        } else {
            Err(DmnError::NoElement)
        }
    }

    fn parse_input_data(&self, node: Node) -> Result<InputData, DmnError> {
        if let Node::Element(_) = node {
            let id = self.get_attribute(node, "id")?;
            let name = self.get_attribute(node, "name")?;
            let requirements = self.parse_requirements(node)?;
            Ok(InputData {
                id,
                name,
                requirements,
            })
        } else {
            Err(DmnError::NoElement)
        }
    }

    fn parse_business_knowledge_model(
        &self,
        node: Node,
    ) -> Result<BusinessKnowledgeModel, DmnError> {
        if let Node::Element(_) = node {
            let id = self.get_attribute(node, "id")?;
            let name = self.get_attribute(node, "name")?;
            let requirements = self.parse_requirements(node)?;
            Ok(BusinessKnowledgeModel {
                id,
                name,
                requirements,
            })
        } else {
            Err(DmnError::NoElement)
        }
    }

    fn parse_knowledge_source(&self, node: Node) -> Result<KnowledgeSource, DmnError> {
        if let Node::Element(_) = node {
            let id = self.get_attribute(node, "id")?;
            let name = self.get_attribute(node, "name")?;
            let requirements = self.parse_requirements(node)?;
            Ok(KnowledgeSource {
                id,
                name,
                requirements,
            })
        } else {
            Err(DmnError::NoElement)
        }
    }

    pub fn parse_diagram(&self, node: Node) -> Result<Diagram, DmnError> {
        if let Node::Element(_) = node {
            let id = self.get_attribute(node, "id")?;

            let dicisions = self.parse_child_elements(node, "dicision", Parser::parse_dicision)?;
            let input_datas =
                self.parse_child_elements(node, "inputData", Parser::parse_input_data)?;
            let business_knowledge_models = self.parse_child_elements(
                node,
                "businessKnowledgeModel",
                Parser::parse_business_knowledge_model,
            )?;
            let knowledge_sources =
                self.parse_child_elements(node, "knowledgeSource", Parser::parse_knowledge_source)?;
            Ok(Diagram {
                id,
                dicisions,
                input_datas,
                business_knowledge_models,
                knowledge_sources,
            })
        } else {
            Err(DmnError::NoElement)
        }
    }

    pub fn parse_file(&self, path: &str) -> Result<DicisionTable, DmnError> {
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
