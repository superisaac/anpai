use lazy_static::lazy_static;

use std::borrow::Borrow;
use std::cell::RefCell;
use std::cmp;
use std::collections::HashMap;
use std::rc::Rc;

use super::eval::{EvalError, EvalResult};
use super::values::context::Context;
use super::values::func::{MacroBody, MacroT, NativeFunc, NativeFuncBody};
use super::values::numeric::Numeric;
use super::values::value::Value::{self, *};

fn from_feel_index(idx: usize) -> usize {
    idx - 1
}

fn to_feel_index(idx: usize) -> usize {
    idx + 1
}

pub fn range_check(pos: usize, low: usize, high: usize) -> Result<usize, EvalError> {
    if pos < low || pos > high {
        Err(EvalError::IndexError)
    } else {
        Ok(pos)
    }
}

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
            let lenn = Numeric::from_usize(s.len());
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

        self.add_native_func_with_optional_args(
            "string join",
            &["list"],
            &["delimiter", "prefix", "suffix"],
            None,
            |_, args| -> EvalResult {
                let arg0 = args.get(&"list".to_owned()).unwrap();
                let arr = arg0.expect_array("argument[1] `list`")?;

                let arg1 = args
                    .get(&"delimiter".to_owned())
                    .map_or(Value::from_str(""), |v| v.clone());
                let delimiter = arg1.expect_string("argument[2] `delimiter`")?;

                let arg2 = args
                    .get(&"prefix".to_owned())
                    .map_or(Value::from_str(""), |v| v.clone());
                let prefix = arg2.expect_string("argument[2] `delimiter`")?;

                let arg3 = args
                    .get(&"suffix".to_owned())
                    .map_or(Value::from_str(""), |v| v.clone());
                let suffix = arg3.expect_string("argument[2] `delimiter`")?;

                let mut res = String::new();
                res.push_str(prefix.as_str());

                for (i, v) in arr.iter().enumerate() {
                    let sv = v.expect_string(format!("argument[1][{}]", i + 1).as_str())?;
                    if i > 0 {
                        res.push_str(delimiter.as_str());
                    }
                    res.push_str(sv.as_str());
                }
                res.push_str(suffix.as_str());
                Ok(Value::StrV(res))
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
                let arr = v.expect_array("argument[1] `list`")?;

                let elem = args.get(&"element".to_owned()).unwrap();
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
                let arr = v.expect_array("arguments `list`")?;
                let count = Numeric::from_usize(arr.len());
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
                let arr = arg0.expect_array("arguments `list`")?;
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
                let arr = arg0.expect_array("arguments `list`")?;
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
                let arr = arg0.expect_array("arguments `list`")?;
                let mut sum: Numeric = Numeric::ZERO;

                for v in arr.iter() {
                    if let Value::NumberV(v) = v {
                        sum += v.clone();
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
                let arr = arg0.expect_array("arguments `list`")?;
                let mut res = Numeric::ONE;

                for v in arr.iter() {
                    if let Value::NumberV(v) = v {
                        res *= v.clone();
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
                let arr = arg0.expect_array("arguments `list`")?;
                let mut sum = Numeric::ZERO;
                let mut count = 0;

                for v in arr.iter() {
                    if let Value::NumberV(v) = v {
                        sum += v.clone();
                        count += 1;
                    }
                }
                if count == 0 {
                    Ok(Value::NullV)
                } else {
                    let cnt = Numeric::from_i32(count);
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
                let arr = arg0.expect_array("arguments `list`")?;
                let mut sum = Numeric::ZERO;
                let mut count = 0;
                for v in arr.iter() {
                    if let Value::NumberV(v) = v {
                        sum += v.clone();
                        count += 1;
                    }
                }
                if count == 0 {
                    return Ok(Value::NullV);
                }
                let avg = sum / Numeric::from_i32(count);

                let mut dev = Numeric::ZERO;
                for v in arr.iter() {
                    if let Value::NumberV(v) = v {
                        let diff = v.clone() - avg.clone();
                        dev += diff.clone() * diff;
                    }
                }
                dev = dev / Numeric::from_i32(count);
                dev.sqrt().map_or(Ok(NullV), |n| Ok(NumberV(n)))
            },
        );

        self.add_native_func_with_optional_args(
            "median",
            &[],
            &[],
            Some("list"),
            |_, args| -> EvalResult {
                let arg0 = args.get(&"list".to_owned()).unwrap();
                let arr = arg0.expect_array("arguments `list`")?;
                let mut value_arr: Vec<Numeric> = vec![];

                for v in arr.iter() {
                    if let Value::NumberV(v) = v {
                        value_arr.push(v.clone());
                    }
                }
                value_arr.sort();
                match value_arr.len() {
                    0 => Ok(NullV),
                    1 => Ok(NumberV(value_arr[0].clone())),
                    x if x % 2 == 1 => Ok(NumberV(value_arr[x / 2].clone())),
                    y => {
                        let half = y / 2;
                        Ok(NumberV(
                            (value_arr[half - 1].clone() + value_arr[half].clone()) / Numeric::TWO,
                        ))
                    }
                }
            },
        );

        self.add_native_func_with_optional_args(
            "all",
            &[],
            &[],
            Some("list"),
            |_, args| -> EvalResult {
                let arg0 = args.get(&"list".to_owned()).unwrap();
                let arr = arg0.expect_array("arguments `list`")?;

                for v in arr.iter() {
                    if !v.bool_value() {
                        return Ok(BoolV(false));
                    }
                }
                Ok(BoolV(true))
            },
        );

        self.add_native_func_with_optional_args(
            "any",
            &[],
            &[],
            Some("list"),
            |_, args| -> EvalResult {
                let arg0 = args.get(&"list".to_owned()).unwrap();
                let arr = arg0.expect_array("arguments `list`")?;

                for v in arr.iter() {
                    if v.bool_value() {
                        return Ok(BoolV(true));
                    }
                }
                Ok(BoolV(false))
            },
        );

        self.add_native_func_with_optional_args(
            "sublist",
            &["list", "start position"],
            &["length"],
            None,
            |_, args| -> EvalResult {
                let arg0 = args.get(&"list".to_owned()).unwrap();
                let arr = arg0.expect_array("argument[1] `list`")?;

                let start_v = args.get(&"start position".to_owned()).unwrap();
                let feel_start_position = range_check(
                    start_v.expect_usize("argument[2] `start position`")?,
                    1,
                    arr.len(),
                )?;
                // 'length' is the optional value
                let start_pos = from_feel_index(feel_start_position);
                let subarr = if let Some(lenv) = args.get(&"length".to_owned()) {
                    let len = lenv.expect_usize("argument[3] `length`")?;
                    arr[start_pos..(cmp::min(start_pos + len, arr.len()))].to_owned()
                } else {
                    arr[start_pos..].to_owned()
                };
                Ok(Value::ArrayV(Rc::new(RefCell::new(subarr))))
            },
        );

        self.add_native_func_with_optional_args(
            "append",
            &["list"],
            &[],
            Some("items"),
            |_, args| -> EvalResult {
                let arg0 = args.get(&"list".to_owned()).unwrap();
                let arr = arg0.expect_array("argument[1], `list`")?;

                let vararg = args.get(&"items".to_owned()).unwrap();
                let items = vararg.expect_array("arguments `items`")?;

                let mut res: Vec<Value> = vec![];

                for v in arr.iter() {
                    res.push(v.clone());
                }
                for v in items.iter() {
                    res.push(v.clone());
                }
                Ok(Value::ArrayV(Rc::new(RefCell::new(res))))
            },
        );

        self.add_native_func_with_optional_args(
            "concatenate",
            &[],
            &[],
            Some("lists"),
            |_, args| -> EvalResult {
                let arg0 = args.get(&"lists".to_owned()).unwrap();
                let arr = arg0.expect_array("arguments `lists`")?;

                let mut lists: Vec<Vec<Value>> = vec![];
                for (i, v) in arr.iter().enumerate() {
                    let childlist = v.expect_array(format!("argument[{}]", (i + 1)).as_str())?;
                    lists.push(childlist.iter().map(|v| v.clone()).collect());
                }
                let res = lists.concat();
                Ok(Value::ArrayV(Rc::new(RefCell::new(res))))
            },
        );

        self.add_native_func("flatten", &["list"], |_, args| -> EvalResult {
            let arg0 = args.get(&"list".to_owned()).unwrap();
            let arr = arg0.expect_array("argument[1] `list`")?;

            let mut res: Vec<Value> = vec![];
            for v in arr.iter() {
                match v {
                    Value::ArrayV(a) => {
                        for x in a.as_ref().borrow().iter() {
                            res.push(x.clone());
                        }
                    }
                    x => res.push(x.clone()),
                }
            }
            Ok(Value::ArrayV(Rc::new(RefCell::new(res))))
        });

        self.add_native_func("sort", &["list"], |_, args| -> EvalResult {
            let arg0 = args.get(&"list".to_owned()).unwrap();
            let arr = arg0.expect_array("argument[1] `list`")?;

            let mut res: Vec<Value> = arr.iter().map(|x| x.clone()).collect();
            res.sort();
            Ok(Value::ArrayV(Rc::new(RefCell::new(res))))
        });

        self.add_native_func(
            "insert before",
            &["list", "position", "newItem"],
            |_, args| -> EvalResult {
                let arg0 = args.get(&"list".to_owned()).unwrap();
                let arr = arg0.expect_array("argument[1] `list`")?;

                let arg1 = args.get(&"position".to_owned()).unwrap();
                let feel_position =
                    range_check(arg1.expect_usize("argument[2] `position`")?, 1, arr.len())?;

                let position = from_feel_index(feel_position);

                let new_item = args.get(&"newItem".to_owned()).unwrap();

                let pre = arr.borrow()[..position].to_owned();
                let post = arr.borrow()[position..].to_owned();
                let res = vec![pre, vec![new_item.clone()], post].concat();
                Ok(Value::ArrayV(Rc::new(RefCell::new(res))))
            },
        );

        self.add_native_func("remove", &["list", "position"], |_, args| -> EvalResult {
            let arg0 = args.get(&"list".to_owned()).unwrap();
            let arr = arg0.expect_array("argument[1] `list`")?;

            let arg1 = args.get(&"position".to_owned()).unwrap();
            let feel_position =
                range_check(arg1.expect_usize("argument[2] `position`")?, 1, arr.len())?;

            let position = from_feel_index(feel_position);

            let pre = arr.borrow()[..position].to_owned();
            let post = arr.borrow()[(position + 1)..].to_owned();
            let res = vec![pre, post].concat();
            Ok(Value::ArrayV(Rc::new(RefCell::new(res))))
        });

        self.add_native_func("reverse", &["list"], |_, args| -> EvalResult {
            let arg0 = args.get(&"list".to_owned()).unwrap();
            let arr = arg0.expect_array("argument[1] `list`")?;

            let res = arr.iter().rev().map(|v| v.clone()).collect();
            Ok(Value::ArrayV(Rc::new(RefCell::new(res))))
        });

        self.add_native_func("index of", &["list", "match"], |_, args| -> EvalResult {
            let arg0 = args.get(&"list".to_owned()).unwrap();
            let arr = arg0.expect_array("argument[1] `list`")?;

            let arg1 = args.get(&"match".to_owned()).unwrap();

            let mut res: Vec<Value> = vec![];

            for (i, v) in arr.iter().enumerate() {
                if *v == *arg1 {
                    //return Ok(Value::from_usize(to_feel_index(i)))
                    res.push(Value::from_usize(to_feel_index(i)))
                }
            }
            Ok(Value::ArrayV(Rc::new(RefCell::new(res))))
        });

        self.add_native_func("distinct values", &["list"], |_, args| -> EvalResult {
            let arg0 = args.get(&"list".to_owned()).unwrap();
            let arr = arg0.expect_array("argument[1] `list`")?;
            let mut res: Vec<Value> = arr.iter().map(|x| x.clone()).collect();
            res.dedup();
            Ok(Value::ArrayV(Rc::new(RefCell::new(res))))
        });

        self.add_native_func_with_optional_args(
            "union",
            &[],
            &[],
            Some("lists"),
            |_, args| -> EvalResult {
                let arg0 = args.get(&"lists".to_owned()).unwrap();
                let arr = arg0.expect_array("arguments `lists`")?;
                let mut lists: Vec<Vec<Value>> = vec![];
                for (i, v) in arr.iter().enumerate() {
                    let childlist = v.expect_array(format!("argument[{}]", (i + 1)).as_str())?;
                    lists.push(childlist.iter().map(|v| v.clone()).collect());
                }
                let mut res = lists.concat();
                res.dedup();
                Ok(Value::ArrayV(Rc::new(RefCell::new(res))))
            },
        );

        // context/map functions
        self.add_native_func("get value", &["context", "key"], |_, args| -> EvalResult {
            let arg0 = args.get(&"context".to_owned()).unwrap();
            let m = arg0.expect_context("argument[1] `context`")?;

            let arg1 = args.get(&"key".to_owned()).unwrap();
            let path = match arg1.clone() {
                Value::StrV(s) => vec![s],
                Value::ArrayV(a) => {
                    let mut keys = vec![];
                    for (i, v) in a.as_ref().borrow().iter().enumerate() {
                        let s = v.expect_string(format!("argument[2][{}]", (i + 1)).as_str())?;
                        keys.push(s);
                    }
                    keys
                }
                _ => {
                    return Err(EvalError::TypeError(format!(
                        "expect string or string list, by {} found",
                        arg1.data_type()
                    )))
                }
            };

            if let Some(v) = m.get_path(path.as_slice()) {
                Ok(v.clone())
            } else {
                Ok(Value::NullV)
            }
        });
        self.add_native_func("get entries", &["context"], |_, args| -> EvalResult {
            let arg0 = args.get(&"context".to_owned()).unwrap();
            let m = arg0.expect_context("argument[1] `context`")?;
            let mut res = vec![];
            for (k, v) in m.entries() {
                let mut ent_ctx = Context::new();
                ent_ctx.insert("key".to_string(), Value::StrV(k));
                ent_ctx.insert("value".to_string(), v);
                res.push(Value::ContextV(Rc::new(RefCell::new(ent_ctx))));
            }
            Ok(Value::ArrayV(Rc::new(RefCell::new(res))))
        });

        self.add_native_func(
            "context put",
            &["context", "key", "value"],
            |_, args| -> EvalResult {
                let arg0 = args.get(&"context".to_owned()).unwrap();
                let m = arg0.expect_context_ref("argument[1] `context`")?;

                let arg1 = args.get(&"key".to_owned()).unwrap();
                let path = match arg1.clone() {
                    Value::StrV(s) => vec![s],
                    Value::ArrayV(a) => {
                        let mut keys = vec![];
                        for (i, v) in a.as_ref().borrow().iter().enumerate() {
                            let s =
                                v.expect_string(format!("argument[2][{}]", (i + 1)).as_str())?;
                            keys.push(s);
                        }
                        keys
                    }
                    _ => {
                        return Err(EvalError::TypeError(format!(
                            "expect string or string list, by {} found",
                            arg1.data_type()
                        )))
                    }
                };

                let arg2 = args.get(&"value".to_owned()).unwrap();
                m.as_ref()
                    .borrow_mut()
                    .insert_path(path.as_slice(), arg2.clone());

                Ok(Value::ContextV(m.clone()))
            },
        ); // end `context put`

        self.add_native_func("context merge", &["contexts"], |_, args| -> EvalResult {
            let arg0 = args.get(&"contexts".to_owned()).unwrap();
            let contexts = arg0.expect_array("argument[1] `contexts`")?;
            let mut res_ctx = Context::new();
            for (i, ctx_v) in contexts.iter().enumerate() {
                let ctx = ctx_v.expect_context(format!("argument[1][{}]", i + 1).as_str())?;
                res_ctx.merge(&ctx);
            }
            Ok(Value::ContextV(Rc::new(RefCell::new(res_ctx))))
        }); // end `context merge`
    }
}

lazy_static! {
    pub static ref PRELUDE: Prelude = {
        let mut p = Prelude::new();
        p.load_preludes();
        p
    };
}
