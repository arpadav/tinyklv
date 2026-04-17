#![cfg(feature = "chrono")]
use tinyklv::prelude::*;

#[test]
fn parse_date_ymd() {
    let mut input: &[u8] = b"2020-12-31";
    let date = tinyklv::as_date!(tinyklv::dec::binary::to_string_utf8, "%Y-%m-%d", 10)(&mut input);
    assert_eq!(
        date,
        Ok(chrono::NaiveDate::from_ymd_opt(2020, 12, 31).unwrap())
    );
}

#[test]
fn parse_date_invalid() {
    let mut input: &[u8] = b"not-a-date";
    let date = tinyklv::as_date!(tinyklv::dec::binary::to_string_utf8, "%Y-%m-%d", 10)(&mut input);
    assert!(date.is_err());
}

#[test]
fn parse_date_wrong_format() {
    // DD-MM-YYYY supplied but parser expects YYYY-MM-DD
    let mut input: &[u8] = b"31-12-2020";
    let date = tinyklv::as_date!(tinyklv::dec::binary::to_string_utf8, "%Y-%m-%d", 10)(&mut input);
    assert!(date.is_err());
}

#[test]
fn parse_date_leap_year() {
    let mut input: &[u8] = b"2024-02-29";
    let date = tinyklv::as_date!(tinyklv::dec::binary::to_string_utf8, "%Y-%m-%d", 10)(&mut input);
    assert_eq!(
        date,
        Ok(chrono::NaiveDate::from_ymd_opt(2024, 2, 29).unwrap())
    );
}

#[test]
fn parse_date_non_leap_year_feb29() {
    let mut input: &[u8] = b"2023-02-29";
    let date = tinyklv::as_date!(tinyklv::dec::binary::to_string_utf8, "%Y-%m-%d", 10)(&mut input);
    assert!(date.is_err());
}

#[test]
fn parse_date_epoch() {
    let mut input: &[u8] = b"1970-01-01";
    let date = tinyklv::as_date!(tinyklv::dec::binary::to_string_utf8, "%Y-%m-%d", 10)(&mut input);
    assert_eq!(
        date,
        Ok(chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
    );
}

#[test]
fn parse_date_end_of_year() {
    let mut input: &[u8] = b"2023-12-31";
    let date = tinyklv::as_date!(tinyklv::dec::binary::to_string_utf8, "%Y-%m-%d", 10)(&mut input);
    assert_eq!(
        date,
        Ok(chrono::NaiveDate::from_ymd_opt(2023, 12, 31).unwrap())
    );
}

#[test]
fn parse_date_dmy_format() {
    // DD/MM/YYYY format
    let mut input: &[u8] = b"25/06/2025";
    let date = tinyklv::as_date!(tinyklv::dec::binary::to_string_utf8, "%d/%m/%Y", 10)(&mut input);
    assert_eq!(
        date,
        Ok(chrono::NaiveDate::from_ymd_opt(2025, 6, 25).unwrap())
    );
}

#[test]
fn parse_time_hms() {
    let mut input: &[u8] = b"12:34:56";
    let time = tinyklv::as_time!(tinyklv::dec::binary::to_string_utf8, "%H:%M:%S", 8)(&mut input);
    assert_eq!(
        time,
        Ok(chrono::NaiveTime::from_hms_opt(12, 34, 56).unwrap())
    );
}

#[test]
fn parse_time_midnight() {
    let mut input: &[u8] = b"00:00:00";
    let time = tinyklv::as_time!(tinyklv::dec::binary::to_string_utf8, "%H:%M:%S", 8)(&mut input);
    assert_eq!(time, Ok(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()));
}

#[test]
fn parse_time_end_of_day() {
    let mut input: &[u8] = b"23:59:59";
    let time = tinyklv::as_time!(tinyklv::dec::binary::to_string_utf8, "%H:%M:%S", 8)(&mut input);
    assert_eq!(
        time,
        Ok(chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap())
    );
}

#[test]
fn parse_time_invalid() {
    // Hour 25 is out of range
    let mut input: &[u8] = b"25:00:00";
    let time = tinyklv::as_time!(tinyklv::dec::binary::to_string_utf8, "%H:%M:%S", 8)(&mut input);
    assert!(time.is_err());
}

#[test]
fn parse_time_invalid_minute() {
    // Minute 60 is out of range
    let mut input: &[u8] = b"12:60:00";
    let time = tinyklv::as_time!(tinyklv::dec::binary::to_string_utf8, "%H:%M:%S", 8)(&mut input);
    assert!(time.is_err());
}

#[test]
fn parse_time_not_a_time() {
    let mut input: &[u8] = b"garbage!";
    let time = tinyklv::as_time!(tinyklv::dec::binary::to_string_utf8, "%H:%M:%S", 8)(&mut input);
    assert!(time.is_err());
}

#[test]
fn parse_datetime() {
    let mut input: &[u8] = b"2020-12-31 12:34:56";
    let dt = tinyklv::as_datetime!(
        tinyklv::dec::binary::to_string_utf8,
        "%Y-%m-%d %H:%M:%S",
        19
    )(&mut input);
    assert_eq!(
        dt,
        Ok(chrono::NaiveDate::from_ymd_opt(2020, 12, 31)
            .unwrap()
            .and_hms_opt(12, 34, 56)
            .unwrap())
    );
}

#[test]
fn parse_datetime_invalid() {
    let mut input: &[u8] = b"not-a-datetime-value";
    let dt = tinyklv::as_datetime!(
        tinyklv::dec::binary::to_string_utf8,
        "%Y-%m-%d %H:%M:%S",
        20
    )(&mut input);
    assert!(dt.is_err());
}

#[test]
fn parse_datetime_midnight() {
    let mut input: &[u8] = b"2000-01-01 00:00:00";
    let dt = tinyklv::as_datetime!(
        tinyklv::dec::binary::to_string_utf8,
        "%Y-%m-%d %H:%M:%S",
        19
    )(&mut input);
    assert_eq!(
        dt,
        Ok(chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap())
    );
}

#[test]
fn parse_datetime_end_of_day() {
    let mut input: &[u8] = b"1999-12-31 23:59:59";
    let dt = tinyklv::as_datetime!(
        tinyklv::dec::binary::to_string_utf8,
        "%Y-%m-%d %H:%M:%S",
        19
    )(&mut input);
    assert_eq!(
        dt,
        Ok(chrono::NaiveDate::from_ymd_opt(1999, 12, 31)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap())
    );
}

#[test]
fn parse_datetime_wrong_separator() {
    // 'T' separator vs space
    let mut input: &[u8] = b"2020-12-31T12:34:56";
    let dt = tinyklv::as_datetime!(
        tinyklv::dec::binary::to_string_utf8,
        "%Y-%m-%d %H:%M:%S",
        19
    )(&mut input);
    assert!(dt.is_err());
}

#[test]
fn parse_datetime_iso8601_t_separator() {
    // Parse with explicit 'T' separator format
    let mut input: &[u8] = b"2020-12-31T12:34:56";
    let dt = tinyklv::as_datetime!(
        tinyklv::dec::binary::to_string_utf8,
        "%Y-%m-%dT%H:%M:%S",
        19
    )(&mut input);
    assert_eq!(
        dt,
        Ok(chrono::NaiveDate::from_ymd_opt(2020, 12, 31)
            .unwrap()
            .and_hms_opt(12, 34, 56)
            .unwrap())
    );
}
