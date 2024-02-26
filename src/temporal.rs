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

fn mod_div(v: u32, max_value: u32) -> (u32, u32) {
    (v % max_value, v / max_value)
}

pub fn datetime_add(
    dt: iso8601::DateTime,
    dur: iso8601::Duration,
) -> Result<iso8601::DateTime, String> {
    if let iso8601::Date::YMD {
        year: dt_year,
        month: dt_month,
        day: dt_day,
    } = dt.date
    {
        if let iso8601::Duration::YMDHMS {
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
        } = dur
        {
            let (r_millisecond, rem) = mod_div(dt.time.millisecond + millisecond, 1000);
            let (r_second, rem) = mod_div(dt.time.second + second + rem, 60);
            let (r_minute, rem) = mod_div(dt.time.minute + minute + rem, 60);
            let (r_hour, rem) = mod_div(dt.time.hour + hour + rem, 24);
            let (r_day, rem) = mod_div(dt_day + day + rem, 30);
            let (r_month, rem) = mod_div(dt_month + month + rem, 12);
            let r_year = (dt_year as u32) + year + rem;
            return Ok(iso8601::DateTime {
                time: iso8601::Time {
                    millisecond: r_millisecond,
                    second: r_second,
                    minute: r_minute,
                    hour: r_hour,
                    tz_offset_hours: dt.time.tz_offset_hours,
                    tz_offset_minutes: dt.time.tz_offset_minutes,
                },
                date: iso8601::Date::YMD {
                    year: r_year as i32,
                    month: r_month,
                    day: r_day,
                },
            });
        }
    }
    Err("fail to add temporal value".to_owned())
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
