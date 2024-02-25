use crate::value::Value;
extern crate iso8601;

pub fn parse_temporal(temp_str: &str) -> Result<Value, String> {
    if temp_str.starts_with("@") {
        let striped = &temp_str[2..temp_str.len() - 1];
        return parse_temporal(striped);
    }

    if let Ok(dt) = iso8601::datetime(temp_str) {
        Ok(Value::DateTimeV(dt))
    } else if let Ok(date) = iso8601::date(temp_str) {
        Ok(Value::DateV(date))
    } else if let Ok(time) = iso8601::time(temp_str) {
        Ok(Value::TimeV(time))
    } else if let Ok(dur) = iso8601::duration(temp_str) {
        Ok(Value::DurationV(dur))
    } else {
        Err("fail to parse temporal value".to_owned())
    }
}

#[cfg(test)]
mod test {
    use super::parse_temporal;
    use crate::value::Value;
    use core::assert_matches::assert_matches;

    #[test]
    fn test_parse_temp_value() {
        assert_matches!(
            parse_temporal(r#"@"2020-04-06T08:00:00@Europe/Berlin""#),
            Ok(Value::DateTimeV(_))
        );
        assert_matches!(
            parse_temporal("2020-04-06T08:00:00@Europe/Berlin"),
            Ok(Value::DateTimeV(_))
        );
        assert_matches!(parse_temporal("PT2H3M"), Ok(Value::DurationV(_)));
    }
}
