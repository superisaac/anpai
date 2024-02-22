use core::slice::Iter;
use std::collections::BTreeMap;
use std::fmt;

pub fn fmt_vec<T: fmt::Display>(
    f: &mut fmt::Formatter,
    vec_iter: Iter<T>,
    prefix: &str,
    suffix: &str,
) -> fmt::Result {
    write!(f, "{}", prefix)?;
    for (i, v) in vec_iter.enumerate() {
        if i > 0 {
            write!(f, ", {}", v)?;
        } else {
            write!(f, "{}", v)?;
        }
    }
    write!(f, "{}", suffix)
}

pub fn fmt_map<T: fmt::Display>(
    f: &mut fmt::Formatter,
    map: &BTreeMap<String, T>,
    prefix: &str,
    suffix: &str,
) -> fmt::Result {
    write!(f, "{}", prefix)?;
    for (i, (k, v)) in map.iter().enumerate() {
        if i > 0 {
            write!(f, ", {}:{}", k, v)?;
        } else {
            write!(f, "{}:{}", k, v)?;
        }
    }
    write!(f, "{}", suffix)
}
