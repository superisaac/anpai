use super::value::Value;
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
