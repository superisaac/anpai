use crate::parse::Parser;
use crate::types::{Decision, Diagram, DmnError, Rule};
use feel::eval::Engine;
use feel::values::context::Context;
use feel::values::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

fn rule_matched(rule: &Rule, engine: &mut Box<Engine>, input_values: &Vec<Value>) -> bool {
    for (i, input_entry) in rule.input_entries.iter().enumerate() {
        if input_entry.text == "" {
            continue;
        }
        let v = input_values[i].clone();
        engine.push_frame();
        engine.set_var("?".to_owned(), v);

        if let Ok(evaluated) = engine.parse_and_eval_unary_tests(input_entry.text.as_str()) {
            engine.pop_frame();
            if !evaluated.bool_value() {
                return false;
            }
        } else {
            engine.pop_frame();
        }
    }
    return true;
}

pub fn eval_decision(
    engine: &mut Box<Engine>,
    decision: Decision,
    diagram: &Diagram,
) -> Result<Context, DmnError> {
    // recursively call required decisions
    for decision_id in decision.requirements.required_decisions.iter() {
        let required = diagram.find_decision(decision_id.clone())?;
        let req_context = eval_decision(engine, required, diagram)?;
        engine.load_context(req_context.entries());
    }

    if let Some(table) = decision.decision_table {
        let mut input_values: Vec<Value> = vec![];
        for (input_idx, input) in table.inputs.iter().enumerate() {
            let input_text = input.expression.text.clone();
            let path = format!("input/{}[@id={}]", input_idx, input.id);
            let input_value = match engine.parse_and_eval(input_text.as_str()) {
                Ok(v) => v,
                Err(err) => return Err(DmnError::FEELEvalError(err, path, input_text)),
            };
            input_values.push(input_value);
        }

        for (rule_idx, rule) in table.rules.iter().enumerate() {
            if rule_matched(&rule, engine, &input_values) {
                // render the result
                let mut output_context = Context::new();
                for (i, output) in table.outputs.iter().enumerate() {
                    let output_entry = rule.output_entries[i].clone();
                    let output_text = output_entry.text;
                    if output_text == "" {
                        continue;
                    }
                    let path = format!(
                        "rule/{}/outputEntry/{}[@id={}]",
                        rule_idx, i, output_entry.id
                    );
                    let output_value = match engine.parse_and_eval(output_text.as_str()) {
                        Ok(v) => v,
                        Err(err) => return Err(DmnError::FEELEvalError(err, path, output_text)),
                    };
                    output_context.insert(output.name.clone(), output_value);
                }
                return Ok(output_context.clone());
            }
        }
    }
    Ok(Context::new())
}

pub fn eval_dmn_diagram(
    engine: &mut Box<Engine>,
    diagram: &Diagram,
    start_decision_id: Option<String>,
) -> Result<Value, DmnError> {
    let decision = match start_decision_id {
        Some(decision_id) => diagram.find_decision(decision_id)?,
        None => match diagram.decisions.last() {
            Some(d) => d.clone(),
            None => return Err(DmnError::NoElement("decision".to_owned())),
        },
    };

    let context = eval_decision(engine, decision, diagram)?;
    return Ok(Value::ContextV(Rc::new(RefCell::new(context))));
}

pub fn eval_file(
    engine: &mut Box<Engine>,
    dmn_path: &str,
    start_decision_id: Option<String>,
) -> Result<Value, DmnError> {
    let parser = Parser::new();
    let diagram = parser.parse_file(dmn_path)?;
    //println!("diagram {:?}", diagram);
    eval_dmn_diagram(engine, &diagram, start_decision_id)
}
