use lazy_static::lazy_static;

use rust_decimal::prelude::*;

use std::cmp;
use std::collections::HashMap;

use super::eval::{EvalError, EvalResult};
use super::helpers::sqrt;
use super::values::func::{MacroBody, MacroT, NativeFunc, NativeFuncBody};
use super::values::value::Value::{self, *};

#[derive(Clone)]
pub struct Prelude {
    vars: HashMap<String, Value>,
}

impl Prelude {
    pub fn new() -> Prelude {
        Prelude {
            vars: HashMap::new(),
        }
    }

    pub fn set_var(&mut self, name: String, value: Value) {
        self.vars.insert(name, value);
    }

    pub fn resolve(&self, name: String) -> Option<Value> {
        match self.vars.get(&name) {
            Some(v) => Some(v.clone()),
            None => None,
        }
    }
    pub fn add_macro(&mut self, name: &str, require_args: &[&str], body: MacroBody) {
        let require_args_vec = require_args.into_iter().map(|s| String::from(*s)).collect();
        let macro_ = MacroT {
            name: name.to_owned(),
            body,
        };
        let macro_value = MacroV {
            macro_,
            require_args: require_args_vec,
        };
        self.set_var(name.to_owned(), macro_value);
    }

    pub fn add_native_func(&mut self, name: &str, require_args: &[&str], func: NativeFuncBody) {
        let require_arg_vec = require_args.into_iter().map(|&s| String::from(s)).collect();
        let func_t = NativeFunc {
            name: name.to_owned(),
            body: func,
        };
        let func_value = NativeFuncV {
            func: func_t,
            require_args: require_arg_vec,
            optional_args: Vec::new(),
            var_arg: None,
        };
        self.set_var(name.to_owned(), func_value);
    }

    pub fn add_native_func_with_optional_args(
        &mut self,
        name: &str,
        require_args: &[&str],
        optional_args: &[&str],
        var_arg: Option<&str>,
        func: NativeFuncBody,
    ) {
        let func_t = NativeFunc {
            name: name.to_owned(),
            body: func,
        };
        let func_value = NativeFuncV {
            func: func_t,
            require_args: require_args.into_iter().map(|&s| String::from(s)).collect(),
            optional_args: optional_args
                .into_iter()
                .map(|&s| String::from(s))
                .collect(),
            var_arg: var_arg.map(|a| a.to_owned()),
        };
        self.set_var(name.to_owned(), func_value);
    }

    pub fn load_preludes(&mut self) {
        self.add_native_func("set", &["name", "value"], |eng, args| -> EvalResult {
            let name_node = args.get(&"name".to_owned()).unwrap();
            let var_name = match name_node {
                StrV(value) => value.clone(),
                _ => return Err(EvalError::runtime("argument name should be string")),
            };
            let value = args.get(&"value".to_owned()).unwrap();
            eng.set_var(var_name, value.clone());
            Ok(value.clone())
        });

        self.add_native_func("bind", &["name", "value"], |eng, args| -> EvalResult {
            let name_node = args.get(&"name".to_owned()).unwrap();
            let var_name = match name_node {
                StrV(value) => value.clone(),
                _ => return Err(EvalError::runtime("argument name should be string")),
            };
            let value = args.get(&"value".to_owned()).unwrap();
            eng.bind_var(var_name, value.clone());
            Ok(value.clone())
        });

        self.add_macro("is defined", &["value"], |eng, nodes| -> EvalResult {
            let value_node = nodes.get(&"value".to_owned()).unwrap();
            eng.is_defined(value_node)
        });

        // conversion functions
        self.add_native_func("string", &["from"], |_, args| -> EvalResult {
            let v = args.get(&"from".to_owned()).unwrap();
            Ok(Value::StrV(v.to_string()))
        });

        self.add_native_func("number", &["from"], |_, args| -> EvalResult {
            let v = args.get(&"from".to_owned()).unwrap();
            let n = v.parse_number()?;
            Ok(Value::NumberV(n))
        });

        self.add_native_func("not", &["from"], |_, args| -> EvalResult {
            let v = args.get(&"from".to_owned()).unwrap();
            Ok(Value::BoolV(!v.bool_value()))
        });

        // string functions
        self.add_native_func("string length", &["string"], |_, args| -> EvalResult {
            let v = args.get(&"string".to_owned()).unwrap();
            let s = v.expect_string("argument[1]")?;
            let lenn = Decimal::from_usize(s.len()).unwrap();
            Ok(Value::NumberV(lenn))
        });

        self.add_native_func_with_optional_args(
            "substring",
            &["string", "start position"],
            &["length"],
            None,
            |_, args| -> EvalResult {
                let v = args.get(&"string".to_owned()).unwrap();
                let s = v.expect_string("argument[1] `string`")?;
                let start_v = args.get(&"start position".to_owned()).unwrap();
                let start_position = start_v.expect_usize("argument[2] `start position`")?;
                if start_position < 1 || start_position > s.len() {
                    return Ok(Value::StrV("".to_owned()));
                }
                // 'length' is the optional value
                let substr = if let Some(lenv) = args.get(&"length".to_owned()) {
                    let len = lenv.expect_usize("argument[3] `length`")?;
                    &s.as_str()[(start_position - 1)..(cmp::min(start_position - 1 + len, s.len()))]
                } else {
                    &s.as_str()[(start_position - 1)..]
                };
                Ok(Value::StrV(substr.to_owned()))
            },
        );

        self.add_native_func("upper case", &["string"], |_, args| -> EvalResult {
            let v = args.get(&"string".to_owned()).unwrap();
            let s = v.expect_string("argument[1] `string`")?;
            Ok(Value::StrV(s.to_uppercase()))
        });

        self.add_native_func("lower case", &["string"], |_, args| -> EvalResult {
            let v = args.get(&"string".to_owned()).unwrap();
            let s = v.expect_string("argument[1] `string`")?;
            Ok(Value::StrV(s.to_lowercase()))
        });

        self.add_native_func("contains", &["string", "match"], |_, args| -> EvalResult {
            let v = args.get(&"string".to_owned()).unwrap();
            let s = v.expect_string("argument[1] `string`")?;
            let mv = args.get(&"match".to_owned()).unwrap();
            let match_s = mv.expect_string("argument[2] `match`")?;
            Ok(Value::BoolV(s.contains(match_s.as_str())))
        });

        self.add_native_func(
            "starts with",
            &["string", "match"],
            |_, args| -> EvalResult {
                let v = args.get(&"string".to_owned()).unwrap();
                let s = v.expect_string("argument[1] `string`")?;
                let mv = args.get(&"match".to_owned()).unwrap();
                let match_s = mv.expect_string("argument[2] `match`")?;
                Ok(Value::BoolV(s.starts_with(match_s.as_str())))
            },
        );

        self.add_native_func("ends with", &["string", "match"], |_, args| -> EvalResult {
            let v = args.get(&"string".to_owned()).unwrap();
            let s = v.expect_string("argument[1] `string`")?;
            let mv = args.get(&"match".to_owned()).unwrap();
            let match_s = mv.expect_string("argument[2] `match`")?;
            Ok(Value::BoolV(s.ends_with(match_s.as_str())))
        });

        // list functions
        self.add_native_func(
            "list contains",
            &["list", "element"],
            |_, args| -> EvalResult {
                let v = args.get(&"list".to_owned()).unwrap();
                let elem = args.get(&"element".to_owned()).unwrap();
                let arr = v.expect_array()?;
                for arr_elem in arr.iter() {
                    if *arr_elem == *elem {
                        return Ok(Value::BoolV(true));
                    }
                }
                Ok(Value::BoolV(false))
            },
        );

        self.add_native_func_with_optional_args(
            "count",
            &[],
            &[],
            Some("list"),
            |_, args| -> EvalResult {
                let v = args.get(&"list".to_owned()).unwrap();
                let arr = v.expect_array()?;
                let count = Decimal::from_usize(arr.len()).unwrap();
                Ok(Value::NumberV(count))
            },
        );

        self.add_native_func_with_optional_args(
            "min",
            &[],
            &[],
            Some("list"),
            |_, args| -> EvalResult {
                let arg0 = args.get(&"list".to_owned()).unwrap();
                let arr = arg0.expect_array()?;
                let mut min_value: Option<Value> = None;

                for v in arr.iter() {
                    if min_value.is_none() || *v < min_value.clone().unwrap() {
                        min_value = Some(v.clone())
                    }
                }
                Ok(min_value.unwrap_or(Value::NullV))
            },
        );

        self.add_native_func_with_optional_args(
            "max",
            &[],
            &[],
            Some("list"),
            |_, args| -> EvalResult {
                let arg0 = args.get(&"list".to_owned()).unwrap();
                let arr = arg0.expect_array()?;
                let mut max_value: Option<Value> = None;

                for v in arr.iter() {
                    if max_value.is_none() || *v > max_value.clone().unwrap() {
                        max_value = Some(v.clone())
                    }
                }
                Ok(max_value.unwrap_or(Value::NullV))
            },
        );

        self.add_native_func_with_optional_args(
            "sum",
            &[],
            &[],
            Some("list"),
            |_, args| -> EvalResult {
                let arg0 = args.get(&"list".to_owned()).unwrap();
                let arr = arg0.expect_array()?;
                let mut sum: Decimal = Decimal::zero();

                for v in arr.iter() {
                    if let Value::NumberV(v) = *v {
                        sum += v;
                    }
                }
                Ok(Value::NumberV(sum))
            },
        );

        self.add_native_func_with_optional_args(
            "product",
            &[],
            &[],
            Some("list"),
            |_, args| -> EvalResult {
                let arg0 = args.get(&"list".to_owned()).unwrap();
                let arr = arg0.expect_array()?;
                let mut res: Decimal = Decimal::one();

                for v in arr.iter() {
                    if let Value::NumberV(v) = *v {
                        res *= v;
                    }
                }
                Ok(Value::NumberV(res))
            },
        );

        self.add_native_func_with_optional_args(
            "mean",
            &[],
            &[],
            Some("list"),
            |_, args| -> EvalResult {
                let arg0 = args.get(&"list".to_owned()).unwrap();
                let arr = arg0.expect_array()?;
                let mut sum: Decimal = Decimal::zero();
                let mut count = 0;

                for v in arr.iter() {
                    if let Value::NumberV(v) = *v {
                        sum += v;
                        count += 1;
                    }
                }
                if count == 0 {
                    Ok(Value::NullV)
                } else {
                    let cnt = Decimal::from_i32(count).unwrap();
                    Ok(Value::NumberV(sum / cnt))
                }
            },
        );

        self.add_native_func_with_optional_args(
            "stddev",
            &[],
            &[],
            Some("list"),
            |_, args| -> EvalResult {
                let arg0 = args.get(&"list".to_owned()).unwrap();
                let arr = arg0.expect_array()?;
                let mut sum: Decimal = Decimal::ZERO;
                let mut count = 0;
                for v in arr.iter() {
                    if let Value::NumberV(v) = *v {
                        sum += v;
                        count += 1;
                    }
                }
                if count == 0 {
                    return Ok(Value::NullV);
                }
                let avg = sum / Decimal::from_i32(count).unwrap();

                let mut dev = Decimal::ZERO;
                for v in arr.iter() {
                    if let Value::NumberV(v) = *v {
                        dev += (v - avg) * (v - avg);
                    }
                }
                dev = dev / Decimal::from_i32(count).unwrap();
                sqrt(dev).map_or(Ok(NullV), |n| Ok(NumberV(n)))
            },
        );

        self.add_native_func_with_optional_args(
            "median",
            &[],
            &[],
            Some("list"),
            |_, args| -> EvalResult {
                let arg0 = args.get(&"list".to_owned()).unwrap();
                let arr = arg0.expect_array()?;
                let mut value_arr: Vec<Decimal> = vec![];

                for v in arr.iter() {
                    if let Value::NumberV(v) = *v {
                        value_arr.push(v);
                    }
                }
                value_arr.sort();
                match value_arr.len() {
                    0 => Ok(NullV),
                    1 => Ok(NumberV(value_arr[0])),
                    x if x % 2 == 0 => Ok(NumberV(value_arr[x / 2])),
                    y => Ok(NumberV(
                        (value_arr[y / 2] + value_arr[(y / 2) + 1]) / Decimal::TWO,
                    )),
                }
            },
        );
    }
}

lazy_static! {
    pub static ref PRELUDE: Prelude = {
        let mut p = Prelude::new();
        p.load_preludes();
        p
    };
}
