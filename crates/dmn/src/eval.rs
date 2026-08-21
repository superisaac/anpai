use crate::parse::Parser;
use crate::types::{Decision, DecisionTable, Diagram, DmnError, Rule};
use feel::eval::Engine;
use feel::values::context::Context;
use feel::values::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

fn rule_matched(
    rule_idx: usize,
    rule: &Rule,
    engine: &mut Box<Engine>,
    input_values: &[Value],
) -> Result<bool, DmnError> {
    for (i, input_entry) in rule.input_entries.iter().enumerate() {
        if input_entry.text.is_empty() {
            continue;
        }
        let v = input_values[i].clone();
        engine.push_frame();
        engine.set_var("?".to_owned(), v);

        let evaluated = engine.parse_and_eval_unary_tests(input_entry.text.as_str());
        engine.pop_frame();

        let evaluated = evaluated.map_err(|err| {
            let path = format!("rule/{}/inputEntry/{}[@id={}]", rule_idx, i, input_entry.id);
            DmnError::FEELEval(err, path, input_entry.text.clone())
        })?;
        if !evaluated.bool_value() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn eval_rule_output(
    engine: &mut Box<Engine>,
    table: &DecisionTable,
    rule_idx: usize,
    rule: &Rule,
) -> Result<Context, DmnError> {
    let mut output_context = Context::new();
    for (i, output) in table.outputs.iter().enumerate() {
        let output_entry = rule.output_entries[i].clone();
        let output_text = output_entry.text;
        if output_text.is_empty() {
            continue;
        }
        let path = format!(
            "rule/{}/outputEntry/{}[@id={}]",
            rule_idx, i, output_entry.id
        );
        let output_value = match engine.parse_and_eval(output_text.as_str()) {
            Ok(v) => v,
            Err(err) => return Err(DmnError::FEELEval(err, path, output_text)),
        };
        output_context.insert(output.name.clone(), output_value);
    }
    Ok(output_context)
}

fn eval_first(
    engine: &mut Box<Engine>,
    table: &DecisionTable,
    input_values: &[Value],
) -> Result<Context, DmnError> {
    for (rule_idx, rule) in table.rules.iter().enumerate() {
        if rule_matched(rule_idx, rule, engine, input_values)? {
            return eval_rule_output(engine, table, rule_idx, rule);
        }
    }
    Ok(Context::new())
}

fn eval_unique(
    engine: &mut Box<Engine>,
    table: &DecisionTable,
    input_values: &[Value],
) -> Result<Context, DmnError> {
    let mut matched_rule: Option<usize> = None;
    for (rule_idx, rule) in table.rules.iter().enumerate() {
        if rule_matched(rule_idx, rule, engine, input_values)? {
            if let Some(first_rule_idx) = matched_rule {
                return Err(DmnError::HitPolicy(format!(
                    "UNIQUE matched multiple rules ({}, {})",
                    first_rule_idx, rule_idx
                )));
            }
            matched_rule = Some(rule_idx);
        }
    }

    match matched_rule {
        Some(rule_idx) => eval_rule_output(engine, table, rule_idx, &table.rules[rule_idx]),
        None => Ok(Context::new()),
    }
}

fn eval_collect(
    engine: &mut Box<Engine>,
    table: &DecisionTable,
    input_values: &[Value],
) -> Result<Context, DmnError> {
    let mut collected: Vec<Vec<Value>> = (0..table.outputs.len()).map(|_| Vec::new()).collect();

    for (rule_idx, rule) in table.rules.iter().enumerate() {
        if !rule_matched(rule_idx, rule, engine, input_values)? {
            continue;
        }

        let output_context = eval_rule_output(engine, table, rule_idx, rule)?;
        for (output_idx, output) in table.outputs.iter().enumerate() {
            let value = output_context
                .get(output.name.clone())
                .unwrap_or(Value::NullV);
            collected[output_idx].push(value);
        }
    }

    let mut result = Context::new();
    for (output_idx, output) in table.outputs.iter().enumerate() {
        result.insert(
            output.name.clone(),
            Value::ArrayV(Rc::new(RefCell::new(collected[output_idx].clone()))),
        );
    }
    Ok(result)
}

fn eval_any(
    engine: &mut Box<Engine>,
    table: &DecisionTable,
    input_values: &[Value],
) -> Result<Context, DmnError> {
    let mut matched_output: Option<(usize, Context)> = None;

    for (rule_idx, rule) in table.rules.iter().enumerate() {
        if !rule_matched(rule_idx, rule, engine, input_values)? {
            continue;
        }

        let output_context = eval_rule_output(engine, table, rule_idx, rule)?;
        if let Some((first_rule_idx, first_output)) = &matched_output {
            if first_output != &output_context {
                return Err(DmnError::HitPolicy(format!(
                    "ANY matched rules {} and {} with different outputs",
                    first_rule_idx, rule_idx
                )));
            }
        } else {
            matched_output = Some((rule_idx, output_context));
        }
    }

    match matched_output {
        Some((_, output_context)) => Ok(output_context),
        None => Ok(Context::new()),
    }
}

fn output_priority(
    engine: &mut Box<Engine>,
    output: &crate::types::Output,
    output_idx: usize,
    value: &Value,
) -> Result<usize, DmnError> {
    if output.allowed_values.is_empty() {
        return Err(DmnError::HitPolicy(format!(
            "PRIORITY output `{}` has no allowed values",
            output.name
        )));
    }

    for (priority, allowed_value_text) in output.allowed_values.iter().enumerate() {
        let allowed_value = engine.parse_and_eval(allowed_value_text).map_err(|err| {
            let path = format!("output/{}/allowedValues/{}", output_idx, priority);
            DmnError::FEELEval(err, path, allowed_value_text.clone())
        })?;
        if &allowed_value == value {
            return Ok(priority);
        }
    }

    Err(DmnError::HitPolicy(format!(
        "PRIORITY output `{}` value `{}` is not in allowed values",
        output.name, value
    )))
}

fn eval_priority(
    engine: &mut Box<Engine>,
    table: &DecisionTable,
    input_values: &[Value],
) -> Result<Context, DmnError> {
    let mut selected: Option<(Vec<usize>, Context)> = None;

    for (rule_idx, rule) in table.rules.iter().enumerate() {
        if !rule_matched(rule_idx, rule, engine, input_values)? {
            continue;
        }

        let output_context = eval_rule_output(engine, table, rule_idx, rule)?;
        let mut priorities = Vec::with_capacity(table.outputs.len());
        for (output_idx, output) in table.outputs.iter().enumerate() {
            let value = output_context
                .get(output.name.clone())
                .unwrap_or(Value::NullV);
            priorities.push(output_priority(engine, output, output_idx, &value)?);
        }

        let should_select = selected
            .as_ref()
            .map(|(selected_priorities, _)| priorities < *selected_priorities)
            .unwrap_or(true);
        if should_select {
            selected = Some((priorities, output_context));
        }
    }

    Ok(selected
        .map(|(_, output_context)| output_context)
        .unwrap_or_default())
}

fn eval_decision_table(
    engine: &mut Box<Engine>,
    table: &DecisionTable,
    input_values: &[Value],
) -> Result<Context, DmnError> {
    match table.hit_policy.as_str() {
        "FIRST" => eval_first(engine, table, input_values),
        "UNIQUE" => eval_unique(engine, table, input_values),
        "COLLECT" => eval_collect(engine, table, input_values),
        "ANY" => eval_any(engine, table, input_values),
        "PRIORITY" => eval_priority(engine, table, input_values),
        policy => Err(DmnError::HitPolicy(format!(
            "unsupported hit policy `{}`",
            policy
        ))),
    }
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
                Err(err) => return Err(DmnError::FEELEval(err, path, input_text)),
            };
            input_values.push(input_value);
        }

        return eval_decision_table(engine, &table, &input_values);
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
    Ok(Value::ContextV(Rc::new(RefCell::new(context))))
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

#[cfg(test)]
mod test {
    use super::{eval_decision, eval_file, rule_matched};
    use crate::types::{
        Decision, DecisionTable, Diagram, DmnError, Input, InputExpression, Output, Requirements,
        Rule, RuleInputEntry, RuleOutputEntry,
    };
    use feel::eval::Engine;
    use feel::values::numeric::Numeric;
    use feel::values::value::Value;

    fn empty_requirements() -> Requirements {
        Requirements {
            required_inputs: vec![],
            required_decisions: vec![],
            required_authorities: vec![],
        }
    }

    fn rule_with_condition(text: &str) -> Rule {
        Rule {
            id: "rule-1".to_owned(),
            description: String::new(),
            input_entries: vec![RuleInputEntry {
                id: "input-entry-1".to_owned(),
                text: text.to_owned(),
            }],
            output_entries: vec![],
        }
    }

    fn rule_with_condition_and_output(condition: &str, output: &str) -> Rule {
        Rule {
            output_entries: vec![RuleOutputEntry {
                id: "output-entry-1".to_owned(),
                text: output.to_owned(),
            }],
            ..rule_with_condition(condition)
        }
    }

    fn decision_with_rules(hit_policy: &str, rules: Vec<Rule>) -> Decision {
        Decision {
            id: "decision-1".to_owned(),
            decision_table: Some(DecisionTable {
                id: "table-1".to_owned(),
                hit_policy: hit_policy.to_owned(),
                inputs: vec![Input {
                    id: "input-1".to_owned(),
                    label: String::new(),
                    expression: InputExpression {
                        id: "input-expression-1".to_owned(),
                        type_ref: "number".to_owned(),
                        text: "5".to_owned(),
                    },
                }],
                outputs: vec![Output {
                    id: "output-1".to_owned(),
                    name: "result".to_owned(),
                    type_ref: "string".to_owned(),
                    allowed_values: vec![],
                }],
                rules,
            }),
            requirements: empty_requirements(),
        }
    }

    fn priority_decision(rules: Vec<Rule>, allowed_values: Vec<&str>) -> Decision {
        let mut decision = decision_with_rules("PRIORITY", rules);
        decision.decision_table.as_mut().unwrap().outputs[0].allowed_values =
            allowed_values.into_iter().map(str::to_owned).collect();
        decision
    }

    fn empty_diagram() -> Diagram {
        Diagram {
            id: "diagram-1".to_owned(),
            decisions: vec![],
            input_datas: vec![],
            business_knowledge_models: vec![],
            knowledge_sources: vec![],
        }
    }

    #[test]
    fn rule_condition_true_matches_and_false_does_not_match() {
        let mut engine = Box::new(Engine::new());
        let input_values = [Value::NumberV(Numeric::from_i32(5))];

        assert!(rule_matched(0, &rule_with_condition("> 3"), &mut engine, &input_values).unwrap());
        assert!(!rule_matched(0, &rule_with_condition("< 3"), &mut engine, &input_values).unwrap());
    }

    #[test]
    fn invalid_rule_condition_is_returned_by_eval_decision() {
        let rule = rule_with_condition(">");
        let decision = Decision {
            id: "decision-1".to_owned(),
            decision_table: Some(DecisionTable {
                id: "table-1".to_owned(),
                hit_policy: "UNIQUE".to_owned(),
                inputs: vec![Input {
                    id: "input-1".to_owned(),
                    label: String::new(),
                    expression: InputExpression {
                        id: "input-expression-1".to_owned(),
                        type_ref: "number".to_owned(),
                        text: "5".to_owned(),
                    },
                }],
                outputs: vec![],
                rules: vec![rule],
            }),
            requirements: empty_requirements(),
        };
        let diagram = Diagram {
            id: "diagram-1".to_owned(),
            decisions: vec![],
            input_datas: vec![],
            business_knowledge_models: vec![],
            knowledge_sources: vec![],
        };
        let mut engine = Box::new(Engine::new());

        let error = eval_decision(&mut engine, decision, &diagram).unwrap_err();

        match error {
            DmnError::FEELEval(_, path, text) => {
                assert_eq!(path, "rule/0/inputEntry/0[@id=input-entry-1]");
                assert_eq!(text, ">");
            }
            other => panic!("expected FEEL evaluation error, got {other:?}"),
        }
        assert!(engine.resolve("?".to_owned()).is_none());
    }

    #[test]
    fn evaluates_boolean_rule_conditions() {
        let mut engine = Box::new(Engine::new());
        engine
            .load_context_string(r#"{season: "Summer", guestCount: 10, guestsWithChildren: true}"#)
            .unwrap();

        let result = eval_file(&mut engine, "src/fixtures/dmn/simpledish.dmn", None).unwrap();

        assert_eq!(result.to_string(), r#"{"Beverages":"Apple Juice"}"#);
    }

    #[test]
    fn first_returns_the_first_matching_rule() {
        let decision = decision_with_rules(
            "FIRST",
            vec![
                rule_with_condition_and_output("> 3", r#""first""#),
                rule_with_condition_and_output("> 3", r#""second""#),
            ],
        );
        let mut engine = Box::new(Engine::new());

        let result = eval_decision(&mut engine, decision, &empty_diagram()).unwrap();

        assert_eq!(result.to_string(), r#"{"result":"first"}"#);
    }

    #[test]
    fn unique_rejects_multiple_matching_rules() {
        let decision = decision_with_rules(
            "UNIQUE",
            vec![
                rule_with_condition_and_output("> 3", r#""first""#),
                rule_with_condition_and_output("> 3", r#""second""#),
            ],
        );
        let mut engine = Box::new(Engine::new());

        let error = eval_decision(&mut engine, decision, &empty_diagram()).unwrap_err();

        assert!(
            matches!(error, DmnError::HitPolicy(message) if message.contains("UNIQUE matched multiple rules"))
        );
    }

    #[test]
    fn collect_returns_all_matching_outputs() {
        let decision = decision_with_rules(
            "COLLECT",
            vec![
                rule_with_condition_and_output("> 3", r#""first""#),
                rule_with_condition_and_output("> 3", r#""second""#),
                rule_with_condition_and_output("< 3", r#""ignored""#),
            ],
        );
        let mut engine = Box::new(Engine::new());

        let result = eval_decision(&mut engine, decision, &empty_diagram()).unwrap();

        assert_eq!(result.to_string(), r#"{"result":["first", "second"]}"#);
    }

    #[test]
    fn collect_returns_empty_arrays_when_no_rule_matches() {
        let decision = decision_with_rules(
            "COLLECT",
            vec![rule_with_condition_and_output("< 3", r#""ignored""#)],
        );
        let mut engine = Box::new(Engine::new());

        let result = eval_decision(&mut engine, decision, &empty_diagram()).unwrap();

        assert_eq!(result.to_string(), r#"{"result":[]}"#);
    }

    #[test]
    fn any_accepts_matching_rules_with_equal_outputs() {
        let decision = decision_with_rules(
            "ANY",
            vec![
                rule_with_condition_and_output("> 3", r#""same""#),
                rule_with_condition_and_output("> 3", r#""same""#),
            ],
        );
        let mut engine = Box::new(Engine::new());

        let result = eval_decision(&mut engine, decision, &empty_diagram()).unwrap();

        assert_eq!(result.to_string(), r#"{"result":"same"}"#);
    }

    #[test]
    fn any_rejects_matching_rules_with_different_outputs() {
        let decision = decision_with_rules(
            "ANY",
            vec![
                rule_with_condition_and_output("> 3", r#""first""#),
                rule_with_condition_and_output("> 3", r#""second""#),
            ],
        );
        let mut engine = Box::new(Engine::new());

        let error = eval_decision(&mut engine, decision, &empty_diagram()).unwrap_err();

        assert!(
            matches!(error, DmnError::HitPolicy(message) if message.contains("ANY matched rules 0 and 1 with different outputs"))
        );
    }

    #[test]
    fn priority_returns_highest_priority_output() {
        let decision = priority_decision(
            vec![
                rule_with_condition_and_output("> 3", r#""low""#),
                rule_with_condition_and_output("> 3", r#""high""#),
            ],
            vec![r#""high""#, r#""low""#],
        );
        let mut engine = Box::new(Engine::new());

        let result = eval_decision(&mut engine, decision, &empty_diagram()).unwrap();

        assert_eq!(result.to_string(), r#"{"result":"high"}"#);
    }

    #[test]
    fn priority_rejects_output_outside_allowed_values() {
        let decision = priority_decision(
            vec![rule_with_condition_and_output("> 3", r#""medium""#)],
            vec![r#""high""#, r#""low""#],
        );
        let mut engine = Box::new(Engine::new());

        let error = eval_decision(&mut engine, decision, &empty_diagram()).unwrap_err();

        assert!(
            matches!(error, DmnError::HitPolicy(message) if message.contains("value `\"medium\"` is not in allowed values"))
        );
    }
}
