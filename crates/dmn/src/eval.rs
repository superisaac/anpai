use crate::parse::Parser;
use crate::types::{Decision, DecisionTable, Diagram, DmnError, Output, Rule};
use feel::eval::Engine;
use feel::values::context::Context;
use feel::values::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

fn input_name(input: &crate::types::Input) -> &str {
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

fn table_location(decision_id: &str, table: &DecisionTable) -> String {
    format!("decision={} table={}", decision_id, table.id)
}

fn rule_matched(
    decision_id: &str,
    table: &DecisionTable,
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
            let location = format!(
                "{} rule={} input={}",
                table_location(decision_id, table),
                rule.id,
                input_name(&table.inputs[i])
            );
            DmnError::FEELEval(err, location, input_entry.text.clone())
        })?;
        if !evaluated.bool_value() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn eval_rule_output(
    engine: &mut Box<Engine>,
    decision_id: &str,
    table: &DecisionTable,
    rule: &Rule,
) -> Result<Context, DmnError> {
    let mut output_context = Context::new();
    for (i, output) in table.outputs.iter().enumerate() {
        let output_entry = rule.output_entries[i].clone();
        let output_text = output_entry.text;
        if output_text.is_empty() {
            continue;
        }
        let location = format!(
            "{} rule={} output={}",
            table_location(decision_id, table),
            rule.id,
            output_name(output)
        );
        let output_value = match engine.parse_and_eval(output_text.as_str()) {
            Ok(v) => v,
            Err(err) => return Err(DmnError::FEELEval(err, location.clone(), output_text)),
        };
        validate_output_type(output, &output_value, &location)?;
        output_context.insert(output.name.clone(), output_value);
    }
    Ok(output_context)
}

fn validate_output_type(output: &Output, value: &Value, path: &str) -> Result<(), DmnError> {
    let actual_type = value.data_type();
    let Some(expected_type) = output.value_type()? else {
        return Ok(());
    };

    if actual_type == expected_type {
        Ok(())
    } else {
        Err(DmnError::TypeError(format!(
            "{} typeRef={:?} actualType={} error=output value type mismatch (expected `{}`, got `{}`)",
            path, output.type_ref, actual_type, output.type_ref, actual_type
        )))
    }
}

fn eval_first(
    engine: &mut Box<Engine>,
    decision_id: &str,
    table: &DecisionTable,
    input_values: &[Value],
) -> Result<Context, DmnError> {
    for rule in &table.rules {
        if rule_matched(decision_id, table, rule, engine, input_values)? {
            return eval_rule_output(engine, decision_id, table, rule);
        }
    }
    Ok(Context::new())
}

fn eval_unique(
    engine: &mut Box<Engine>,
    decision_id: &str,
    table: &DecisionTable,
    input_values: &[Value],
) -> Result<Context, DmnError> {
    let mut matched_rule: Option<usize> = None;
    for (rule_idx, rule) in table.rules.iter().enumerate() {
        if rule_matched(decision_id, table, rule, engine, input_values)? {
            if let Some(first_rule_idx) = matched_rule {
                return Err(DmnError::HitPolicy(format!(
                    "{} error=UNIQUE matched multiple rules ({}, {})",
                    table_location(decision_id, table),
                    table.rules[first_rule_idx].id,
                    rule.id
                )));
            }
            matched_rule = Some(rule_idx);
        }
    }

    match matched_rule {
        Some(rule_idx) => eval_rule_output(engine, decision_id, table, &table.rules[rule_idx]),
        None => Ok(Context::new()),
    }
}

fn eval_collect(
    engine: &mut Box<Engine>,
    decision_id: &str,
    table: &DecisionTable,
    input_values: &[Value],
) -> Result<Context, DmnError> {
    let mut collected: Vec<Vec<Value>> = (0..table.outputs.len()).map(|_| Vec::new()).collect();

    for rule in &table.rules {
        if !rule_matched(decision_id, table, rule, engine, input_values)? {
            continue;
        }

        let output_context = eval_rule_output(engine, decision_id, table, rule)?;
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
    decision_id: &str,
    table: &DecisionTable,
    input_values: &[Value],
) -> Result<Context, DmnError> {
    let mut matched_output: Option<(usize, Context)> = None;

    for (rule_idx, rule) in table.rules.iter().enumerate() {
        if !rule_matched(decision_id, table, rule, engine, input_values)? {
            continue;
        }

        let output_context = eval_rule_output(engine, decision_id, table, rule)?;
        if let Some((first_rule_idx, first_output)) = &matched_output {
            if first_output != &output_context {
                return Err(DmnError::HitPolicy(format!(
                    "{} error=ANY matched rules {} and {} with different outputs",
                    table_location(decision_id, table),
                    table.rules[*first_rule_idx].id,
                    rule.id
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
    decision_id: &str,
    table: &DecisionTable,
    output: &crate::types::Output,
    value: &Value,
) -> Result<usize, DmnError> {
    if output.allowed_values.is_empty() {
        return Err(DmnError::HitPolicy(format!(
            "{} output={} error=PRIORITY output has no allowed values",
            table_location(decision_id, table),
            output_name(output)
        )));
    }

    for (priority, allowed_value_text) in output.allowed_values.iter().enumerate() {
        let allowed_value = engine.parse_and_eval(allowed_value_text).map_err(|err| {
            let location = format!(
                "{} output={} allowed_value={}",
                table_location(decision_id, table),
                output_name(output),
                priority
            );
            DmnError::FEELEval(err, location, allowed_value_text.clone())
        })?;
        if &allowed_value == value {
            return Ok(priority);
        }
    }

    Err(DmnError::HitPolicy(format!(
        "{} output={} error=PRIORITY value `{}` is not in allowed values",
        table_location(decision_id, table),
        output_name(output),
        value
    )))
}

fn eval_priority(
    engine: &mut Box<Engine>,
    decision_id: &str,
    table: &DecisionTable,
    input_values: &[Value],
) -> Result<Context, DmnError> {
    let mut selected: Option<(Vec<usize>, Context)> = None;

    for rule in &table.rules {
        if !rule_matched(decision_id, table, rule, engine, input_values)? {
            continue;
        }

        let output_context = eval_rule_output(engine, decision_id, table, rule)?;
        let mut priorities = Vec::with_capacity(table.outputs.len());
        for output in &table.outputs {
            let value = output_context
                .get(output.name.clone())
                .unwrap_or(Value::NullV);
            priorities.push(output_priority(engine, decision_id, table, output, &value)?);
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
    decision_id: &str,
    table: &DecisionTable,
    input_values: &[Value],
) -> Result<Context, DmnError> {
    match table.hit_policy.as_str() {
        "FIRST" => eval_first(engine, decision_id, table, input_values),
        "UNIQUE" => eval_unique(engine, decision_id, table, input_values),
        "COLLECT" => eval_collect(engine, decision_id, table, input_values),
        "ANY" => eval_any(engine, decision_id, table, input_values),
        "PRIORITY" => eval_priority(engine, decision_id, table, input_values),
        policy => Err(DmnError::HitPolicy(format!(
            "{} error=unsupported hit policy {:?}",
            table_location(decision_id, table),
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

    let decision_id = decision.id;
    if let Some(table) = decision.decision_table {
        let mut input_values: Vec<Value> = vec![];
        for input in &table.inputs {
            let input_text = input.expression.text.clone();
            let location = format!(
                "{} input={}",
                table_location(&decision_id, &table),
                input_name(input)
            );
            let input_value = match engine.parse_and_eval(input_text.as_str()) {
                Ok(v) => v,
                Err(err) => return Err(DmnError::FEELEval(err, location, input_text)),
            };
            input_values.push(input_value);
        }

        return eval_decision_table(engine, &decision_id, &table, &input_values);
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

/// Evaluate the default decision in a parsed diagram with an in-memory context.
pub fn eval_diagram(diagram: &Diagram, context: Context) -> Result<Value, DmnError> {
    let mut engine = Box::new(Engine::new());
    engine.load_context(context.entries());
    eval_dmn_diagram(&mut engine, diagram, None)
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
        .map_err(|err| DmnError::File(dmn_path.to_owned(), Box::new(err)))
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
        let decision = decision_with_rules(
            "FIRST",
            vec![rule_with_condition("> 3"), rule_with_condition("< 3")],
        );
        let table = decision.decision_table.as_ref().unwrap();

        assert!(rule_matched(
            &decision.id,
            table,
            &table.rules[0],
            &mut engine,
            &input_values
        )
        .unwrap());
        assert!(!rule_matched(
            &decision.id,
            table,
            &table.rules[1],
            &mut engine,
            &input_values
        )
        .unwrap());
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
                assert_eq!(
                    path,
                    "decision=decision-1 table=table-1 rule=rule-1 input=5"
                );
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
            matches!(error, DmnError::HitPolicy(message) if message.contains("ANY matched rules rule-1 and rule-1 with different outputs"))
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

    #[test]
    fn output_value_must_match_declared_type() {
        let mut decision = decision_with_rules(
            "FIRST",
            vec![rule_with_condition_and_output("> 3", r#""text""#)],
        );
        decision.decision_table.as_mut().unwrap().outputs[0].type_ref = "number".to_owned();
        let mut engine = Box::new(Engine::new());

        let error = eval_decision(&mut engine, decision, &empty_diagram()).unwrap_err();

        assert!(
            matches!(error, DmnError::TypeError(message) if message.contains("expected `number`, got `string`"))
        );
    }
}
