use super::value::Value;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Context(pub BTreeMap<String, Value>);

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let prefix = "{";
        let suffix = "}";
        write!(f, "{}", prefix)?;
        for (i, (k, v)) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, r#", "{}":{}"#, k, v)?;
            } else {
                write!(f, r#""{}":{}"#, k, v)?;
            }
        }
        write!(f, "{}", suffix)
    }
}

impl Context {
    pub fn new() -> Context {
        Context(BTreeMap::new())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, key: String) -> Option<Value> {
        self.0.get(&key).map(|v| v.clone())
    }

    pub fn get_mut(&mut self, key: String) -> Option<&mut Value> {
        self.0.get_mut(&key)
    }

    pub fn entries(&self) -> Vec<(String, Value)> {
        self.0.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        // let res: Vec<(String, Value)> = self.0.iter().map(|(k, v)| (k.clone(), v.clone()) ).collect();
        // res
    }

    pub fn get_path(&self, path: &[String]) -> Option<Value> {
        match path.len() {
            0 => None,
            1 => self.get(path[0].clone()),
            _ => {
                if let Some(Value::ContextV(ctx)) = self.get(path[0].clone()) {
                    let rest = &path[1..];
                    ctx.borrow().get_path(rest)
                } else {
                    None
                }
            }
        }
    }

    pub fn insert(&mut self, key: String, value: Value) -> Option<Value> {
        self.0.insert(key, value)
    }

    pub fn insert_path(&mut self, path: &[String], value: Value) -> Option<Value> {
        match path.len() {
            0 => None,
            1 => self.insert(path[0].clone(), value),
            _ => {
                let first_key = path[0].clone();
                match self.get_mut(first_key.clone()) {
                    Some(Value::ContextV(ctx)) => {
                        let rest = &path[1..];
                        let mut r = ctx.borrow_mut();

                        //Rc::get_mut(r)
                        r.insert_path(rest, value)
                    }
                    None => {
                        let mut childmap = Context::new();
                        let rest = &path[1..];
                        //Rc::get_mut(r)
                        childmap.insert_path(rest, value);
                        self.0
                            .insert(first_key, Value::ContextV(Rc::new(RefCell::new(childmap))))
                    }
                    _ => None,
                }
            }
        }
    }

    pub fn merge(&mut self, other: &Context) {
        for (k, v) in other.0.iter() {
            self.0.insert(k.clone(), v.clone());
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::value::Value;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    pub fn test_context_cell() {
        let cell = Rc::new(RefCell::new(super::Context::new()));
        cell.borrow_mut()
            .insert("a".to_owned(), Value::StrV("hello".to_owned()));

        assert_eq!(cell.borrow().len(), 1);

        let cell1 = cell.clone();

        cell1
            .borrow_mut()
            .insert("b".to_owned(), Value::StrV("world".to_owned()));

        assert_eq!(cell.borrow().len(), 2);
    }
}

pub type ContextRef = Rc<RefCell<Context>>;
