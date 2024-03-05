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

/// format a vector of displayable
pub fn fmt_vec<T: fmt::Display>(
    f: &mut fmt::Formatter,
    vec_iter: Iter<T>,
    prefix: &str,
    suffix: &str,
) -> fmt::Result {
    fmt_iter(f, vec_iter, ", ", prefix, suffix)
}

/// format over iterators
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

/// format over a map
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

/// restore an escaped string
pub fn unescape(input: &str) -> String {
    let mut escaping = false;
    let mut res = String::from("");
    for c in input.chars() {
        if escaping {
            let mc = match c {
                't' => '\t',
                'r' => '\r',
                'n' => '\n',
                kc => kc,
            };
            res.push(mc);
            escaping = false;
        } else if c == '\\' {
            escaping = true;
        } else {
            res.push(c);
        }
    }
    res
}

/// escape special characters in a string
pub fn escape(input: &str) -> String {
    let mut res = String::from("");
    for c in input.chars() {
        match c {
            '\t' => {
                res.push_str("\\t");
            }
            '\r' => {
                res.push_str("\\r");
            }
            '\n' => {
                res.push_str("\\n");
            }
            '"' => res.push_str("\\\""),
            xc => res.push(xc),
        }
    }
    res
}

#[test]
fn test_string_escape_unescape() {
    let input = "abc\tdef\r\nte\"ck";
    let escaped = escape(input);
    assert_eq!(escaped, "abc\\tdef\\r\\nte\\\"ck");
    let unescaped = unescape(escaped.as_str());
    assert_eq!(unescaped.as_str(), input);
}
