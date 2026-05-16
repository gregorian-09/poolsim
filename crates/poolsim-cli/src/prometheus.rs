use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::Path,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use poolsim_core::{
    telemetry::TelemetrySnapshot,
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};
use serde::Deserialize;
use serde_json::Value;

use crate::{args::PrometheusImportArgs, config::TelemetryInput};

const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
struct QuerySet {
    rps: String,
    p50: String,
    p95: String,
    p99: String,
}

#[derive(Debug, Clone, Copy)]
struct PrometheusValues {
    rps: f64,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
}

trait PrometheusClient {
    fn query(&self, query: &str) -> Result<String>;
}

#[derive(Debug, Clone)]
struct HttpPrometheusClient {
    endpoint: HttpEndpoint,
    headers: Vec<String>,
}

#[derive(Debug, Clone)]
struct HttpEndpoint {
    host: String,
    port: u16,
    base_path: String,
}

#[derive(Debug, Deserialize)]
struct ResponseFile {
    rps: Value,
    p50: Value,
    p95: Value,
    p99: Value,
}

pub(crate) fn resolve_prometheus_input(args: &PrometheusImportArgs) -> Result<TelemetryInput> {
    let values = match &args.response_file {
        Some(path) => read_values_from_response_file(path)?,
        None => {
            let endpoint = args
                .endpoint
                .as_deref()
                .ok_or_else(|| anyhow!("missing --endpoint or --response-file"))?;
            let queries = query_set_from_args(args)?;
            let client = HttpPrometheusClient::new(endpoint, &args.header)?;
            query_values(&client, &queries)?
        }
    };

    build_input(args, values)
}

fn build_input(args: &PrometheusImportArgs, values: PrometheusValues) -> Result<TelemetryInput> {
    let mut options = SimulationOptions::default();
    if let Some(value) = args.iterations {
        options.iterations = value;
    }
    if let Some(value) = args.seed {
        options.seed = Some(value);
    }
    if let Some(value) = args.distribution {
        options.distribution = value.into();
    }
    if let Some(value) = args.queue_model {
        options.queue_model = value.into();
    }
    if let Some(value) = args.target_wait_p99_ms {
        options.target_wait_p99_ms = value;
    }
    if let Some(value) = args.max_acceptable_rho {
        options.max_acceptable_rho = value;
    }

    let snapshot = TelemetrySnapshot {
        service_name: args.service_name.clone(),
        window: args.window.clone(),
        observed_at: args.observed_at.clone(),
        current_pool_size: args.current_pool_size,
        workload: WorkloadConfig {
            requests_per_second: values.rps,
            latency_p50_ms: values.p50_ms,
            latency_p95_ms: values.p95_ms,
            latency_p99_ms: values.p99_ms,
            raw_samples_ms: None,
            step_load_profile: None,
        },
        pool: PoolConfig {
            max_server_connections: args.max_server_connections,
            connection_overhead_ms: args.connection_overhead_ms,
            idle_timeout_ms: args.idle_timeout_ms,
            min_pool_size: args.min,
            max_pool_size: args.max,
        },
    };

    snapshot.validate()?;
    options.validate()?;
    Ok(TelemetryInput { snapshot, options })
}

fn query_set_from_args(args: &PrometheusImportArgs) -> Result<QuerySet> {
    Ok(QuerySet {
        rps: required_query("rps", args.rps_query.as_deref())?,
        p50: required_query("p50", args.p50_query.as_deref())?,
        p95: required_query("p95", args.p95_query.as_deref())?,
        p99: required_query("p99", args.p99_query.as_deref())?,
    })
}

fn required_query(name: &str, value: Option<&str>) -> Result<String> {
    let query = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("missing --{name}-query"))?;
    Ok(query.to_string())
}

fn query_values(client: &impl PrometheusClient, queries: &QuerySet) -> Result<PrometheusValues> {
    Ok(PrometheusValues {
        rps: parse_prometheus_value(&client.query(&queries.rps)?)
            .context("invalid rps query response")?,
        p50_ms: parse_prometheus_value(&client.query(&queries.p50)?)
            .context("invalid p50 query response")?,
        p95_ms: parse_prometheus_value(&client.query(&queries.p95)?)
            .context("invalid p95 query response")?,
        p99_ms: parse_prometheus_value(&client.query(&queries.p99)?)
            .context("invalid p99 query response")?,
    })
}

fn read_values_from_response_file(path: &Path) -> Result<PrometheusValues> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read Prometheus response file {}", path.display()))?;
    let file: ResponseFile = serde_json::from_str(&raw)
        .with_context(|| format!("invalid Prometheus response file {}", path.display()))?;

    Ok(PrometheusValues {
        rps: parse_prometheus_value_from_json(&file.rps).context("invalid rps response")?,
        p50_ms: parse_prometheus_value_from_json(&file.p50).context("invalid p50 response")?,
        p95_ms: parse_prometheus_value_from_json(&file.p95).context("invalid p95 response")?,
        p99_ms: parse_prometheus_value_from_json(&file.p99).context("invalid p99 response")?,
    })
}

fn parse_prometheus_value(raw: &str) -> Result<f64> {
    let value: Value = serde_json::from_str(raw).context("response was not JSON")?;
    parse_prometheus_value_from_json(&value)
}

fn parse_prometheus_value_from_json(value: &Value) -> Result<f64> {
    if value.get("status").and_then(Value::as_str) != Some("success") {
        let message = value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("Prometheus query did not succeed");
        bail!("{message}");
    }

    let data = value
        .get("data")
        .ok_or_else(|| anyhow!("missing data object in Prometheus response"))?;
    match data.get("resultType").and_then(Value::as_str) {
        Some("vector") => value_from_vector(data),
        Some("scalar") => value_from_pair(
            data.get("result")
                .ok_or_else(|| anyhow!("missing scalar result in Prometheus response"))?,
        ),
        Some(other) => bail!("unsupported Prometheus resultType '{other}'"),
        None => bail!("missing resultType in Prometheus response"),
    }
}

fn value_from_vector(data: &Value) -> Result<f64> {
    let result = data
        .get("result")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("missing vector result array in Prometheus response"))?;

    match result.as_slice() {
        [] => bail!("Prometheus query returned no series"),
        [series] => value_from_pair(
            series
                .get("value")
                .ok_or_else(|| anyhow!("missing value in Prometheus vector result"))?,
        ),
        _ => bail!("Prometheus query returned multiple series; aggregate the query to one series"),
    }
}

fn value_from_pair(value: &Value) -> Result<f64> {
    let pair = value
        .as_array()
        .ok_or_else(|| anyhow!("Prometheus value must be a [timestamp, value] array"))?;
    let raw_value = pair
        .get(1)
        .ok_or_else(|| anyhow!("Prometheus value array is missing the sample value"))?;

    let parsed = match raw_value {
        Value::String(text) => text
            .parse::<f64>()
            .with_context(|| format!("Prometheus sample value '{text}' is not numeric"))?,
        Value::Number(number) => number.as_f64().unwrap_or(f64::NAN),
        _ => bail!("Prometheus sample value must be a string or number"),
    };

    if !parsed.is_finite() {
        bail!("Prometheus sample value must be finite");
    }
    Ok(parsed)
}

impl HttpPrometheusClient {
    fn new(endpoint: &str, headers: &[String]) -> Result<Self> {
        let endpoint = HttpEndpoint::parse(endpoint)?;
        let headers = headers
            .iter()
            .map(|header| validate_header(header))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { endpoint, headers })
    }
}

impl PrometheusClient for HttpPrometheusClient {
    fn query(&self, query: &str) -> Result<String> {
        let path = self.endpoint.query_path(query);
        http_get(&self.endpoint, &path, &self.headers)
    }
}

impl HttpEndpoint {
    fn parse(raw: &str) -> Result<Self> {
        let without_scheme = raw
            .strip_prefix("http://")
            .ok_or_else(|| anyhow!("Prometheus endpoint must use http://"))?;
        if without_scheme.is_empty() {
            bail!("Prometheus endpoint is missing host");
        }

        let (authority, path) = without_scheme
            .split_once('/')
            .map(|(authority, path)| (authority, format!("/{path}")))
            .unwrap_or((without_scheme, String::new()));
        if authority.is_empty() {
            bail!("Prometheus endpoint is missing host");
        }

        let (host, port) = parse_authority(authority)?;
        Ok(Self {
            host,
            port,
            base_path: normalize_base_path(&path),
        })
    }

    fn query_path(&self, query: &str) -> String {
        format!(
            "{}/api/v1/query?query={}",
            self.base_path,
            percent_encode(query)
        )
    }
}

fn parse_authority(authority: &str) -> Result<(String, u16)> {
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => {
            let parsed = port
                .parse::<u16>()
                .with_context(|| format!("invalid Prometheus endpoint port '{port}'"))?;
            (host, parsed)
        }
        None => (authority, 80),
    };

    if host.is_empty() {
        bail!("Prometheus endpoint is missing host");
    }
    Ok((host.to_string(), port))
}

fn normalize_base_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        trimmed.to_string()
    }
}

fn validate_header(raw: &str) -> Result<String> {
    if raw.contains('\r') || raw.contains('\n') {
        bail!("Prometheus headers must not contain newlines");
    }
    let (name, value) = raw
        .split_once(':')
        .ok_or_else(|| anyhow!("Prometheus header must use 'Name: value' format"))?;
    if name.trim().is_empty() || value.trim().is_empty() {
        bail!("Prometheus header must include a non-empty name and value");
    }
    Ok(format!("{}: {}", name.trim(), value.trim()))
}

fn http_get(endpoint: &HttpEndpoint, path: &str, headers: &[String]) -> Result<String> {
    let mut stream =
        TcpStream::connect((endpoint.host.as_str(), endpoint.port)).with_context(|| {
            format!(
                "failed to connect to Prometheus endpoint {}:{}",
                endpoint.host, endpoint.port
            )
        })?;
    stream.set_read_timeout(Some(DEFAULT_HTTP_TIMEOUT)).ok();
    stream.set_write_timeout(Some(DEFAULT_HTTP_TIMEOUT)).ok();

    let mut request = format!(
        "GET {path} HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\nConnection: close\r\n",
        endpoint.host
    );
    for header in headers {
        request.push_str(header);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");

    stream
        .write_all(request.as_bytes())
        .context("failed to write Prometheus HTTP request")?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .context("failed to read Prometheus HTTP response")?;
    parse_http_response(&response)
}

fn parse_http_response(response: &str) -> Result<String> {
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| anyhow!("invalid HTTP response from Prometheus"))?;
    let status_line = head
        .lines()
        .next()
        .ok_or_else(|| anyhow!("missing HTTP status line from Prometheus"))?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow!("missing HTTP status code from Prometheus"))?
        .parse::<u16>()
        .context("invalid HTTP status code from Prometheus")?;

    let body = if head
        .lines()
        .any(|line| line.eq_ignore_ascii_case("transfer-encoding: chunked"))
    {
        decode_chunked_body(body)?
    } else {
        body.to_string()
    };

    if !(200..300).contains(&status) {
        bail!("Prometheus returned HTTP {status}: {body}");
    }
    Ok(body)
}

fn decode_chunked_body(body: &str) -> Result<String> {
    let mut remaining = body;
    let mut decoded = String::new();

    loop {
        let (size_line, rest) = remaining
            .split_once("\r\n")
            .ok_or_else(|| anyhow!("invalid chunked response from Prometheus"))?;
        let size_text = size_line.split(';').next().unwrap_or(size_line).trim();
        let size = usize::from_str_radix(size_text, 16)
            .with_context(|| format!("invalid chunk size '{size_text}' from Prometheus"))?;
        if size == 0 {
            return Ok(decoded);
        }
        if rest.len() < size + 2 {
            bail!("chunked response ended before chunk body completed");
        }
        decoded.push_str(&rest[..size]);
        remaining = &rest[size + 2..];
    }
}

fn percent_encode(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use std::{io::ErrorKind, net::TcpListener, thread};

    use crate::args::{CliDistributionModel, CliQueueModel};

    use super::*;

    fn vector_response(value: &str) -> Value {
        serde_json::json!({
            "status": "success",
            "data": {
                "resultType": "vector",
                "result": [
                    {
                        "metric": {},
                        "value": [1710000000.0, value]
                    }
                ]
            }
        })
    }

    fn scalar_response(value: &str) -> Value {
        serde_json::json!({
            "status": "success",
            "data": {
                "resultType": "scalar",
                "result": [1710000000.0, value]
            }
        })
    }

    fn sample_args(response_file: Option<std::path::PathBuf>) -> PrometheusImportArgs {
        PrometheusImportArgs {
            endpoint: None,
            response_file,
            rps_query: None,
            p50_query: None,
            p95_query: None,
            p99_query: None,
            header: Vec::new(),
            service_name: Some("checkout-api".to_string()),
            window: Some("5m".to_string()),
            observed_at: None,
            current_pool_size: 8,
            max_server_connections: 100,
            connection_overhead_ms: 2.0,
            idle_timeout_ms: Some(60_000),
            min: 2,
            max: 20,
            iterations: Some(1_500),
            seed: Some(11),
            distribution: Some(CliDistributionModel::Gamma),
            queue_model: Some(CliQueueModel::Mdc),
            target_wait_p99_ms: Some(50.0),
            max_acceptable_rho: Some(0.82),
        }
    }

    #[test]
    fn parses_vector_and_scalar_prometheus_values() {
        assert_eq!(
            parse_prometheus_value_from_json(&vector_response("123.5"))
                .expect("vector response should parse"),
            123.5
        );
        assert_eq!(
            parse_prometheus_value_from_json(&scalar_response("42"))
                .expect("scalar response should parse"),
            42.0
        );
    }

    #[test]
    fn rejects_invalid_prometheus_values() {
        let failed = serde_json::json!({"status": "error", "error": "bad query"});
        assert!(parse_prometheus_value_from_json(&failed)
            .expect_err("error status should fail")
            .to_string()
            .contains("bad query"));

        let empty = serde_json::json!({"status": "success", "data": {"resultType": "vector", "result": []}});
        assert!(parse_prometheus_value_from_json(&empty)
            .expect_err("empty vector should fail")
            .to_string()
            .contains("no series"));

        let multiple = serde_json::json!({
            "status": "success",
            "data": {
                "resultType": "vector",
                "result": [
                    {"metric": {"job": "a"}, "value": [1, "1"]},
                    {"metric": {"job": "b"}, "value": [1, "2"]}
                ]
            }
        });
        assert!(parse_prometheus_value_from_json(&multiple)
            .expect_err("multiple series should fail")
            .to_string()
            .contains("multiple series"));

        let unsupported = serde_json::json!({"status": "success", "data": {"resultType": "matrix", "result": []}});
        assert!(parse_prometheus_value_from_json(&unsupported)
            .expect_err("unsupported result type should fail")
            .to_string()
            .contains("unsupported"));
    }

    #[test]
    fn rejects_malformed_prometheus_payload_shapes() {
        let cases = [
            (
                serde_json::json!({"status": "success"}),
                "missing data object",
            ),
            (
                serde_json::json!({"status": "success", "data": {}}),
                "missing resultType",
            ),
            (
                serde_json::json!({"status": "success", "data": {"resultType": "scalar"}}),
                "missing scalar result",
            ),
            (
                serde_json::json!({"status": "success", "data": {"resultType": "vector"}}),
                "missing vector result array",
            ),
            (
                serde_json::json!({"status": "success", "data": {"resultType": "vector", "result": [{}]}}),
                "missing value",
            ),
            (
                serde_json::json!({"status": "success", "data": {"resultType": "scalar", "result": "bad"}}),
                "must be a [timestamp, value] array",
            ),
            (
                serde_json::json!({"status": "success", "data": {"resultType": "scalar", "result": [1]}}),
                "missing the sample value",
            ),
            (
                serde_json::json!({"status": "success", "data": {"resultType": "scalar", "result": [1, true]}}),
                "must be a string or number",
            ),
            (
                serde_json::json!({"status": "success", "data": {"resultType": "scalar", "result": [1, "NaN"]}}),
                "must be finite",
            ),
        ];

        for (payload, expected) in cases {
            assert!(
                parse_prometheus_value_from_json(&payload)
                    .expect_err("malformed payload should fail")
                    .to_string()
                    .contains(expected),
                "expected error containing {expected}"
            );
        }

        let number_payload = serde_json::json!({"status": "success", "data": {"resultType": "scalar", "result": [1, 7.5]}});
        assert_eq!(
            parse_prometheus_value_from_json(&number_payload)
                .expect("numeric samples should parse"),
            7.5
        );
    }

    #[test]
    fn reads_response_file_and_builds_valid_input() {
        let path = std::env::temp_dir().join(format!(
            "poolsim_prometheus_response_{}_{}.json",
            std::process::id(),
            1
        ));
        let content = serde_json::json!({
            "rps": vector_response("180"),
            "p50": vector_response("8"),
            "p95": vector_response("30"),
            "p99": vector_response("70")
        });
        fs::write(&path, content.to_string()).expect("response fixture should write");

        let input = resolve_prometheus_input(&sample_args(Some(path.clone())))
            .expect("prometheus response file should resolve");
        assert_eq!(input.snapshot.service_name.as_deref(), Some("checkout-api"));
        assert_eq!(input.snapshot.workload.requests_per_second, 180.0);
        assert_eq!(input.snapshot.workload.latency_p99_ms, 70.0);
        assert_eq!(input.options.seed, Some(11));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn query_values_uses_all_queries() {
        struct FakeClient;

        impl PrometheusClient for FakeClient {
            fn query(&self, query: &str) -> Result<String> {
                let value = match query {
                    "rps" => "100",
                    "p50" => "5",
                    "p95" => "20",
                    "p99" => "45",
                    _ => "0",
                };
                Ok(vector_response(value).to_string())
            }
        }

        let values = query_values(
            &FakeClient,
            &QuerySet {
                rps: "rps".to_string(),
                p50: "p50".to_string(),
                p95: "p95".to_string(),
                p99: "p99".to_string(),
            },
        )
        .expect("fake queries should parse");

        assert_eq!(values.rps, 100.0);
        assert_eq!(values.p99_ms, 45.0);
    }

    #[test]
    fn validates_endpoint_headers_and_encoding() {
        let endpoint = HttpEndpoint::parse("http://localhost:9090/prometheus/")
            .expect("endpoint should parse");
        assert_eq!(endpoint.host, "localhost");
        assert_eq!(endpoint.port, 9090);
        assert_eq!(
            endpoint.query_path("sum(rate(x[5m]))"),
            "/prometheus/api/v1/query?query=sum%28rate%28x%5B5m%5D%29%29"
        );

        let default_port =
            HttpEndpoint::parse("http://localhost").expect("default port should parse");
        assert_eq!(default_port.port, 80);

        assert_eq!(
            validate_header("Authorization: Bearer token").expect("valid header should pass"),
            "Authorization: Bearer token"
        );
        assert!(validate_header("bad").is_err());
        assert!(validate_header(" : token").is_err());
        assert!(validate_header("X:\nY").is_err());
        assert!(HttpEndpoint::parse("https://example.com").is_err());
        assert!(HttpEndpoint::parse("http://").is_err());
        assert!(HttpEndpoint::parse("http:///prometheus").is_err());
        assert!(HttpEndpoint::parse("http://:9090").is_err());
        assert!(HttpEndpoint::parse("http://localhost:notaport").is_err());
    }

    #[test]
    fn parses_plain_and_chunked_http_responses() {
        let body = vector_response("1").to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        assert_eq!(
            parse_http_response(&response).expect("plain response should parse"),
            body
        );

        let chunked = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nrust\r\n0\r\n\r\n";
        assert_eq!(
            parse_http_response(chunked).expect("chunked response should parse"),
            "rust"
        );

        assert!(parse_http_response("HTTP/1.1 500 nope\r\n\r\nbad")
            .expect_err("HTTP 500 should fail")
            .to_string()
            .contains("HTTP 500"));
        assert!(parse_http_response("not an http response").is_err());
        assert!(parse_http_response("\r\n\r\nbody").is_err());
        assert!(parse_http_response("HTTP/1.1\r\n\r\nbody").is_err());
        assert!(decode_chunked_body("bad").is_err());
        assert!(decode_chunked_body("4\r\nrs").is_err());
    }

    #[test]
    fn http_client_queries_local_server() {
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(error) if error.kind() == ErrorKind::PermissionDenied => return,
            Err(error) => panic!("listener should bind: {error}"),
        };
        let addr = listener
            .local_addr()
            .expect("listener addr should be available");
        let body = vector_response("12.5").to_string();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("client should connect");
            let mut buf = [0_u8; 1024];
            let n = stream.read(&mut buf).expect("request should read");
            let request = String::from_utf8_lossy(&buf[..n]);
            assert!(request.contains("GET /api/v1/query?query=up HTTP/1.1"));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("response should write");
        });

        let client = HttpPrometheusClient::new(&format!("http://{}", addr), &[])
            .expect("client should build");
        let raw = client.query("up").expect("query should succeed");
        assert_eq!(
            parse_prometheus_value(&raw).expect("raw Prometheus value should parse"),
            12.5
        );
        server.join().expect("server should finish");
    }

    #[test]
    fn http_client_reports_connection_failures_with_endpoint() {
        let endpoint = HttpEndpoint {
            host: "127.0.0.1".to_string(),
            port: 0,
            base_path: String::new(),
        };
        assert!(http_get(&endpoint, "/api/v1/query?query=up", &[])
            .expect_err("port 0 connection should fail")
            .to_string()
            .contains("failed to connect to Prometheus endpoint 127.0.0.1:0"));
    }

    #[test]
    fn resolve_prometheus_input_queries_live_endpoint() {
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(error) if error.kind() == ErrorKind::PermissionDenied => return,
            Err(error) => panic!("listener should bind: {error}"),
        };
        let addr = listener
            .local_addr()
            .expect("listener addr should be available");

        let server = thread::spawn(move || {
            for _ in 0..4 {
                let (mut stream, _) = listener.accept().expect("client should connect");
                let mut buf = [0_u8; 2048];
                let n = stream.read(&mut buf).expect("request should read");
                let request = String::from_utf8_lossy(&buf[..n]);
                assert!(request.contains("Authorization: Bearer token"));

                let value = if request.contains("query=rps") {
                    "180"
                } else if request.contains("query=p50") {
                    "8"
                } else if request.contains("query=p95") {
                    "30"
                } else if request.contains("query=p99") {
                    "70"
                } else {
                    "0"
                };
                let body = vector_response(value).to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("response should write");
            }
        });

        let mut args = sample_args(None);
        args.endpoint = Some(format!("http://{}", addr));
        args.rps_query = Some("rps".to_string());
        args.p50_query = Some("p50".to_string());
        args.p95_query = Some("p95".to_string());
        args.p99_query = Some("p99".to_string());
        args.header = vec!["Authorization: Bearer token".to_string()];

        let input = resolve_prometheus_input(&args).expect("live endpoint should resolve");
        assert_eq!(input.snapshot.workload.requests_per_second, 180.0);
        assert_eq!(input.snapshot.workload.latency_p50_ms, 8.0);
        assert_eq!(input.snapshot.workload.latency_p95_ms, 30.0);
        assert_eq!(input.snapshot.workload.latency_p99_ms, 70.0);
        server.join().expect("server should finish");
    }

    #[test]
    fn query_set_requires_all_queries() {
        let args = sample_args(None);
        assert!(query_set_from_args(&args).is_err());
        assert!(resolve_prometheus_input(&args)
            .expect_err("missing endpoint should fail")
            .to_string()
            .contains("missing --endpoint"));

        let mut args = sample_args(None);
        args.rps_query = Some("rps".to_string());
        args.p50_query = Some("p50".to_string());
        args.p95_query = Some("p95".to_string());
        args.p99_query = Some("p99".to_string());
        assert!(query_set_from_args(&args).is_ok());
    }
}
