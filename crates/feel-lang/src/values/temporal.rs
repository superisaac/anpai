use super::value::Value;
use super::value::ValueError;
use chrono::Datelike;
use lazy_static::lazy_static;
use regex::Regex;

use std::cmp;
use std::str::FromStr;
extern crate iso8601;
use crate::helpers::compare_value;

pub(crate) type DateTimeT = chrono::DateTime<chrono::FixedOffset>;

lazy_static! {
    static ref WEEK_NAMES: Vec<String> = {
        let week_strs = vec![
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
            "Sunday",
        ];
        let week_names: Vec<String> = week_strs.into_iter().map(|s| s.to_owned()).collect();
        week_names
    };
    static ref TZ_PATTERN: Regex = Regex::new(r"\+.*$").unwrap();
}

pub fn parse_temporal(temp_str: &str) -> Result<Value, ValueError> {
    if temp_str.starts_with("@") {
        let striped = &temp_str[2..temp_str.len() - 1];
        return parse_temporal(striped);
    }

    if temp_str.starts_with("-") {
        if let Ok(Value::DurationV {
            duration,
            negative: _,
        }) = parse_duration(&temp_str[1..])
        {
            Ok(Value::DurationV {
                duration,
                negative: true,
            })
        } else {
            Err(ValueError(
                "fail to parse negative temporal value".to_owned(),
            ))
        }
    } else {
        parse_datetime(temp_str)
            .or_else(|_| parse_date(temp_str))
            .or_else(|_| parse_time(temp_str))
            .or_else(|_| parse_duration(temp_str))
    }
}

pub(crate) fn parse_datetime(s: &str) -> Result<Value, ValueError> {
    if let Ok(dt) = iso8601::datetime(s) {
        let has_timezone = TZ_PATTERN.find(s).is_some();

        let mut cdt = match chrono::DateTime::try_from(dt) {
            Ok(v) => v,
            Err(err) => return Err(ValueError(format!("{:?}", err))),
        };
        if !has_timezone {
            // use local time zone if timezone is not specified
            let local_now = chrono::Local::now();
            cdt = chrono::DateTime::<chrono::FixedOffset>::from_naive_utc_and_offset(
                cdt.naive_utc(),
                local_now.offset().clone(),
            );
        }
        Ok(Value::DateTimeV(cdt))
    } else {
        Err(ValueError("fail to parse date".to_owned()))
    }
}

pub(crate) fn parse_date(s: &str) -> Result<Value, ValueError> {
    if let Ok(date) = iso8601::date(s) {
        Ok(Value::DateV(date))
    } else {
        Err(ValueError("fail to parse date".to_owned()))
    }
}

pub(crate) fn parse_time(s: &str) -> Result<Value, ValueError> {
    if let Ok(time) = iso8601::time(s) {
        Ok(Value::TimeV(time))
    } else {
        Err(ValueError("fail to parse time".to_owned()))
    }
}

pub(crate) fn parse_duration(s: &str) -> Result<Value, ValueError> {
    if let Ok(dur) = iso8601::duration(s) {
        Ok(Value::DurationV {
            duration: dur,
            negative: false,
        })
    } else {
        Err(ValueError("fail to parse duration".to_owned()))
    }
}

pub(crate) fn now() -> DateTimeT {
    chrono::Local::now().into()
}

pub(crate) fn today() -> iso8601::Date {
    let dn = chrono::Local::now().date_naive();
    iso8601::Date::YMD {
        year: dn.year(),
        month: dn.month(),
        day: dn.day(),
    }
}

pub fn compare_date(a: &iso8601::Date, b: &iso8601::Date) -> Option<cmp::Ordering> {
    match *a {
        iso8601::Date::YMD { year, month, day } => match *b {
            iso8601::Date::YMD {
                year: b_year,
                month: b_month,
                day: b_day,
            } => Some(compare_value((year, month, day), (b_year, b_month, b_day))),
            _ => None,
        },
        iso8601::Date::Week { year, ww, d } => match *b {
            iso8601::Date::Week {
                year: b_year,
                ww: b_ww,
                d: b_d,
            } => Some(compare_value((year, ww, d), (b_year, b_ww, b_d))),
            _ => None,
        },
        iso8601::Date::Ordinal { year, ddd } => match *b {
            iso8601::Date::Ordinal {
                year: b_year,
                ddd: b_ddd,
            } => Some(compare_value((year, ddd), (b_year, b_ddd))),
            _ => None,
        },
    }
}

pub(crate) fn datetime_op(
    op_is_add: bool,
    cdt: DateTimeT,
    dur: iso8601::Duration,
    duration_negative: bool,
) -> Result<DateTimeT, String> {
    //if (op_is_add && !duration_negative) || (!op_is_add && duration_negative) {
    if op_is_add ^ duration_negative {
        datetime_add(cdt, dur)
    } else {
        datetime_sub(cdt, dur)
    }
}

pub(crate) fn datetime_add(cdt: DateTimeT, dur: iso8601::Duration) -> Result<DateTimeT, String> {
    //let cdt = chrono::DateTime::try_from(dt).unwrap();
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
        let secs = second + 60 * minute + 3600 * hour + 86400 * day;
        let cdur = chrono::TimeDelta::new(secs as i64, millisecond * 1000_000).unwrap();
        let mut d0 = cdt
            .checked_add_months(chrono::Months::new(month + year * 12))
            .unwrap();
        d0 = d0.checked_add_days(chrono::Days::new(day as u64)).unwrap();
        d0 = d0.checked_add_signed(cdur).unwrap();
        Ok(d0)
    } else {
        Err("fail to add datetime and duration".to_owned())
    }
}

pub(crate) fn datetime_sub(cdt: DateTimeT, dur: iso8601::Duration) -> Result<DateTimeT, String> {
    //let cdt = chrono::DateTime::try_from(dt).unwrap();
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
        let secs = second + 60 * minute + 3600 * hour + 86400 * day;
        let cdur = chrono::TimeDelta::new(secs as i64, millisecond * 1000_000).unwrap();
        let mut d0 = cdt
            .checked_sub_months(chrono::Months::new(month + year * 12))
            .unwrap();
        d0 = d0.checked_sub_days(chrono::Days::new(day as u64)).unwrap();
        d0 = d0.checked_sub_signed(cdur).unwrap();
        Ok(d0)
    } else {
        Err("fail to add datetime and duration".to_owned())
    }
}

pub(crate) fn timedelta_to_duration(delta: chrono::TimeDelta) -> (iso8601::Duration, bool) {
    let mut nsecs = delta.num_seconds();
    let negative = nsecs < 0;
    if negative {
        nsecs = -nsecs;
    }
    let nano = delta.num_nanoseconds().unwrap_or_default().abs();

    let day = nsecs / 86400;
    let hour = (nsecs - day * 86400) / 3600;
    let min = (nsecs - day * 86400 - hour * 3600) / 60;
    let sec = nsecs - day * 86400 - hour * 3600 - 60 * min;
    (
        iso8601::Duration::YMDHMS {
            year: 0,
            month: 0,
            day: day as u32,
            hour: hour as u32,
            minute: min as u32,
            second: sec as u32,
            millisecond: (nano / 1000_000) as u32,
        },
        negative,
    )
}

pub(crate) fn date_to_datetime(date: iso8601::Date) -> DateTimeT {
    let ndate = chrono::NaiveDate::try_from(date).unwrap();
    let ntime = chrono::NaiveTime::from_str("00:00:00").unwrap();
    let ndt = chrono::NaiveDateTime::new(ndate, ntime);
    let nowdt = chrono::Local::now();
    let cdt: DateTimeT = chrono::DateTime::from_naive_utc_and_offset(ndt, nowdt.offset().clone());
    cdt
}

pub(crate) fn day_of_week(cdt: DateTimeT) -> String {
    WEEK_NAMES[cdt.weekday().num_days_from_monday() as usize].clone()
}

#[cfg(test)]
mod test {
    use super::parse_temporal;
    use crate::values::value::Value;
    use core::assert_matches::assert_matches;
    extern crate chrono;
    extern crate iso8601;
    use chrono::Datelike;

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
        assert_matches!(
            parse_temporal("PT2H3M"),
            Ok(Value::DurationV {
                duration: _,
                negative: false
            })
        );
    }

    #[test]
    fn test_chrono_functions() {
        let dt = iso8601::datetime("2021-02-27T08:00:00+08:00").unwrap();
        //let dur = iso8601::duration("PT2H3M").unwrap();
        //assert_eq!(dt.date.year, 2020);
        let cdt = chrono::DateTime::try_from(dt).unwrap();
        //let cdur = chrono::Duration::try_from(dur);
        assert_eq!(cdt.month(), 2);
        assert_eq!(cdt.day(), 27);

        let r0 = cdt.checked_add_months(chrono::Months::new(8)).unwrap();
        assert_eq!(
            format!("{}", r0.format("%Y-%m-%dT%H:%M:%S%:z")),
            "2021-10-27T08:00:00+08:00".to_owned()
        );

        let r1 = cdt.checked_add_days(chrono::Days::new(2)).unwrap();
        assert_eq!(
            format!("{}", r1.format("%Y-%m-%dT%H:%M:%S%:z")),
            "2021-03-01T08:00:00+08:00".to_owned()
        );

        let r2 = r0 - r1;
        assert_eq!(r2.to_string(), "PT20736000S");
    }
}
