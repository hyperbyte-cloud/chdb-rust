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

use arrow::array::RecordBatchReader;
use arrow::datatypes::DataType;
use chdb_rust::arrow_options::ArrowOptions;

#[test]
fn a_one_shot_query_returns_every_row() {
    let conn = Connection::open_in_memory().expect("open");

    let reader = conn
        .query_arrow("SELECT number FROM numbers(1000)")
        .expect("query_arrow");

    let rows: usize = reader.map(|b| b.expect("batch").num_rows()).sum();
    assert_eq!(rows, 1000);
}

#[test]
fn a_one_shot_query_matches_the_streamed_result() {
    let mut conn = Connection::open_in_memory().expect("open");
    let sql = "SELECT number, toString(number) AS s FROM numbers(500)";

    let one_shot: usize = conn
        .query_arrow(sql)
        .expect("query_arrow")
        .map(|b| b.expect("batch").num_rows())
        .sum();

    let mut stream = conn.query_stream_arrow(sql).expect("stream");
    let mut streamed = 0usize;
    while let Some(batch) = stream.next_batch().expect("batch") {
        streamed += batch.num_rows();
    }

    assert_eq!(one_shot, streamed);
}

#[test]
fn low_cardinality_becomes_a_dictionary_when_asked() {
    let conn = Connection::open_in_memory().expect("open");
    let sql = "SELECT CAST('x', 'LowCardinality(String)') AS c";

    let default = conn.query_arrow(sql).expect("default");
    assert!(
        !matches!(
            default.schema().field(0).data_type(),
            DataType::Dictionary(..)
        ),
        "the engine default materializes LowCardinality to its base type"
    );

    let opts = ArrowOptions {
        low_cardinality_as_dictionary: true,
        ..ArrowOptions::default()
    };
    let dict = conn.query_arrow_with_opts(sql, &opts).expect("dictionary");
    assert!(
        matches!(dict.schema().field(0).data_type(), DataType::Dictionary(..)),
        "got {:?}",
        dict.schema().field(0).data_type()
    );
}

#[test]
fn strings_can_be_emitted_as_binary() {
    let conn = Connection::open_in_memory().expect("open");
    let sql = "SELECT 'hello' AS s";

    let opts = ArrowOptions {
        string_as_string: false,
        ..ArrowOptions::default()
    };
    let reader = conn.query_arrow_with_opts(sql, &opts).expect("binary");

    assert_eq!(reader.schema().field(0).data_type(), &DataType::Binary);
}

#[test]
fn options_reach_the_streaming_path_too() {
    let mut conn = Connection::open_in_memory().expect("open");
    let opts = ArrowOptions {
        low_cardinality_as_dictionary: true,
        ..ArrowOptions::default()
    };

    let mut stream = conn
        .query_stream_arrow_with_opts("SELECT CAST('x', 'LowCardinality(String)') AS c", &opts)
        .expect("stream");

    let batch = stream.next_batch().expect("batch").expect("one batch");
    assert!(matches!(
        batch.schema().field(0).data_type(),
        DataType::Dictionary(..)
    ));
}
