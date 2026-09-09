//! Query text is passed to the engine by pointer and length, so a NUL byte
//! inside a string literal is data rather than a terminator.

use chdb_rust::connection::Connection;
use chdb_rust::format::OutputFormat;

#[test]
fn interior_nul_in_a_string_literal_is_data() {
    let conn = Connection::open_in_memory().expect("open");

    // The literal holds a real NUL. Under CString this returned Error::Nul
    // before the query ever reached the engine.
    let sql = "SELECT length('a\0b') AS n";

    let result = conn
        .query(sql, OutputFormat::TabSeparated)
        .expect("query with an interior NUL should reach the engine");

    assert_eq!(result.data_utf8_lossy().trim(), "3");
}

#[test]
fn plain_queries_are_unaffected() {
    let conn = Connection::open_in_memory().expect("open");
    let result = conn
        .query("SELECT 1 + 1 AS sum", OutputFormat::TabSeparated)
        .expect("query");
    assert_eq!(result.data_utf8_lossy().trim(), "2");
}
