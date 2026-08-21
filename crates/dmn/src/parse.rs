extern crate sxd_document;
extern crate sxd_xpath;

use std::fs;

use sxd_xpath::nodeset::Node;

use crate::types::*;

use anpaiutils::xml::{parse_string as parse_xml_string, XMLQuery, XmlError};
use feel::ast::NodeSyntax;
use feel::eval::Engine;
use feel::parse::{parse, ParseTop};

pub struct Parser<'a> {
    xml_query: XMLQuery<'a>,
}

static DEFAULT_NAMESPACE: &str = "https://www.omg.org/spec/DMN/20191111/MODEL/";

fn parse_allowed_values(text: &str, location: &str) -> Result<Vec<String>, DmnError> {
    if text.is_empty() {
        return Ok(vec![]);
    }

    let node = parse(text, Box::new(Engine::new()), ParseTop::UnaryTests)
        .map_err(|err| DmnError::FEELEval(err.into(), location.to_owned(), text.to_owned()))?;
    match node.syntax.as_ref() {
        NodeSyntax::UnaryTests(elements) => elements
            .iter()
            .map(|element| match element.syntax.as_ref() {
                NodeSyntax::UnaryTest { op, right } if op == "=" => Ok(right.to_string()),
                _ => Err(DmnError::InvalidElement(format!(
                    "{} expression={:?} position=chars: 0, lines: 0, cols: 0 error=allowed values must contain literal values",
                    location, text
                ))),
            })
            .collect(),
        _ => Err(DmnError::InvalidElement(format!(
            "{} expression={:?} position=chars: 0, lines: 0, cols: 0 error=allowed values must contain literal values",
            location, text
        ))),
    }
}

fn validate_feel(text: &str, top: ParseTop, location: String) -> Result<(), DmnError> {
    parse(text, Box::new(Engine::new()), top)
        .map_err(|err| DmnError::FEELEval(err.into(), location, text.to_owned()))?;
    Ok(())
}

fn validate_hit_policy(hit_policy: &str, location: &str) -> Result<(), DmnError> {
    match hit_policy {
        "FIRST" | "UNIQUE" | "COLLECT" | "ANY" | "PRIORITY" => Ok(()),
        _ => Err(DmnError::HitPolicy(format!(
            "{} error=unsupported hit policy {:?}",
            location, hit_policy
        ))),
    }
}

fn input_name(input: &Input) -> &str {
    if !input.label.is_empty() {
        &input.label
    } else if !input.expression.text.is_empty() {
        &input.expression.text
    } else {
        &input.id
    }
}

fn output_name(output: &Output) -> &str {
    if output.name.is_empty() {
        &output.id
    } else {
        &output.name
    }
}

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

    fn parse_output(&self, n: Node, table_location: &str) -> Result<Output, DmnError> {
        let id = self.xml_query.get_attribute(n, "id").map_err(|err| {
            DmnError::Context(
                format!("{} output=<unknown>", table_location),
                Box::new(err.into()),
            )
        })?;
        let type_ref = self
            .xml_query
            .get_attribute(n, "typeRef")
            .unwrap_or_default();
        let name = self.xml_query.get_attribute(n, "name").unwrap_or_default();
        let column_name = if name.is_empty() { &id } else { &name };
        let location = format!("{} output={}", table_location, column_name);
        let allowed_values = match self.xml_query.get_first_element_node(n, "ns:allowedValues") {
            Ok(allowed_values_node) => {
                let text = self
                    .xml_query
                    .get_text(allowed_values_node, "ns:text")
                    .map_err(|err| DmnError::Context(location.clone(), Box::new(err.into())))?;
                parse_allowed_values(&text, &location)?
            }
            Err(XmlError::NoElement(_)) => vec![],
            Err(err) => return Err(err.into()),
        };
        let output = Output {
            id,
            type_ref,
            name,
            allowed_values,
        };
        if output.value_type().is_err() {
            return Err(DmnError::InvalidElement(format!(
                "{} typeRef={:?} error=unsupported output type",
                location, output.type_ref
            )));
        }
        Ok(output)
    }

    fn parse_decision_table(
        &self,
        node: Node,
        decision_id: &str,
    ) -> Result<DecisionTable, DmnError> {
        if let Node::Element(_) = node {
            let id = self.xml_query.get_attribute(node, "id").map_err(|err| {
                DmnError::Context(
                    format!("decision={} table=<unknown>", decision_id),
                    Box::new(err.into()),
                )
            })?;
            let table_location = format!("decision={} table={}", decision_id, id);
            let hit_policy = self
                .xml_query
                .get_attribute(node, "hitPolicy")
                .unwrap_or("FIRST".to_owned());
            validate_hit_policy(&hit_policy, &table_location)?;

            let mut inputs = Vec::new();
            for input_node in self.xml_query.get_child_element_nodes(node, "input") {
                let input_id = self
                    .xml_query
                    .get_attribute(input_node, "id")
                    .unwrap_or_else(|_| "<unknown>".to_owned());
                let location = format!("{} input={}", table_location, input_id);
                let input = self
                    .parse_input(input_node)
                    .map_err(|err| DmnError::Context(location, Box::new(err)))?;
                validate_feel(
                    &input.expression.text,
                    ParseTop::Expression,
                    format!("{} input={}", table_location, input_name(&input)),
                )?;
                inputs.push(input);
            }

            let mut outputs = Vec::new();
            for output_node in self.xml_query.get_child_element_nodes(node, "output") {
                outputs.push(self.parse_output(output_node, &table_location)?);
            }

            let mut rules: Vec<Rule> = vec![];
            for rule_node in self.xml_query.get_child_element_nodes(node, "rule").iter() {
                let rule_id = self
                    .xml_query
                    .get_attribute(*rule_node, "id")
                    .unwrap_or_else(|_| "<unknown>".to_owned());
                let rule_location = format!("{} rule={}", table_location, rule_id);
                let rule = self
                    .parse_rule(*rule_node)
                    .map_err(|err| DmnError::Context(rule_location.clone(), Box::new(err)))?;
                if rule.input_entries.len() != inputs.len() {
                    return Err(DmnError::InvalidElement(format!(
                        "{} error=input entry count {} does not match input column count {}",
                        rule_location,
                        rule.input_entries.len(),
                        inputs.len()
                    )));
                }
                if rule.output_entries.len() != outputs.len() {
                    return Err(DmnError::InvalidElement(format!(
                        "{} error=output entry count {} does not match output column count {}",
                        rule_location,
                        rule.output_entries.len(),
                        outputs.len()
                    )));
                }

                for (input_entry, input) in rule.input_entries.iter().zip(&inputs) {
                    if !input_entry.text.is_empty() {
                        validate_feel(
                            &input_entry.text,
                            ParseTop::UnaryTests,
                            format!("{} input={}", rule_location, input_name(input)),
                        )?;
                    }
                }
                for (output_entry, output) in rule.output_entries.iter().zip(&outputs) {
                    if !output_entry.text.is_empty() {
                        validate_feel(
                            &output_entry.text,
                            ParseTop::Expression,
                            format!("{} output={}", rule_location, output_name(output)),
                        )?;
                    }
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
                Ok(n) => Some(self.parse_decision_table(n, &id)?),
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

    pub fn parse_string(&self, xml: &str) -> Result<Diagram, DmnError> {
        let package = parse_xml_string(xml)?;
        let doc = package.as_document();
        let node = self
            .xml_query
            .get_first_element_node(doc.root().into(), "ns:definitions")?;
        self.parse_diagram(node)
    }

    pub fn parse_file(&self, path: &str) -> Result<Diagram, DmnError> {
        let result = (|| {
            let contents =
                fs::read_to_string(path).map_err(|e| DmnError::IOError(e.to_string()))?;
            self.parse_string(&contents)
        })();
        result.map_err(|err| DmnError::File(path.to_owned(), Box::new(err)))
    }
}

/// Parse a DMN XML document held in memory.
pub fn parse_string(xml: &str) -> Result<Diagram, DmnError> {
    Parser::new().parse_string(xml)
}

/// Parse a DMN XML document from a file.
pub fn parse_file(path: &str) -> Result<Diagram, DmnError> {
    Parser::new().parse_file(path)
}

#[cfg(test)]
mod test {
    fn parse_diagram_xml(xml: &str) -> Result<super::Diagram, super::DmnError> {
        let package = anpaiutils::xml::parse_string(xml)?;
        let parser = super::Parser::new();
        let definitions = parser
            .xml_query
            .get_first_element_node(package.as_document().root().into(), "ns:definitions")?;
        parser.parse_diagram(definitions)
    }

    #[test]
    fn test_parse_simple_dmn() {
        super::parse_file("src/fixtures/dmn/simpledish.dmn").unwrap();
    }

    #[test]
    fn test_parse_output_allowed_values() {
        let diagram = parse_diagram_xml(
            r#"<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/" id="definitions-1">
                <decision id="decision-1">
                    <decisionTable id="table-1" hitPolicy="PRIORITY">
                        <output id="output-1" name="result" typeRef="string">
                            <allowedValues><text>"high", "low"</text></allowedValues>
                        </output>
                    </decisionTable>
                </decision>
            </definitions>"#,
        )
        .unwrap();
        let output = &diagram.decisions[0]
            .decision_table
            .as_ref()
            .unwrap()
            .outputs[0];

        assert_eq!(output.allowed_values, vec![r#""high""#, r#""low""#]);
    }

    #[test]
    fn test_parse_rejects_unsupported_hit_policy() {
        let error = parse_diagram_xml(
            r#"<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/" id="definitions-1">
                <decision id="decision-1">
                    <decisionTable id="table-1" hitPolicy="RULE ORDER" />
                </decision>
            </definitions>"#,
        )
        .unwrap_err();

        assert!(
            matches!(error, super::DmnError::HitPolicy(message) if message.contains("RULE ORDER"))
        );
    }

    #[test]
    fn test_parse_rejects_unsupported_output_type() {
        let error = parse_diagram_xml(
            r#"<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/" id="definitions-1">
                <decision id="decision-1">
                    <decisionTable id="table-1">
                        <output id="output-1" name="result" typeRef="unsupported" />
                    </decisionTable>
                </decision>
            </definitions>"#,
        )
        .unwrap_err();

        assert!(
            matches!(error, super::DmnError::InvalidElement(message) if message.contains("typeRef=\"unsupported\"") && message.contains("unsupported output type"))
        );
    }

    #[test]
    fn test_parse_rejects_rule_output_count_mismatch() {
        let error = parse_diagram_xml(
            r#"<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/" id="definitions-1">
                <decision id="decision-1">
                    <decisionTable id="table-1">
                        <output id="output-1" name="result" typeRef="string" />
                        <rule id="rule-1" />
                    </decisionTable>
                </decision>
            </definitions>"#,
        )
        .unwrap_err();

        assert!(
            matches!(error, super::DmnError::InvalidElement(message) if message.contains("output entry count 0"))
        );
    }

    #[test]
    fn test_parse_rejects_rule_input_count_mismatch() {
        let error = parse_diagram_xml(
            r#"<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/" id="definitions-1">
                <decision id="decision-1">
                    <decisionTable id="table-1">
                        <input id="input-1" label="localMinute">
                            <inputExpression id="input-expression-1" typeRef="number"><text>value</text></inputExpression>
                        </input>
                        <rule id="rule-1" />
                    </decisionTable>
                </decision>
            </definitions>"#,
        )
        .unwrap_err();

        assert!(
            matches!(error, super::DmnError::InvalidElement(message) if message.contains("input entry count 0"))
        );
    }

    #[test]
    fn test_parse_rejects_invalid_input_expression() {
        let error = parse_diagram_xml(
            r#"<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/" id="definitions-1">
                <decision id="decision-1">
                    <decisionTable id="table-1">
                        <input id="input-1" label="localMinute">
                            <inputExpression id="input-expression-1" typeRef="number"><text>]</text></inputExpression>
                        </input>
                    </decisionTable>
                </decision>
            </definitions>"#,
        )
        .unwrap_err();

        let rendered = error.to_string();
        assert!(rendered.contains("decision=decision-1"));
        assert!(rendered.contains("table=table-1"));
        assert!(rendered.contains("input=localMinute"));
        assert!(rendered.contains("expression=\"]\""));
        assert!(rendered.contains("position=chars:"));
        assert!(rendered.contains("error="));
    }

    #[test]
    fn test_parse_rejects_invalid_output_expression() {
        let error = parse_diagram_xml(
            r#"<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/" id="definitions-1">
                <decision id="decision-1">
                    <decisionTable id="table-1">
                        <output id="output-1" name="result" typeRef="string" />
                        <rule id="rule-1">
                            <outputEntry id="output-entry-1"><text>]</text></outputEntry>
                        </rule>
                    </decisionTable>
                </decision>
            </definitions>"#,
        )
        .unwrap_err();

        let rendered = error.to_string();
        assert!(rendered.contains("decision=decision-1"));
        assert!(rendered.contains("table=table-1"));
        assert!(rendered.contains("rule=rule-1"));
        assert!(rendered.contains("output=result"));
        assert!(rendered.contains("expression=\"]\""));
        assert!(rendered.contains("position=chars:"));
        assert!(rendered.contains("error="));
    }

    #[test]
    fn test_parse_rule_error_contains_complete_location() {
        let error = parse_diagram_xml(
            r#"<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/" id="definitions-1">
                <decision id="price_decision">
                    <decisionTable id="price_table">
                        <input id="input-1" label="localMinute">
                            <inputExpression id="input-expression-1" typeRef="number"><text>localMinute</text></inputExpression>
                        </input>
                        <output id="output-1" name="price" typeRef="number" />
                        <rule id="rule_03">
                            <inputEntry id="input-entry-1"><text>&gt;</text></inputEntry>
                            <outputEntry id="output-entry-1"><text>100</text></outputEntry>
                        </rule>
                    </decisionTable>
                </decision>
            </definitions>"#,
        )
        .unwrap_err();

        let rendered = error.to_string();
        assert!(rendered.contains("decision=price_decision"));
        assert!(rendered.contains("table=price_table"));
        assert!(rendered.contains("rule=rule_03"));
        assert!(rendered.contains("input=localMinute"));
        assert!(rendered.contains("expression=\">\""));
        assert!(rendered.contains("position=chars:"));
        assert!(rendered.contains("error="));
    }

    #[test]
    fn test_parse_reports_malformed_xml_with_file() {
        let path = std::env::temp_dir().join(format!("anpai-malformed-{}.dmn", std::process::id()));
        std::fs::write(&path, "<definitions>").unwrap();
        let error = super::Parser::new()
            .parse_file(path.to_str().unwrap())
            .unwrap_err();
        std::fs::remove_file(&path).unwrap();

        let rendered = error.to_string();
        assert!(rendered.contains("file="));
        assert!(rendered.contains("parse XML error"));
        assert!(rendered.contains("parse xml error"));
    }
}
