use crate::eval::{EvalError, EvalResult};
use crate::values::func::{MacroCb, MacroCbT, NativeFunc, NativeFuncT};
use crate::values::value::Value::{self, *};
use lazy_static::lazy_static;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::collections::HashMap;

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
    pub fn add_macro(&mut self, name: &str, require_args: &[&str], cb: MacroCb) {
        let require_args_vec = require_args.into_iter().map(|s| String::from(*s)).collect();
        let macro_t = MacroCbT(cb);
        let macro_value = MacroV {
            callback: macro_t,
            require_args: require_args_vec,
        };
        self.set_var(name.to_owned(), macro_value);
    }

    pub fn add_native_func(&mut self, name: &str, require_args: &[&str], func: NativeFunc) {
        let require_arg_vec = require_args.into_iter().map(|&s| String::from(s)).collect();
        let func_t = NativeFuncT(func);
        let func_value = NativeFuncV {
            func: func_t,
            require_args: require_arg_vec,
        };
        self.set_var(name.to_owned(), func_value);
    }

    pub fn load_preludes(&mut self) {
        self.add_native_func(
            "set",
            &["name", "value"],
            |intp, args| -> Result<Value, EvalError> {
                let name_node = args.get(&"name".to_owned()).unwrap();
                let var_name = match name_node {
                    StrV(value) => value.clone(),
                    _ => return Err(EvalError::runtime("argument name should be string")),
                };
                let value = args.get(&"value".to_owned()).unwrap();
                intp.set_var(var_name, value.clone());
                Ok(value.clone())
            },
        );

        self.add_native_func(
            "bind",
            &["name", "value"],
            |intp, args| -> Result<Value, EvalError> {
                let name_node = args.get(&"name".to_owned()).unwrap();
                let var_name = match name_node {
                    StrV(value) => value.clone(),
                    _ => return Err(EvalError::runtime("argument name should be string")),
                };
                let value = args.get(&"value".to_owned()).unwrap();
                intp.bind_var(var_name, value.clone());
                Ok(value.clone())
            },
        );

        self.add_macro(
            "is defined",
            &["value"],
            |intp, nodes| -> Result<Value, EvalError> {
                let value_node = nodes.get(&"value".to_owned()).unwrap();
                intp.is_defined(value_node)
            },
        );

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

        self.add_native_func("string length", &["string"], |_, args| -> EvalResult {
            let v = args.get(&"string".to_owned()).unwrap();
            if let Value::StrV(s) = v {
                let lenn = Decimal::from_usize(s.len()).unwrap();
                Ok(Value::NumberV(lenn))
            } else {
                Err(EvalError::TypeError("string".to_owned()))
            }
        });
    }
}

lazy_static! {
    pub static ref PRELUDE: Prelude = {
        let mut p = Prelude::new();
        p.load_preludes();
        p
    };
}
