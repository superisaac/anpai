pub mod eval;
pub mod parse;
pub mod types;

pub use eval::eval_diagram;
pub use feel::values::context::Context;
pub use feel::values::value::Value;
pub use parse::{parse_file, parse_string};
pub use types::{Diagram, DmnError};

#[cfg(test)]
mod test {
    use super::{eval_diagram, parse_string, Context, Value};

    #[test]
    fn parses_and_evaluates_dmn_from_memory() {
        let diagram = parse_string(
            r#"<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/" id="definitions-1">
                <decision id="beverage-decision">
                    <decisionTable id="beverage-table">
                        <input id="season-input" label="season">
                            <inputExpression id="season-expression" typeRef="string">
                                <text>season</text>
                            </inputExpression>
                        </input>
                        <output id="beverage-output" name="beverage" typeRef="string" />
                        <rule id="summer-rule">
                            <inputEntry id="summer-condition"><text>"Summer"</text></inputEntry>
                            <outputEntry id="summer-output"><text>"Apple Juice"</text></outputEntry>
                        </rule>
                    </decisionTable>
                </decision>
            </definitions>"#,
        )
        .unwrap();
        let mut context = Context::new();
        context.insert("season".to_owned(), Value::StrV("Summer".to_owned()));

        let result = eval_diagram(&diagram, context).unwrap();

        assert_eq!(result.to_string(), r#"{"beverage":"Apple Juice"}"#);
    }
}
