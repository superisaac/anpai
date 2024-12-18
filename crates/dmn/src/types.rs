use std::error;
use std::fmt;

use anpaiutils::xml::XmlError;
use feel::eval::EvalError as FEELEvelError;

// errors
#[derive(Debug, Clone)]
pub enum DmnError {
    NoAttribute(String),
    InvalidElement(String),
    NoElement(String),
    IOError(String),
    XML(XmlError),
    FEELEval(FEELEvelError, String, String),
}
impl error::Error for DmnError {}

impl From<XmlError> for DmnError {
    fn from(err: XmlError) -> DmnError {
        Self::XML(err)
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
            Self::NoElement(elem_name) => write!(f, "no element `{}`", elem_name),
            Self::IOError(error_message) => write!(f, "io error {}", error_message),
            Self::XML(err) => write!(f, "parse XML error {}", err),
            Self::FEELEval(err, path, _) => write!(f, "eval FEEL error at {}, {}", path, err),
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
pub struct DecisionTable {
    pub id: String,
    pub hit_policy: String,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug)]
pub struct Requirements {
    pub required_inputs: Vec<String>,
    pub required_decisions: Vec<String>,
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
pub struct Decision {
    pub id: String,
    pub decision_table: Option<DecisionTable>,
    pub requirements: Requirements,
}

#[derive(Clone, Debug)]
pub struct Diagram {
    pub id: String,
    pub decisions: Vec<Decision>,
    pub input_datas: Vec<InputData>,
    pub business_knowledge_models: Vec<BusinessKnowledgeModel>,
    pub knowledge_sources: Vec<KnowledgeSource>,
}

impl Diagram {
    pub fn find_decision(&self, decision_id: String) -> Result<Decision, DmnError> {
        match self
            .decisions
            .iter()
            .find(|x| format!("#{}", x.id) == decision_id)
        {
            Some(found) => Ok(found.clone()),
            None => Err(DmnError::NoElement(format!(
                "decision[@id={}]",
                decision_id
            ))),
        }
    }
}
