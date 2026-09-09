//! Arrow-side entry points: parameter binding on the streaming path, one-shot
//! export, and type-mapping options.

use chdb_rust::connection::Connection;

#[test]
fn an_arrow_stream_binds_parameters() {
    let mut conn = Connection::open_in_memory().expect("open");

    let mut stream = conn
        .query_stream_arrow_with_params("SELECT number FROM numbers({n:UInt64})", &[("n", "3")])
        .expect("stream");

    let mut rows = 0usize;
    while let Some(batch) = stream.next_batch().expect("batch") {
        rows += batch.num_rows();
    }

    assert_eq!(rows, 3);
}
