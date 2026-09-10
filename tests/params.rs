//! Server-side {name:Type} binding. Values reach the engine as strings and are
//! never interpolated into SQL.

use chdb_rust::connection::Connection;
use chdb_rust::format::OutputFormat;

#[test]
fn a_typed_placeholder_round_trips() {
    let conn = Connection::open_in_memory().expect("open");

    let result = conn
        .query_with_params(
            "SELECT {x:Int64} + 1 AS v",
            OutputFormat::TabSeparated,
            &[("x", "41")],
        )
        .expect("query");

    assert_eq!(result.data_utf8_lossy().trim(), "42");
}

#[test]
fn several_parameters_bind_by_name_not_position() {
    let conn = Connection::open_in_memory().expect("open");

    let result = conn
        .query_with_params(
            "SELECT concat({b:String}, {a:String}) AS v",
            OutputFormat::TabSeparated,
            &[("a", "world"), ("b", "hello ")],
        )
        .expect("query");

    assert_eq!(result.data_utf8_lossy().trim(), "hello world");
}

#[test]
fn an_injection_payload_is_bound_as_data_and_never_executed() {
    let conn = Connection::open_in_memory().expect("open");

    conn.query(
        "CREATE TABLE t (a UInt64) ENGINE = MergeTree ORDER BY a",
        OutputFormat::TabSeparated,
    )
    .expect("create table");

    // If this were interpolated into the SQL text it would close the string
    // literal and run a second statement, dropping `t`. Bound, it is just a
    // String value: `t` surviving below is the actual proof the payload never
    // executed, not a byte-for-byte round-trip of the value.
    let result = conn
        .query_with_params(
            "SELECT {s:String} AS v",
            OutputFormat::JSONEachRow,
            &[("s", "'; DROP TABLE t; --")],
        )
        .expect("query");

    let survives = conn
        .query(
            "SELECT count() FROM system.tables WHERE name = 't'",
            OutputFormat::TabSeparated,
        )
        .expect("check table survival");
    assert_eq!(
        survives.data_utf8_lossy().trim(),
        "1",
        "table t must survive an unexecuted DROP"
    );

    assert_eq!(
        result.data_utf8_lossy().trim(),
        "{\"v\":\"'; DROP TABLE t; --\"}"
    );
}

#[test]
fn an_empty_slice_behaves_like_a_plain_query() {
    let conn = Connection::open_in_memory().expect("open");

    let result = conn
        .query_with_params("SELECT 7 AS v", OutputFormat::TabSeparated, &[])
        .expect("query");

    assert_eq!(result.data_utf8_lossy().trim(), "7");
}

#[test]
fn a_missing_parameter_is_an_error_not_a_panic() {
    let conn = Connection::open_in_memory().expect("open");

    let err = conn
        .query_with_params("SELECT {x:Int64} AS v", OutputFormat::TabSeparated, &[])
        .expect_err("an unbound placeholder must fail");

    assert!(
        matches!(err, chdb_rust::error::Error::QueryError(_)),
        "expected QueryError, got {err:?}"
    );
}

#[test]
fn a_format_stream_binds_parameters() {
    let mut conn = Connection::open_in_memory().expect("open");

    let mut stream = conn
        .query_stream_with_params(
            "SELECT number FROM numbers({n:UInt64})",
            OutputFormat::TabSeparated,
            &[("n", "5")],
        )
        .expect("stream");

    let mut rows = 0usize;
    while let Some(chunk) = stream.next_chunk().expect("chunk") {
        rows += chunk.data_utf8_lossy().lines().count();
    }

    assert_eq!(rows, 5);
}

#[test]
fn a_parameter_value_may_contain_an_interior_nul() {
    let conn = Connection::open_in_memory().expect("open");

    // "a\0b" is 3 bytes; if the value were truncated at the NUL (e.g. if it
    // were passed through as a C string) length() would come back 1.
    let value = "a\0b";
    let result = conn
        .query_with_params(
            "SELECT length({s:String}) AS v",
            OutputFormat::TabSeparated,
            &[("s", value)],
        )
        .expect("query");

    assert_eq!(
        result.data_utf8_lossy().trim(),
        value.len().to_string(),
        "expected the engine to see the full byte length, interior NUL included"
    );
}
