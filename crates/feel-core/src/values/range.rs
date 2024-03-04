use crate::values::value::Value;
use std::fmt;
use std::rc::Rc;

// range
#[derive(Clone, PartialEq)]
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

    pub fn position(&self, p: Value) -> i32 {
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
    pub fn contains(&self, n: Value) -> bool {
        self.position(n) == 0
    }
}
