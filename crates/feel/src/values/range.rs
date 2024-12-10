use super::value::Value;
use crate::eval::EvalResult;
use crate::prelude::Prelude;
use std::fmt;
use std::rc::Rc;

// range
#[derive(Clone, PartialEq, Eq)]
pub struct RangeT {
    pub start_open: bool,
    pub start: Rc<Value>,
    pub end_open: bool,
    pub end: Rc<Value>,
}

impl fmt::Debug for RangeT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let start_sym = if self.start_open { "(" } else { "[" };
        let end_sym = if self.end_open { ")" } else { "]" };
        write!(f, "{}{}..{}{}", start_sym, self.start, self.end, end_sym)
    }
}

impl fmt::Display for RangeT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let start_sym = if self.start_open { "(" } else { "[" };
        let end_sym = if self.end_open { ")" } else { "]" };
        write!(f, "{}{}..{}{}", start_sym, self.start, self.end, end_sym)
    }
}

impl RangeT {
    fn compare(a: &Value, b: &Value) -> i32 {
        if *a < *b {
            -1
        } else if *a == *b {
            0
        } else {
            1
        }
    }

    pub fn position(&self, p: &Value) -> i32 {
        let cmp_start = Self::compare(&p, &self.start);
        if self.start_open {
            if cmp_start <= 0 {
                return -1;
            }
        } else {
            if cmp_start <= 0 {
                return cmp_start;
            }
        }

        let cmp_end = Self::compare(&p, &self.end);
        if self.end_open && cmp_end >= 0 {
            1
        } else if !self.end_open && cmp_end > 0 {
            1
        } else {
            0
        }
    }
    pub fn contains(&self, n: &Value) -> bool {
        self.position(n) == 0
    }

    pub fn before_point(&self, p: &Value) -> bool {
        self.position(p) > 0
    }

    pub fn after_point(&self, p: &Value) -> bool {
        self.position(p) < 0
    }

    pub fn before(&self, other: &RangeT) -> bool {
        //let r = self.end.as_ref().cmp(other.start.as_ref());
        let r = Self::compare(self.end.as_ref(), other.start.as_ref());
        if !self.end_open && !other.start_open {
            // two ranges meet
            r < 0
        } else {
            r <= 0
        }
    }

    pub fn after(&self, other: &RangeT) -> bool {
        let r = Self::compare(self.start.as_ref(), other.end.as_ref());
        if !self.start_open && !other.end_open {
            // two ranges meet
            r > 0
        } else {
            r >= 0
        }
    }

    pub fn includes(&self, other: &RangeT) -> bool {
        let cmp_start = Self::compare(self.start.as_ref(), other.start.as_ref());
        let cmp_end = Self::compare(self.end.as_ref(), other.end.as_ref());

        if cmp_start > 0 || cmp_end < 0 {
            return false;
        }

        if !(cmp_start < 0 || !self.start_open || other.start_open) {
            return false;
        }

        if !(cmp_end > 0 || !self.end_open || other.end_open) {
            return false;
        }
        return true;
    }

    pub fn overlaps_before(&self, other: &RangeT) -> bool {
        let pos = other.position(self.end.as_ref());
        if pos != 0 {
            return false;
        }

        if self.end_open && Self::compare(self.end.as_ref(), other.start.as_ref()) == 0 {
            return false;
        }
        true
    }

    pub fn overlaps_after(&self, other: &RangeT) -> bool {
        let pos = other.position(self.start.as_ref());
        if pos != 0 {
            return false;
        }
        if self.end_open && Self::compare(self.start.as_ref(), other.end.as_ref()) == 0 {
            return false;
        }
        true
    }

    pub fn meets(&self, other: &RangeT) -> bool {
        let r = Self::compare(self.end.as_ref(), other.start.as_ref());
        r == 0 && !self.end_open && !other.start_open
    }

    pub fn finished_by(&self, v: &Value) -> bool {
        if Self::compare(self.end.as_ref(), v) != 0 {
            false
        } else if self.end_open {
            false
        } else {
            true
        }
    }

    pub fn finished_by_range(&self, other: &RangeT) -> bool {
        if Self::compare(self.end.as_ref(), other.end.as_ref()) != 0 {
            false
        } else if self.end_open != other.end_open {
            false
        } else if !self.includes(other) {
            false
        } else {
            true
        }
    }

    pub fn started_by(&self, v: &Value) -> bool {
        if Self::compare(self.start.as_ref(), v) != 0 {
            false
        } else if self.start_open {
            false
        } else {
            true
        }
    }

    pub fn started_by_range(&self, other: &RangeT) -> bool {
        if Self::compare(self.start.as_ref(), other.start.as_ref()) != 0 {
            false
        } else if self.start_open != other.start_open {
            false
        } else if !self.includes(other) {
            false
        } else {
            true
        }
    }
}

pub(crate) fn install_range_prelude(prelude: &mut Prelude) {
    // refer to https://docs.camunda.io/docs/components/modeler/feel/builtin-functions/feel-built-in-functions-range/

    prelude.add_native_func("before", &["a", "b"], |_, args| -> EvalResult {
        let arg0 = args.get(&"a".to_owned()).unwrap();
        let arg1 = args.get(&"b".to_owned()).unwrap();
        match arg0 {
            Value::RangeV(rng_a) => match arg1 {
                Value::RangeV(rng_b) => Ok(Value::BoolV(rng_a.before(rng_b))),
                b => Ok(Value::BoolV(rng_a.before_point(b))),
            },
            a => match arg1 {
                Value::RangeV(rng_b) => Ok(Value::BoolV(rng_b.after_point(a))),
                b => Ok(Value::BoolV(a < b)),
            },
        }
    });

    prelude.add_native_func("after", &["a", "b"], |_, args| -> EvalResult {
        let arg0 = args.get(&"a".to_owned()).unwrap();
        let arg1 = args.get(&"b".to_owned()).unwrap();
        match arg0 {
            Value::RangeV(rng_a) => match arg1 {
                Value::RangeV(rng_b) => Ok(Value::BoolV(rng_a.after(rng_b))),
                b => Ok(Value::BoolV(rng_a.after_point(b))),
            },
            a => match arg1 {
                Value::RangeV(rng_b) => Ok(Value::BoolV(rng_b.before_point(a))),
                b => Ok(Value::BoolV(a > b)),
            },
        }
    });

    prelude.add_native_func("meets", &["a", "b"], |_, args| -> EvalResult {
        let arg0 = args.get(&"a".to_owned()).unwrap();
        let arg1 = args.get(&"b".to_owned()).unwrap();
        let rng0 = arg0.expect_range("argument[1] `a`")?;
        let rng1 = arg1.expect_range("argument[2] `b`")?;
        Ok(Value::BoolV(rng0.meets(rng1)))
    });

    prelude.add_native_func("met by", &["a", "b"], |_, args| -> EvalResult {
        let arg0 = args.get(&"a".to_owned()).unwrap();
        let arg1 = args.get(&"b".to_owned()).unwrap();
        let rng0 = arg0.expect_range("argument[1] `a`")?;
        let rng1 = arg1.expect_range("argument[2] `b`")?;
        Ok(Value::BoolV(rng1.meets(rng0)))
    });

    prelude.add_native_func("overlaps", &["a", "b"], |_, args| -> EvalResult {
        let arg0 = args.get(&"a".to_owned()).unwrap();
        let arg1 = args.get(&"b".to_owned()).unwrap();
        let rng0 = arg0.expect_range("argument[1] `a`")?;
        let rng1 = arg1.expect_range("argument[2] `b`")?;
        Ok(Value::BoolV(
            rng0.overlaps_before(rng1) || rng0.overlaps_after(rng1),
        ))
    });

    prelude.add_native_func("overlaps before", &["a", "b"], |_, args| -> EvalResult {
        let arg0 = args.get(&"a".to_owned()).unwrap();
        let arg1 = args.get(&"b".to_owned()).unwrap();
        let rng0 = arg0.expect_range("argument[1] `a`")?;
        let rng1 = arg1.expect_range("argument[2] `b`")?;
        Ok(Value::BoolV(rng0.overlaps_before(rng1)))
    });

    prelude.add_native_func("overlaps after", &["a", "b"], |_, args| -> EvalResult {
        let arg0 = args.get(&"a".to_owned()).unwrap();
        let arg1 = args.get(&"b".to_owned()).unwrap();
        let rng0 = arg0.expect_range("argument[1] `a`")?;
        let rng1 = arg1.expect_range("argument[2] `b`")?;
        Ok(Value::BoolV(rng0.overlaps_after(rng1)))
    });

    prelude.add_native_func("starts", &["a", "b"], |_, args| -> EvalResult {
        let arg0 = args.get(&"a".to_owned()).unwrap();
        let arg1 = args.get(&"b".to_owned()).unwrap();
        let rng1 = arg1.expect_range("argument[2] `b`")?;
        match arg0 {
            Value::RangeV(rng0) => Ok(Value::BoolV(rng1.started_by_range(rng0))),
            x => Ok(Value::BoolV(rng1.started_by(x))),
        }
    });

    prelude.add_native_func("started by", &["a", "b"], |_, args| -> EvalResult {
        let arg0 = args.get(&"a".to_owned()).unwrap();
        let arg1 = args.get(&"b".to_owned()).unwrap();
        let rng0 = arg0.expect_range("argument[1] `a`")?;
        match arg1 {
            Value::RangeV(rng1) => Ok(Value::BoolV(rng0.started_by_range(rng1))),
            x => Ok(Value::BoolV(rng0.started_by(x))),
        }
    });

    prelude.add_native_func("finishes", &["a", "b"], |_, args| -> EvalResult {
        let arg0 = args.get(&"a".to_owned()).unwrap();
        let arg1 = args.get(&"b".to_owned()).unwrap();
        let rng1 = arg1.expect_range("argument[2] `b`")?;
        match arg0 {
            Value::RangeV(rng0) => Ok(Value::BoolV(rng1.finished_by_range(rng0))),
            x => Ok(Value::BoolV(rng1.finished_by(x))),
        }
    });

    prelude.add_native_func("finished by", &["a", "b"], |_, args| -> EvalResult {
        let arg0 = args.get(&"a".to_owned()).unwrap();
        let arg1 = args.get(&"b".to_owned()).unwrap();
        let rng0 = arg0.expect_range("argument[1] `a`")?;
        match arg1 {
            Value::RangeV(rng1) => Ok(Value::BoolV(rng0.finished_by_range(rng1))),
            x => Ok(Value::BoolV(rng0.finished_by(x))),
        }
    });

    prelude.add_native_func("includes", &["a", "b"], |_, args| -> EvalResult {
        let arg0 = args.get(&"a".to_owned()).unwrap();
        let arg1 = args.get(&"b".to_owned()).unwrap();
        let rng0 = arg0.expect_range("argument[1] `a`")?;
        match arg1 {
            Value::RangeV(rng1) => Ok(Value::BoolV(rng0.includes(rng1))),
            x => Ok(Value::BoolV(rng0.position(x) == 0)),
        }
    });

    prelude.add_native_func("during", &["a", "b"], |_, args| -> EvalResult {
        let arg0 = args.get(&"a".to_owned()).unwrap();
        let arg1 = args.get(&"b".to_owned()).unwrap();
        let rng1 = arg1.expect_range("argument[1] `a`")?;
        match arg0 {
            Value::RangeV(rng0) => Ok(Value::BoolV(rng1.includes(rng0))),
            x => Ok(Value::BoolV(rng1.position(x) == 0)),
        }
    });

    prelude.add_native_func("coincides", &["a", "b"], |_, args| -> EvalResult {
        let arg0 = args.get(&"a".to_owned()).unwrap();
        let arg1 = args.get(&"b".to_owned()).unwrap();
        let rng0 = arg0.expect_range("argument[1] `a`")?;
        let rng1 = arg1.expect_range("argument[2] `b`")?;
        Ok(Value::BoolV(*rng0 == *rng1))
    });
}
