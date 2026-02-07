use std::fmt::Write;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

/// Generate a realistic JSON log line of approximately the given size.
///
/// Produces lines resembling real structured-logging output from frameworks
/// like logrus, zap, slog, pino, etc.
fn generate_log_line(variant: usize) -> String {
    match variant % 6 {
        0 => {
            // logrus-style (~220 bytes)
            r#"{"time":"2026-01-15T10:30:00.123Z","level":"info","msg":"request completed","method":"GET","path":"/api/v1/users","status":200,"latency_ms":42,"user_id":"usr_abc123","request_id":"req_xyz789"}"#.to_string()
        }
        1 => {
            // zap-style with nested object (~300 bytes)
            r#"{"ts":1768473000.123,"level":"debug","caller":"server/handler.go:42","msg":"processing request","http":{"method":"POST","url":"/api/v1/orders","status":201},"user":"john@example.com","duration":"15.2ms","trace_id":"abc123def456"}"#.to_string()
        }
        2 => {
            // slog-style (~250 bytes)
            r#"{"time":"2026-01-15T10:30:01.456Z","level":"WARN","msg":"high memory usage detected","source":"monitor","component":"health-checker","memory_mb":1842,"threshold_mb":1500,"hostname":"prod-web-03"}"#.to_string()
        }
        3 => {
            // pino-style with numeric level (~280 bytes)
            r#"{"level":30,"time":1768473000456,"pid":12345,"hostname":"api-server-01","msg":"database query executed","query":"SELECT * FROM users WHERE active = true","duration_ms":23,"rows_returned":150,"connection_pool":"primary"}"#.to_string()
        }
        4 => {
            // bunyan-style (~320 bytes)
            r#"{"v":0,"name":"myapp","hostname":"prod-01","pid":9876,"level":50,"msg":"connection pool exhausted","time":"2026-01-15T10:30:02.789Z","src":{"file":"db/pool.rs","line":142},"pool_size":20,"active_connections":20,"waiting_requests":15}"#.to_string()
        }
        _ => {
            // structlog-style (~350 bytes)
            r#"{"event":"payment processed","level":"info","timestamp":"2026-01-15T10:30:03.012Z","logger":"payments.processor","amount":99.99,"currency":"USD","customer_id":"cust_12345","payment_method":"card","transaction_id":"txn_abcdef123456","processing_time_ms":234}"#.to_string()
        }
    }
}

/// Generate a batch of log lines as a single string (newline-delimited).
fn generate_log_batch(count: usize) -> Vec<String> {
    (0..count).map(generate_log_line).collect()
}

fn bench_parse_and_format(c: &mut Criterion) {
    let config = cor::Config::default();
    let lines = generate_log_batch(1000);

    let mut group = c.benchmark_group("throughput");
    group.throughput(Throughput::Elements(lines.len() as u64));

    group.bench_function("parse_and_format_1k_lines", |b| {
        let mut out = String::with_capacity(512);
        b.iter(|| {
            for line in &lines {
                out.clear();
                cor::format_line(criterion::black_box(line), &config, false, &mut out);
                criterion::black_box(&out);
            }
        });
    });

    group.finish();
}

fn bench_parse_only(c: &mut Criterion) {
    let config = cor::Config::default();
    let lines = generate_log_batch(1000);

    let mut group = c.benchmark_group("parse");
    group.throughput(Throughput::Elements(lines.len() as u64));

    group.bench_function("parse_1k_lines", |b| {
        b.iter(|| {
            for line in &lines {
                let _ = cor::parse_line(criterion::black_box(line), &config);
            }
        });
    });

    group.finish();
}

fn bench_format_mixed_input(c: &mut Criterion) {
    let config = cor::Config::default();

    // Mix of JSON and non-JSON lines (realistic workload)
    let mut lines: Vec<String> = Vec::with_capacity(1000);
    for i in 0..1000 {
        if i % 10 == 0 {
            // 10% non-JSON lines
            lines.push(format!(
                "plain text log line number {i} with some extra content"
            ));
        } else {
            lines.push(generate_log_line(i));
        }
    }

    let mut group = c.benchmark_group("mixed_input");
    group.throughput(Throughput::Elements(lines.len() as u64));

    group.bench_function("mixed_1k_lines", |b| {
        let mut out = String::with_capacity(512);
        b.iter(|| {
            for line in &lines {
                out.clear();
                cor::format_line(criterion::black_box(line), &config, false, &mut out);
                criterion::black_box(&out);
            }
        });
    });

    group.finish();
}

fn bench_line_sizes(c: &mut Criterion) {
    let config = cor::Config::default();

    let mut group = c.benchmark_group("line_size");

    for size_label in &["small_200b", "medium_500b", "large_1kb"] {
        let line = match *size_label {
            "small_200b" => {
                r#"{"level":"info","msg":"ok","ts":"2026-01-15T10:30:00Z","port":8080}"#.to_string()
            }
            "medium_500b" => {
                let mut s = r#"{"level":"debug","msg":"request details","ts":"2026-01-15T10:30:00Z","method":"POST","path":"/api/v1/orders","status":201,"user":"john@example.com","trace":"abc123","span":"def456""#.to_string();
                for i in 0..10 {
                    write!(s, r#","field_{i}":"value_{i}_padding_data""#).unwrap();
                }
                s.push('}');
                s
            }
            _ => {
                let mut s =
                    r#"{"level":"warn","msg":"large payload detected","ts":"2026-01-15T10:30:00Z""#
                        .to_string();
                for i in 0..40 {
                    write!(s, r#","field_{i}":"value_with_extra_content_{i}""#).unwrap();
                }
                s.push('}');
                s
            }
        };

        group.throughput(Throughput::Bytes(line.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size_label), &line, |b, line| {
            let mut out = String::with_capacity(line.len() * 2);
            b.iter(|| {
                out.clear();
                cor::format_line(criterion::black_box(line), &config, false, &mut out);
                criterion::black_box(&out);
            });
        });
    }

    group.finish();
}

fn bench_level_filtering(c: &mut Criterion) {
    let config = cor::Config {
        min_level: Some(cor::Level::Warn),
        ..cor::Config::default()
    };
    let lines = generate_log_batch(1000);

    let mut group = c.benchmark_group("level_filter");
    group.throughput(Throughput::Elements(lines.len() as u64));

    group.bench_function("filter_1k_lines", |b| {
        let mut out = String::with_capacity(512);
        b.iter(|| {
            for line in &lines {
                out.clear();
                cor::format_line(criterion::black_box(line), &config, false, &mut out);
                criterion::black_box(&out);
            }
        });
    });

    group.finish();
}

fn bench_embedded_json(c: &mut Criterion) {
    let config = cor::Config::default();

    let lines: Vec<String> = (0..1000)
        .map(|i| {
            format!(
                "2026-01-15 10:30:{:02}.{:03} {{\"level\":\"info\",\"msg\":\"embedded line {i}\",\"counter\":{i}}}",
                i % 60,
                i % 1000
            )
        })
        .collect();

    let mut group = c.benchmark_group("embedded_json");
    group.throughput(Throughput::Elements(lines.len() as u64));

    group.bench_function("embedded_1k_lines", |b| {
        let mut out = String::with_capacity(512);
        b.iter(|| {
            for line in &lines {
                out.clear();
                cor::format_line(criterion::black_box(line), &config, false, &mut out);
                criterion::black_box(&out);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_and_format,
    bench_parse_only,
    bench_format_mixed_input,
    bench_line_sizes,
    bench_level_filtering,
    bench_embedded_json,
);
criterion_main!(benches);
