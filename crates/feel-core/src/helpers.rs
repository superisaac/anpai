use core::slice::Iter;
use std::cmp;
use std::collections::BTreeMap;
use std::fmt;

#[inline(always)]
pub fn compare_value<T>(a: T, b: T) -> cmp::Ordering
where
    T: cmp::PartialOrd,
{
    if a < b {
        cmp::Ordering::Less
    } else if a == b {
        cmp::Ordering::Equal
    } else {
        cmp::Ordering::Greater
    }
}

pub fn fmt_vec<T: fmt::Display>(
    f: &mut fmt::Formatter,
    vec_iter: Iter<T>,
    prefix: &str,
    suffix: &str,
) -> fmt::Result {
    fmt_iter(f, vec_iter, ", ", prefix, suffix)
}

pub fn fmt_iter<T: fmt::Display>(
    f: &mut fmt::Formatter,
    vec_iter: Iter<T>,
    delim: &str,
    prefix: &str,
    suffix: &str,
) -> fmt::Result {
    write!(f, "{}", prefix)?;
    for (i, v) in vec_iter.enumerate() {
        if i > 0 {
            write!(f, "{}{}", delim, v)?;
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
