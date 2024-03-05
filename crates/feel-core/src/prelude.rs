use crate::eval::EvalError;
use crate::values::func::{MacroCb, MacroCbT, NativeFunc, NativeFuncT};
use crate::values::value::Value::{self, *};
use lazy_static::lazy_static;
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
            optional_args: Vec::new(),
        };
        self.set_var(name.to_owned(), func_value);
    }

    pub fn add_native_func_with_optional_args(
        &mut self,
        name: &str,
        require_args: &[&str],
        optional_args: &[&str],
        func: NativeFunc,
    ) {
        let func_t = NativeFuncT(func);
        let func_value = NativeFuncV {
            func: func_t,
            require_args: require_args.into_iter().map(|&s| String::from(s)).collect(),
            optional_args: optional_args
                .into_iter()
                .map(|&s| String::from(s))
                .collect(),
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

        crate::values::value::add_preludes(self);
    }
}

lazy_static! {
    pub static ref PRELUDE: Prelude = {
        let mut p = Prelude::new();
        p.load_preludes();
        p
    };
}
