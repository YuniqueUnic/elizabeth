use chrono::{DateTime, NaiveDateTime};
use sqlx::{Error, Row, any::AnyRow};

pub const DEFAULT_FORMAT_WITH_FRACTION: &str = "%Y-%m-%d %H:%M:%S%.f";
pub const DEFAULT_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn parse_any_timestamp(raw: &str) -> Result<NaiveDateTime, chrono::ParseError> {
    NaiveDateTime::parse_from_str(raw, DEFAULT_FORMAT_WITH_FRACTION)
        .or_else(|_| NaiveDateTime::parse_from_str(raw, DEFAULT_FORMAT))
        // PostgreSQL default `TIMESTAMPTZ::text` format:
        // - "2026-01-21 06:01:33.950326+00"
        // - "2026-01-21 06:01:33+00"
        .or_else(|_| {
            DateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S%.f%#z").map(|dt| dt.naive_utc())
        })
        .or_else(|_| DateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S%#z").map(|dt| dt.naive_utc()))
        // Alternate offset format:
        // - "2026-01-21 06:01:33.950326+00:00"
        // - "2026-01-21 06:01:33+00:00"
        .or_else(|_| {
            DateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S%.f%:z").map(|dt| dt.naive_utc())
        })
        .or_else(|_| DateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S%:z").map(|dt| dt.naive_utc()))
        .or_else(|_| DateTime::parse_from_rfc3339(raw).map(|dt| dt.naive_utc()))
}

pub fn format_naive_datetime(dt: NaiveDateTime) -> String {
    dt.format(DEFAULT_FORMAT_WITH_FRACTION).to_string()
}

pub fn format_optional_naive_datetime(dt: Option<NaiveDateTime>) -> Option<String> {
    dt.map(format_naive_datetime)
}

pub fn read_datetime_from_any(row: &AnyRow, column: &str) -> Result<NaiveDateTime, Error> {
    let raw: String = row.try_get(column)?;
    parse_any_timestamp(raw.trim()).map_err(|err| Error::Decode(Box::new(err)))
}

pub fn read_optional_datetime_from_any(
    row: &AnyRow,
    column: &str,
) -> Result<Option<NaiveDateTime>, Error> {
    let raw: Option<String> = row.try_get(column)?;
    match raw {
        Some(value) => parse_any_timestamp(value.trim())
            .map(Some)
            .map_err(|err| Error::Decode(Box::new(err))),
        None => Ok(None),
    }
}

pub fn read_bool_from_any(row: &AnyRow, column: &str) -> Result<bool, Error> {
    if let Ok(b) = row.try_get::<bool, _>(column) {
        return Ok(b);
    }
    if let Ok(i) = row.try_get::<i64, _>(column) {
        return Ok(i != 0);
    }
    if let Ok(i) = row.try_get::<i32, _>(column) {
        return Ok(i != 0);
    }
    let s: String = row.try_get(column)?;
    match s.trim().to_lowercase().as_str() {
        "1" | "true" | "t" | "yes" => Ok(true),
        "0" | "false" | "f" | "no" => Ok(false),
        _ => Err(Error::Decode(
            format!("Invalid boolean value for column '{column}': {s}").into(),
        )),
    }
}
