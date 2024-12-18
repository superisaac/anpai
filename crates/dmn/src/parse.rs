extern crate sxd_document;
extern crate sxd_xpath;

use std::fs;

use sxd_xpath::nodeset::Node;

use crate::types::*;

use anpaiutils::xml::{parse_string, XMLQuery, XmlError};

pub struct Parser<'a> {
    xml_query: XMLQuery<'a>,
}

static DEFAULT_NAMESPACE: &str = "https://www.omg.org/spec/DMN/20191111/MODEL/";

impl Parser<'_> {
    pub fn new<'a>() -> Parser<'a> {
        let xml_query = XMLQuery::new(DEFAULT_NAMESPACE);
        Parser { xml_query }
    }

    pub fn parse_child_elements<ElemType>(
        &self,
        node: Node,
        local_name: &str,
        child_fn: fn(&Self, node: Node) -> Result<ElemType, DmnError>,
    ) -> Result<Vec<ElemType>, DmnError> {
        let mut elements: Vec<ElemType> = vec![];
        for child_node in self.xml_query.get_child_element_nodes(node, local_name) {
            elements.push(child_fn(self, child_node)?);
        }
        Ok(elements)
    }

    fn parse_input(&self, n: Node) -> Result<Input, DmnError> {
        if let Node::Element(_) = n {
            let id = self.xml_query.get_attribute(n, "id")?;
            let label = self.xml_query.get_attribute(n, "label").unwrap_or_default();

            let expr_node = self
                .xml_query
                .get_first_element_node(n, "ns:inputExpression")?;
            let input_expr = InputExpression {
                id: self.xml_query.get_attribute(expr_node, "id")?,
                type_ref: self
                    .xml_query
                    .get_attribute(expr_node, "typeRef")
                    .unwrap_or("".to_owned()),
                text: self.xml_query.get_text(expr_node, "ns:text")?,
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
        let id = self.xml_query.get_attribute(node, "id")?;
        let text = self
            .xml_query
            .get_text(node, "ns:text")
            .unwrap_or("".to_owned());
        Ok(RuleInputEntry { id, text })
    }

    fn parse_rule_output_entry(&self, node: Node) -> Result<RuleOutputEntry, DmnError> {
        let id = self.xml_query.get_attribute(node, "id")?;
        let text = self
            .xml_query
            .get_text(node, "ns:text")
            .unwrap_or("".to_owned());
        Ok(RuleOutputEntry { id, text })
    }

    fn parse_rule(&self, n: Node) -> Result<Rule, DmnError> {
        let id: String = self.xml_query.get_attribute(n, "id")?;
        let description = self
            .xml_query
            .get_text(n, "ns:description")
            .unwrap_or("".to_owned());

        let input_entries =
            self.parse_child_elements(n, "inputEntry", Parser::parse_rule_input_entry)?;

        let output_entries =
            self.parse_child_elements(n, "outputEntry", Parser::parse_rule_output_entry)?;

        Ok(Rule {
            id,
            description,
            input_entries,
            output_entries,
        })
    }

    fn parse_output(&self, n: Node) -> Result<Output, DmnError> {
        let id = self.xml_query.get_attribute(n, "id")?;
        let type_ref = self
            .xml_query
            .get_attribute(n, "typeRef")
            .unwrap_or_default();
        let name = self.xml_query.get_attribute(n, "name").unwrap_or_default();
        Ok(Output { id, type_ref, name })
    }

    fn parse_decision_table(&self, node: Node) -> Result<DecisionTable, DmnError> {
        if let Node::Element(_) = node {
            let id = self.xml_query.get_attribute(node, "id")?;
            let hit_policy = self
                .xml_query
                .get_attribute(node, "hitPolicy")
                .unwrap_or("UNIQUE".to_owned());

            let inputs = self.parse_child_elements(node, "input", Parser::parse_input)?;
            let outputs = self.parse_child_elements(node, "output", Parser::parse_output)?;

            let mut rules: Vec<Rule> = vec![];
            for (i, rule_node) in self
                .xml_query
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
            Ok(DecisionTable {
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
            required_decisions: vec![],
        };

        for node in self
            .xml_query
            .get_element_nodes(parent_node, "ns:informationRequirement/ns:requiredDecision")?
        {
            requirements
                .required_decisions
                .push(self.xml_query.get_attribute(node, "href")?);
        }

        for node in self
            .xml_query
            .get_element_nodes(parent_node, "ns:informationRequirement/ns:requiredInput")?
        {
            requirements
                .required_inputs
                .push(self.xml_query.get_attribute(node, "href")?);
        }

        for node in self
            .xml_query
            .get_element_nodes(parent_node, "ns:authorityRequirement/ns:requiredAuthority")?
        {
            requirements
                .required_authorities
                .push(self.xml_query.get_attribute(node, "href")?);
        }

        Ok(requirements)
    }

    fn parse_decision(&self, node: Node) -> Result<Decision, DmnError> {
        if let Node::Element(_) = node {
            let id = self.xml_query.get_attribute(node, "id")?;
            let decision_table = match self
                .xml_query
                .get_first_element_node(node, "ns:decisionTable")
            {
                Ok(n) => Some(self.parse_decision_table(n)?),
                Err(XmlError::NoElement(_)) => None,
                Err(err) => return Err(err.into()),
            };

            let requirements = self.parse_requirements(node)?;
            Ok(Decision {
                id,
                decision_table,
                requirements,
            })
        } else {
            Err(DmnError::NoElement("decision".to_owned()))
        }
    }

    fn parse_input_data(&self, node: Node) -> Result<InputData, DmnError> {
        let id = self.xml_query.get_attribute(node, "id")?;
        let name = self.xml_query.get_attribute(node, "name")?;
        let requirements = self.parse_requirements(node)?;
        Ok(InputData {
            id,
            name,
            requirements,
        })
    }

    fn parse_business_knowledge_model(
        &self,
        node: Node,
    ) -> Result<BusinessKnowledgeModel, DmnError> {
        let id = self.xml_query.get_attribute(node, "id")?;
        let name = self.xml_query.get_attribute(node, "name")?;
        let requirements = self.parse_requirements(node)?;
        Ok(BusinessKnowledgeModel {
            id,
            name,
            requirements,
        })
    }

    fn parse_knowledge_source(&self, node: Node) -> Result<KnowledgeSource, DmnError> {
        let id = self.xml_query.get_attribute(node, "id")?;
        let name = self.xml_query.get_attribute(node, "name")?;
        let requirements = self.parse_requirements(node)?;
        Ok(KnowledgeSource {
            id,
            name,
            requirements,
        })
    }

    pub fn parse_diagram(&self, node: Node) -> Result<Diagram, DmnError> {
        let id = self.xml_query.get_attribute(node, "id")?;

        let decisions = self.parse_child_elements(node, "decision", Parser::parse_decision)?;
        let input_datas = self.parse_child_elements(node, "inputData", Parser::parse_input_data)?;
        let business_knowledge_models = self.parse_child_elements(
            node,
            "businessKnowledgeModel",
            Parser::parse_business_knowledge_model,
        )?;
        let knowledge_sources =
            self.parse_child_elements(node, "knowledgeSource", Parser::parse_knowledge_source)?;
        Ok(Diagram {
            id,
            decisions,
            input_datas,
            business_knowledge_models,
            knowledge_sources,
        })
    }

    pub fn parse_file(&self, path: &str) -> Result<Diagram, DmnError> {
        let contents =
            fs::read_to_string(path).or_else(|e| Err(DmnError::IOError(e.to_string())))?;
        let package = parse_string(contents.as_str())?;
        let doc = package.as_document();
        let node = self
            .xml_query
            .get_first_element_node(doc.root().into(), "ns:definitions")?;
        self.parse_diagram(node)
    }
}

pub fn parse_file(path: &str) {
    let parser = Parser::new();
    let diagram = parser.parse_file(path).unwrap();
    println!("{:?}", diagram);
}

#[cfg(test)]
mod test {

    #[test]
    fn test_parse_simple_dmn() {
        super::parse_file("src/fixtures/dmn/simpledish.dmn");
    }
}
