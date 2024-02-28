use crate::eval::EvalError;
use crate::value::{
    MacroCb, MacroCbT, NativeFunc, NativeFuncT,
    Value::{self, *},
};
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
    pub fn add_macro(&mut self, name: &str, arg_names: &[&str], cb: MacroCb) {
        let arg_names_vec = arg_names.into_iter().map(|s| String::from(*s)).collect();
        let macro_t = MacroCbT(cb);
        let macro_value = MacroV {
            callback: macro_t,
            arg_names: arg_names_vec,
        };
        self.set_var(name.to_owned(), macro_value);
    }

    pub fn add_native_func(&mut self, name: &str, arg_names: &[&str], func: NativeFunc) {
        let arg_names_vec = arg_names.into_iter().map(|s| String::from(*s)).collect();
        let func_t = NativeFuncT(func);
        let func_value = NativeFuncV {
            func: func_t,
            arg_names: arg_names_vec,
        };
        self.set_var(name.to_owned(), func_value);
    }

    pub fn load_preludes(&mut self) {
        self.add_native_func(
            "set",
            &["name", "value"],
            |intp, args| -> Result<Value, EvalError> {
                let name_node = args
                    .get(&"name".to_owned())
                    .ok_or(EvalError::runtime("no name"))?;
                let var_name = match name_node {
                    StrV(value) => value.clone(),
                    _ => return Err(EvalError::runtime("argument name should be string")),
                };
                let value = args
                    .get(&"value".to_owned())
                    .ok_or(EvalError::runtime("no value"))?;
                intp.set_var(var_name, value.clone());
                Ok(value.clone())
            },
        );

        self.add_macro(
            "is defined",
            &["value"],
            |intp, nodes| -> Result<Value, EvalError> {
                let value_node = nodes
                    .get(&"value".to_owned())
                    .ok_or(EvalError::runtime("no value"))?;
                intp.is_defined(value_node.clone())
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
