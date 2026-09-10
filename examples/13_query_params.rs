//! Server-side parameter binding: values never touch the SQL text.

use chdb_rust::connection::Connection;
use chdb_rust::format::OutputFormat;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open_in_memory()?;

    let result = conn.query_with_params(
        "SELECT {greeting:String} AS g, {n:Int64} * 2 AS doubled",
        OutputFormat::JSONEachRow,
        &[("greeting", "hello"), ("n", "21")],
    )?;
    println!("{}", result.data_utf8_lossy());

    // A value that would be dangerous if interpolated is just a string here.
    let result = conn.query_with_params(
        "SELECT {s:String} AS literal",
        OutputFormat::JSONEachRow,
        &[("s", "'; DROP TABLE users; --")],
    )?;
    println!("{}", result.data_utf8_lossy());

    Ok(())
}
