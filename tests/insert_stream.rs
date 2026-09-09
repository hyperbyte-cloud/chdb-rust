//! Write-side streaming INSERT: open, push chunks, finalise.

use chdb_rust::connection::Connection;
use chdb_rust::format::{InputFormat, OutputFormat};

fn conn_with_table() -> Connection {
    let conn = Connection::open_in_memory().expect("open");
    conn.query(
        "CREATE TABLE t (a UInt64, b String) ENGINE = MergeTree ORDER BY a",
        OutputFormat::TabSeparated,
    )
    .expect("create");
    conn
}

fn count(conn: &Connection) -> u64 {
    conn.query("SELECT count() FROM t", OutputFormat::TabSeparated)
        .expect("count")
        .data_utf8_lossy()
        .trim()
        .parse()
        .expect("parse count")
}

#[test]
fn a_csv_stream_commits_every_chunk() {
    let mut conn = conn_with_table();

    let mut ins = conn
        .insert_stream("INSERT INTO t (a, b)", InputFormat::CSV)
        .expect("open stream");
    ins.append(b"1,\"one\"\n").expect("append");
    ins.append(b"2,\"two\"\n3,\"three\"\n").expect("append");
    let stats = ins.finish().expect("finish");

    assert_eq!(stats.rows_written, 3);
    assert!(stats.bytes_written > 0);
    assert_eq!(count(&conn), 3);
}

#[test]
fn a_json_each_row_stream_commits() {
    let mut conn = conn_with_table();

    let mut ins = conn
        .insert_stream("INSERT INTO t (a, b)", InputFormat::JSONEachRow)
        .expect("open stream");
    ins.append(br#"{"a":1,"b":"one"}"#).expect("append");
    ins.append(b"\n").expect("append");
    let stats = ins.finish().expect("finish");

    assert_eq!(stats.rows_written, 1);
    assert_eq!(count(&conn), 1);
}

#[test]
fn a_malformed_chunk_surfaces_an_error() {
    let mut conn = conn_with_table();

    let mut ins = conn
        .insert_stream("INSERT INTO t (a, b)", InputFormat::CSV)
        .expect("open stream");

    // "not-a-number" cannot parse as UInt64. The engine may reject it on the
    // append that carries it or when the stream is finalised; either is a
    // failure, and neither may be silent.
    let appended = ins.append(b"not-a-number,\"x\"\n");
    let finished = ins.finish();
    assert!(
        appended.is_err() || finished.is_err(),
        "a malformed row must fail on append or on finish"
    );

    assert_eq!(count(&conn), 0);
}

#[test]
fn dropping_without_finishing_commits_nothing() {
    let mut conn = conn_with_table();

    {
        let mut ins = conn
            .insert_stream("INSERT INTO t (a, b)", InputFormat::CSV)
            .expect("open stream");
        ins.append(b"1,\"one\"\n").expect("append");
        // Dropped here without finish(): cancelled, then destroyed.
    }

    assert_eq!(count(&conn), 0);
}

#[test]
fn the_connection_is_usable_again_after_finishing() {
    let mut conn = conn_with_table();

    let mut ins = conn
        .insert_stream("INSERT INTO t (a, b)", InputFormat::CSV)
        .expect("open stream");
    ins.append(b"1,\"one\"\n").expect("append");
    ins.finish().expect("finish");

    assert_eq!(count(&conn), 1);
}

#[test]
fn opening_a_bad_statement_fails_at_open() {
    let mut conn = conn_with_table();

    let err = conn
        .insert_stream("INSERT INTO no_such_table (a)", InputFormat::CSV)
        .expect_err("a missing table must fail when the stream is opened");

    assert!(
        matches!(err, chdb_rust::error::Error::QueryError(_)),
        "expected QueryError, got {err:?}"
    );
}
