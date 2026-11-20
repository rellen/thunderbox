//! Tests that run the data source against a server that answers over HTTP.
//!
//! A unit test checks each step of the work on its own. These tests check the
//! whole path: the data source builds a request, a server reads that request,
//! and the answer of the server becomes a frame.
//!
//! The server in this file is not a Prometheus server. It writes an answer that
//! the test holds, and it keeps the target of each request. A test can then
//! state what the data source must ask for. The point that matters most is the
//! step, because the panel decides the step and the server obeys it.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use thunderbox_core::{Query, QueryRequest, RelativeTime, TimeRange};
use thunderbox_source::DataSource;
use thunderbox_source_prometheus::{Prometheus, Settings};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// A server that answers every request with the same text.
struct FakeServer {
    /// The address of the server, as `host:port`.
    address: String,
    /// The target of each request that the server read, in order.
    requests: Arc<Mutex<Vec<String>>>,
}

impl FakeServer {
    /// Starts a server that answers with `body` and the status line `status`.
    async fn start(status: &'static str, body: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("The test must be able to take a port.");
        let address = listener
            .local_addr()
            .expect("The test must be able to read the port.")
            .to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    return;
                };
                let seen = seen.clone();
                tokio::spawn(async move {
                    let _ = answer(stream, status, body, seen).await;
                });
            }
        });
        Self { address, requests }
    }

    /// Returns the target of the request that the server read first.
    fn first_request(&self) -> String {
        self.requests
            .lock()
            .expect("The test must be able to read the requests.")
            .first()
            .cloned()
            .expect("The data source must send a request.")
    }

    /// Returns the parameters of the request that the server read first.
    fn first_parameters(&self) -> BTreeMap<String, String> {
        parameters(&self.first_request())
    }

    /// Returns the number of requests that the server read.
    fn count(&self) -> usize {
        self.requests
            .lock()
            .expect("The test must be able to read the requests.")
            .len()
    }
}

/// Reads one request and writes the answer.
async fn answer(
    mut stream: TcpStream,
    status: &str,
    body: &str,
    seen: Arc<Mutex<Vec<String>>>,
) -> std::io::Result<()> {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).await? == 0 {
            return Ok(());
        }
        head.push(byte[0]);
    }
    let text = String::from_utf8_lossy(&head).to_string();
    let target = text
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_string();
    seen.lock()
        .expect("The server must be able to keep the request.")
        .push(target);

    let answer = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(answer.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

/// Returns the parameters of a target, with each value in its plain form.
fn parameters(target: &str) -> BTreeMap<String, String> {
    let Some((_, query)) = target.split_once('?') else {
        return BTreeMap::new();
    };
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (decode(k), decode(v)))
        .collect()
}

/// Returns the plain form of a text that a request holds.
///
/// A request writes a space as a plus sign. It writes every other character
/// that a query string reserves as a percent sign and two digits.
fn decode(text: &str) -> String {
    let bytes = text.replace('+', " ").into_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let pair = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or_default();
            if let Ok(byte) = u8::from_str_radix(pair, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// The answer of a server that holds two series of four points.
const TWO_SERIES: &str = r#"{
    "status": "success",
    "data": {
        "resultType": "matrix",
        "result": [
            {
                "metric": {"__name__": "up", "instance": "web-1", "job": "node"},
                "values": [
                    [1700000000, "1"], [1700000002, "1"],
                    [1700000004, "0"], [1700000006, "1"]
                ]
            },
            {
                "metric": {"__name__": "up", "instance": "web-2", "job": "node"},
                "values": [[1700000000, "1"], [1700000006, "1"]]
            }
        ]
    }
}"#;

/// Returns a request that covers `span_ms` and asks for `limit` points.
fn request(expr: &str, span_ms: i64, limit: u32) -> QueryRequest {
    QueryRequest::new(
        Query {
            source: "prom".into(),
            expr: expr.into(),
        },
        &TimeRange {
            from: RelativeTime::Offset(-span_ms),
            to: RelativeTime::Now,
        },
        1_700_000_006_000,
        limit,
    )
}

/// Returns a data source that reads the server.
fn source(server: &FakeServer) -> Prometheus {
    Prometheus::new(&format!("http://{}", server.address), Settings::default())
        .expect("The address of the test server must be correct.")
}

#[tokio::test]
async fn the_point_limit_of_the_panel_becomes_the_step_of_the_query() {
    let server = FakeServer::start("200 OK", TWO_SERIES).await;
    source(&server)
        .query(request("up", 300_000, 160))
        .await
        .expect("The server answers, so the query must not return an error.");

    let parameters = server.first_parameters();
    assert_eq!(server.count(), 1, "One panel makes one request.");
    assert!(
        server.first_request().starts_with("/api/v1/query_range?"),
        "The data source must ask for a range. It asked for {}.",
        server.first_request()
    );
    assert_eq!(parameters["query"], "up");
    // Five minutes across 160 points suggests 1875 milliseconds. The data
    // source raises that step to the next round value, which is 2 seconds.
    assert_eq!(parameters["step"], "2");

    let start: i64 = parameters["start"].parse().unwrap();
    let end: i64 = parameters["end"].parse().unwrap();
    let points = (end - start) / 2 + 1;
    assert!(
        points <= 160,
        "The server must return {points} points, and the panel draws 160."
    );
}

#[tokio::test]
async fn a_panel_that_is_half_as_wide_asks_for_half_the_points() {
    let wide = FakeServer::start("200 OK", TWO_SERIES).await;
    let narrow = FakeServer::start("200 OK", TWO_SERIES).await;
    source(&wide)
        .query(request("up", 3_600_000, 320))
        .await
        .ok();
    source(&narrow)
        .query(request("up", 3_600_000, 40))
        .await
        .ok();

    let wide_step: f64 = wide.first_parameters()["step"].parse().unwrap();
    let narrow_step: f64 = narrow.first_parameters()["step"].parse().unwrap();
    assert!(
        narrow_step > wide_step,
        "A narrow panel must ask the server for fewer points. The steps were \
         {narrow_step} and {wide_step}."
    );
}

#[tokio::test]
async fn both_ends_of_the_range_fall_on_the_step() {
    let server = FakeServer::start("200 OK", TWO_SERIES).await;
    source(&server)
        .query(request("up", 300_000, 160))
        .await
        .ok();

    let parameters = server.first_parameters();
    let start: i64 = parameters["start"].parse().unwrap();
    let end: i64 = parameters["end"].parse().unwrap();
    let step: i64 = parameters["step"].parse().unwrap();
    assert_eq!(
        start % step,
        0,
        "The chart must not shake while it refreshes."
    );
    assert_eq!(end % step, 0);
}

#[tokio::test]
async fn the_smallest_step_of_the_dashboard_reaches_the_server() {
    let server = FakeServer::start("200 OK", TWO_SERIES).await;
    let settings = Settings {
        min_step_ms: 60_000,
        ..Settings::default()
    };
    let prometheus = Prometheus::new(&format!("http://{}", server.address), settings)
        .expect("The address of the test server must be correct.");
    prometheus.query(request("up", 300_000, 160)).await.ok();

    assert_eq!(server.first_parameters()["step"], "60");
}

#[tokio::test]
async fn the_query_reaches_the_server_without_a_change() {
    let server = FakeServer::start("200 OK", TWO_SERIES).await;
    let expr = r#"rate(http_requests_total{job="api"}[5m])"#;
    source(&server)
        .query(request(expr, 300_000, 160))
        .await
        .ok();

    assert_eq!(
        server.first_parameters()["query"],
        expr,
        "thunderbox reads no query. It must send the text as a person wrote it."
    );
}

#[tokio::test]
async fn the_answer_of_the_server_becomes_one_frame() {
    let server = FakeServer::start("200 OK", TWO_SERIES).await;
    let frames = source(&server)
        .query(request("up", 8_000, 5))
        .await
        .expect("The server answers, so the query must not return an error.");

    assert_eq!(frames.len(), 1);
    let (times, numeric) = frames[0]
        .time_series()
        .expect("A time series panel must be able to draw the frame.");
    assert_eq!(numeric.len(), 2, "The answer holds two series.");
    assert!(times.len() >= 4);
    assert_eq!(numeric[0].labels["instance"], "web-1");
    assert!(
        numeric[1].values.as_f64().unwrap().contains(&None),
        "The second series has no sample in the middle, so it must show a hole."
    );
}

#[tokio::test]
async fn a_query_that_matched_no_series_returns_no_frame() {
    let empty = r#"{"status":"success","data":{"resultType":"matrix","result":[]}}"#;
    let server = FakeServer::start("200 OK", empty).await;
    let frames = source(&server)
        .query(request("up", 300_000, 160))
        .await
        .unwrap();
    assert!(
        frames.is_empty(),
        "The panel must then show that it has no data."
    );
}

#[tokio::test]
async fn an_error_of_the_server_becomes_an_error_of_the_panel() {
    let body = r#"{"status":"error","errorType":"bad_data","error":"invalid parameter"}"#;
    let server = FakeServer::start("400 Bad Request", body).await;
    let error = source(&server)
        .query(request("up[", 300_000, 160))
        .await
        .expect_err("A server that refuses the query must give an error.")
        .to_string();
    assert!(
        error.contains("invalid parameter"),
        "The panel must show what the server said. It said {error}."
    );
}

#[tokio::test]
async fn a_server_that_is_not_there_becomes_an_error_of_the_panel() {
    // The port is closed, because the test drops the listener at once.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    drop(listener);

    let prometheus = Prometheus::new(&format!("http://{address}"), Settings::default()).unwrap();
    assert!(
        prometheus.query(request("up", 300_000, 160)).await.is_err(),
        "A data source that cannot answer must return an error, and it must \
         not stop the dashboard."
    );
}

#[tokio::test]
async fn label_values_returns_the_values_that_the_server_holds() {
    let body = r#"{"status":"success","data":["web-1","web-2"]}"#;
    let server = FakeServer::start("200 OK", body).await;
    let values = source(&server).label_values("instance").await.unwrap();

    assert_eq!(values, vec!["web-1", "web-2"]);
    assert!(
        server.first_request().contains("/label/instance/values"),
        "The data source must ask for the values of one label. It asked for {}.",
        server.first_request()
    );
}

#[tokio::test]
async fn label_values_returns_an_error_for_a_query_that_holds_no_label() {
    let server = FakeServer::start("200 OK", TWO_SERIES).await;
    assert!(source(&server).label_values("  ").await.is_err());
    assert_eq!(server.count(), 0, "The data source must send no request.");
}

/// The parameter `timeout` of a query holds a count of milliseconds. The
/// default timeout of the settings is 30 seconds.
#[tokio::test]
async fn the_server_receives_the_default_timeout_in_milliseconds() {
    let server = FakeServer::start("200 OK", TWO_SERIES).await;
    source(&server)
        .query(request("up", 300_000, 160))
        .await
        .expect("The server answers, so the query must not return an error.");

    assert_eq!(
        server.first_parameters()["timeout"],
        "30000ms",
        "The server must stop the query after 30 seconds, and not after 30 milliseconds."
    );
}

/// A large timeout reaches the server as a count of milliseconds.
///
/// A timeout that holds more milliseconds than an `i64` holds reaches the
/// server as the largest value of an `i64`.
///
/// The unit tests of `lib.rs` check a timeout of 0 seconds and a timeout of 1
/// millisecond. A test in this file cannot check them. The client stops a
/// request that has a timeout of 0 seconds before the server reads it. The
/// measurement for 1 millisecond: 2 of 200 such requests never reached the
/// server of this file.
#[tokio::test]
async fn a_large_timeout_reaches_the_server_in_milliseconds() {
    for (timeout, expected) in [
        (Duration::from_secs(365 * 86_400), "31536000000ms"),
        (Duration::MAX, "9223372036854775807ms"),
    ] {
        let server = FakeServer::start("200 OK", TWO_SERIES).await;
        let settings = Settings {
            timeout,
            ..Settings::default()
        };
        Prometheus::new(&format!("http://{}", server.address), settings)
            .expect("The address of the test server must be correct.")
            .query(request("up", 300_000, 160))
            .await
            .expect("The server answers, so the query must not return an error.");

        assert_eq!(
            server.first_parameters()["timeout"],
            expected,
            "The timeout {timeout:?} reached the server with the wrong value."
        );
    }
}
