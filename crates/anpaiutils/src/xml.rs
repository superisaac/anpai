use std::error;
use std::fmt;

use sxd_document::parser;
use sxd_document::Package;
use sxd_xpath::nodeset::Node;
use sxd_xpath::{Context, Factory, Value};
use sxd_xpath::{ExecutionError, ParserError};

// errors
#[derive(Debug, Clone)]
pub enum XmlError {
    ParseError(String),
    NoAttribute(String),
    InvalidElement(String),
    NoElement(String),
    XPathParserError(ParserError),
    XPathExecutionError(ExecutionError),
}
impl error::Error for XmlError {}

impl From<ParserError> for XmlError {
    fn from(err: ParserError) -> XmlError {
        Self::XPathParserError(err)
    }
}

impl From<ExecutionError> for XmlError {
    fn from(err: ExecutionError) -> XmlError {
        Self::XPathExecutionError(err)
    }
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ParseError(error_message) => write!(f, "parse xml error {}", error_message),
            Self::NoAttribute(attr_name) => write!(f, "attribute `{}` not found", attr_name),
            Self::InvalidElement(elem_name) => write!(f, "invalid element `{}`", elem_name),
            Self::NoElement(elem_name) => write!(f, "no element `{}`", elem_name),
            Self::XPathParserError(err) => write!(f, "parse xpath error {}", err),
            Self::XPathExecutionError(err) => write!(f, "execute xpath error {}", err),
        }
    }
}

pub struct XMLQuery<'a> {
    //default_namespace: String,
    factory: Factory,
    context: Context<'a>,
}

impl XMLQuery<'_> {
    pub fn new<'a>(default_namespace: &str) -> XMLQuery<'a> {
        let factory = Factory::new();
        let mut context = Context::new();
        context.set_namespace("ns", default_namespace);
        XMLQuery { factory, context }
    }

    pub fn get_text(&self, node: Node, xpath: &str) -> Result<String, XmlError> {
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

    pub fn get_attribute(&self, node: Node, attr_name: &str) -> Result<String, XmlError> {
        if let Node::Element(e) = node {
            if let Some(v) = e.attribute(attr_name) {
                return Ok(v.value().to_owned());
            }
        }
        return Err(XmlError::NoAttribute(attr_name.to_owned()));
    }

    pub fn get_first_element_node<'a>(
        &'a self,
        node: Node<'a>,
        xpath_str: &str,
    ) -> Result<Node<'a>, XmlError> {
        let b = self.factory.build(xpath_str)?;
        if let Some(xpath) = b {
            match xpath.evaluate(&self.context, node)? {
                Value::Nodeset(nodeset) => {
                    for n in nodeset.iter() {
                        if let Node::Element(_) = n {
                            return Ok(n.clone());
                        }
                    }
                    Err(XmlError::NoElement(xpath_str.to_owned()))
                }
                _ => Err(XmlError::NoElement(xpath_str.to_owned())),
            }
        } else {
            Err(XmlError::NoElement(xpath_str.to_owned()))
        }
    }

    pub fn get_element_nodes<'a>(
        &'a self,
        node: Node<'a>,
        xpath: &str,
    ) -> Result<Vec<Node<'a>>, XmlError> {
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

    pub fn get_child_element_nodes<'a>(
        &'a self,
        node: Node<'a>,
        local_name: &str,
    ) -> Vec<Node<'a>> {
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
}

pub fn parse_string(xml_content: &str) -> Result<Package, XmlError> {
    let package =
        parser::parse(xml_content).or_else(|e| Err(XmlError::ParseError(e.to_string())))?;
    return Ok(package);
}
