use crate::parse::Parser;
use crate::types::{DmnError, Rule};
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
pub fn eval_file(engine: &mut Box<Engine>, dmn_path: &str) -> Result<Value, DmnError> {
    let parser = Parser::new();
    let table = parser.parse_file(dmn_path)?;
    let mut input_values: Vec<Value> = vec![];
    for input in table.inputs {
        let input_text = input.expression.text;
        let v = engine.parse_and_eval(input_text.as_str())?;
        input_values.push(v);
    }

    for rule in table.rules.iter() {
        if rule_matched(&rule, engine, &input_values) {
            // render the result
            let mut output_context = Context::new();
            for (i, output) in table.outputs.iter().enumerate() {
                let output_entry = rule.output_entries[i].clone();
                let v = engine.parse_and_eval(output_entry.text.as_str())?;
                output_context.insert(output.name.clone(), v);
            }
            return Ok(Value::ContextV(Rc::new(RefCell::new(output_context))));
        }
    }
    Ok(Value::NullV)
}
