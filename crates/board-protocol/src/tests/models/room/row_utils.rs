use crate::models::room::row_utils::{DEFAULT_FORMAT_WITH_FRACTION, parse_any_timestamp};

#[test]
fn parse_any_timestamp_accepts_sqlite_datetime() {
    let raw = "2026-01-21 06:01:33.950326";
    let parsed = parse_any_timestamp(raw).expect("should parse sqlite datetime");
    let expected =
        chrono::NaiveDateTime::parse_from_str(raw, DEFAULT_FORMAT_WITH_FRACTION).unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn parse_any_timestamp_accepts_postgres_timestamptz_cast() {
    let raw = "2026-01-21 06:01:33.950326+00";
    let parsed = parse_any_timestamp(raw).expect("should parse postgres timestamptz text");
    let expected = chrono::NaiveDateTime::parse_from_str(
        "2026-01-21 06:01:33.950326",
        DEFAULT_FORMAT_WITH_FRACTION,
    )
    .unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn parse_any_timestamp_accepts_postgres_timestamptz_cast_with_colon_offset() {
    let raw = "2026-01-21 06:01:33.950326+00:00";
    let parsed = parse_any_timestamp(raw).expect("should parse postgres timestamptz text");
    let expected = chrono::NaiveDateTime::parse_from_str(
        "2026-01-21 06:01:33.950326",
        DEFAULT_FORMAT_WITH_FRACTION,
    )
    .unwrap();
    assert_eq!(parsed, expected);
}
