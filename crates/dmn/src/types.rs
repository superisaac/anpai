use std::error;
use std::fmt;
use sxd_xpath::{ExecutionError, ParserError};

use feel::eval::EvalError as FEELEvelError;

// errors
#[derive(Debug, Clone)]
pub enum DmnError {
    NoAttribute(String),
    InvalidElement(String),
    NoElement,
    IOError(String),
    XMLError(String),
    XPathParserError(ParserError),
    XPathExecutionError(ExecutionError),
    FEELEvalError(FEELEvelError, String, String),
}
impl error::Error for DmnError {}

impl From<ParserError> for DmnError {
    fn from(err: ParserError) -> DmnError {
        Self::XPathParserError(err)
    }
}

impl From<ExecutionError> for DmnError {
    fn from(err: ExecutionError) -> DmnError {
        Self::XPathExecutionError(err)
    }
}

// impl From<FEELEvelError> for DmnError {
//     fn from(err: FEELEvelError) -> DmnError {
//         Self::FEELEvelError(err)
//     }
// }

impl fmt::Display for DmnError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NoAttribute(name) => write!(f, "attribute {} not found", name),
            Self::InvalidElement(elem_name) => write!(f, "invalid element {}", elem_name),
            Self::NoElement => write!(f, "no element"),
            Self::IOError(error_message) => write!(f, "io error {}", error_message),
            Self::XMLError(error_message) => write!(f, "parse XML error {}", error_message),
            Self::XPathParserError(err) => write!(f, "parse xpath error {}", err),
            Self::XPathExecutionError(err) => write!(f, "execute xpath error {}", err),
            Self::FEELEvalError(err, path, _) => write!(f, "eval FEEL error at {}, {}", path, err),
        }
    }
}

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
    pub expression: InputExpression,
}

#[derive(Clone, Debug)]
pub struct Output {
    pub id: String,
    pub name: String,
    pub type_ref: String,
}

#[derive(Clone, Debug)]
pub struct RuleInputEntry {
    pub id: String,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct RuleOutputEntry {
    pub id: String,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub id: String,
    pub description: String,
    pub input_entries: Vec<RuleInputEntry>,
    pub output_entries: Vec<RuleOutputEntry>,
}

#[derive(Clone, Debug)]
pub struct DicisionTable {
    pub id: String,
    pub hit_policy: String,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug)]
pub struct Requirements {
    pub required_inputs: Vec<String>,
    pub required_dicisions: Vec<String>,
    pub required_authorities: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct InputData {
    pub id: String,
    pub name: String,
    pub requirements: Requirements,
}

#[derive(Clone, Debug)]
pub struct BusinessKnowledgeModel {
    pub id: String,
    pub name: String,
    pub requirements: Requirements,
}

#[derive(Clone, Debug)]
pub struct KnowledgeSource {
    pub id: String,
    pub name: String,
    pub requirements: Requirements,
}

#[derive(Clone, Debug)]
pub struct Dicision {
    pub id: String,
    pub dicision_table: Option<DicisionTable>,
    pub requirements: Requirements,
}

#[derive(Clone, Debug)]
pub struct Diagram {
    pub id: String,
    pub dicisions: Vec<Dicision>,
    pub input_datas: Vec<InputData>,
    pub business_knowledge_models: Vec<BusinessKnowledgeModel>,
    pub knowledge_sources: Vec<KnowledgeSource>,
}
