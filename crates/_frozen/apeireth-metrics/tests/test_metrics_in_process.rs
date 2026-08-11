//! # apeireth-metrics 集成测试 (30+ 测试, per task spec §6)
//!
//! 覆盖 4 metric 类型 + 5 exporter + registry + K-1 label 守门 + 并发安全 + Prometheus 格式校验.

use std::collections::HashMap;
use std::sync::Arc;

use apeireth_metrics::*;

// ============================================================================
// §1 4 Metric 类型 (per task spec §6.1)
// ============================================================================

#[test]
fn test_metric_4_kinds() {
    let c = Counter::new("c", "h", HashMap::new()).unwrap();
    let g = Gauge::new("g", "h", HashMap::new()).unwrap();
    let h = Histogram::new("h", "h", HashMap::new()).unwrap();
    let s = Summary::new("s", "h", HashMap::new()).unwrap();

    // 4 类型都实现 Metric trait
    let _m1: &dyn Metric = &c;
    let _m2: &dyn Metric = &g;
    let _m3: &dyn Metric = &h;
    let _m4: &dyn Metric = &s;

    // type_name 正确
    assert_eq!(c.type_name(), "counter");
    assert_eq!(g.type_name(), "gauge");
    assert_eq!(h.type_name(), "histogram");
    assert_eq!(s.type_name(), "summary");
}

// ============================================================================
// §2 Counter
// ============================================================================

#[test]
fn test_counter_increment() {
    let c = Counter::new("c", "h", HashMap::new()).unwrap();
    c.inc();
    c.inc();
    c.inc();
    assert_eq!(c.get(), 3);
    c.inc_by(10);
    assert_eq!(c.get(), 13);
}

#[test]
fn test_counter_reset_zero_only() {
    let c = Counter::new("c", "h", HashMap::new()).unwrap();
    c.inc_by(100);
    assert_eq!(c.get(), 100);
    c.reset();
    assert_eq!(c.get(), 0);
    // reset 后再 inc 仍 OK
    c.inc();
    assert_eq!(c.get(), 1);
}

// ============================================================================
// §3 Gauge
// ============================================================================

#[test]
fn test_gauge_inc_dec() {
    let g = Gauge::new("g", "h", HashMap::new()).unwrap();
    g.inc();
    g.inc();
    g.dec();
    assert_eq!(g.get(), 1.0);
    g.dec();
    g.dec();
    assert_eq!(g.get(), -1.0);
}

#[test]
fn test_gauge_set() {
    let g = Gauge::new("g", "h", HashMap::new()).unwrap();
    g.set(42.5);
    assert_eq!(g.get(), 42.5);
    g.set(-100.0);
    assert_eq!(g.get(), -100.0);
    g.set(0.0);
    assert_eq!(g.get(), 0.0);
}

// ============================================================================
// §4 Histogram
// ============================================================================

#[test]
fn test_histogram_observe_bucket() {
    let h = Histogram::new("h", "h", HashMap::new()).unwrap();
    h.observe(0.001);
    h.observe(0.5);
    h.observe(15.0);
    let counts = h.bucket_counts();
    assert_eq!(counts[0], 1); // 0.005 bucket
    assert_eq!(counts[6], 1); // 0.5 bucket
    // 15.0 不入任何 bucket
    assert_eq!(h.count(), 3);
}

#[test]
fn test_histogram_quantile() {
    let h = Histogram::new("h", "h", HashMap::new()).unwrap();
    for i in 1..=100 {
        h.observe(i as f64 * 0.01);
    }
    let p50 = h.quantile(0.5);
    let p99 = h.quantile(0.99);
    // p50 ~ 0.5, p99 ~ 0.99
    assert!(p50 > 0.4 && p50 < 0.6, "p50 = {p50}");
    assert!(p99 > 0.9 && p99 < 1.5, "p99 = {p99}");
}

// ============================================================================
// §5 Summary
// ============================================================================

#[test]
fn test_summary_quantile_p50() {
    let s = Summary::new("s", "h", HashMap::new()).unwrap();
    for i in 1..=100 {
        s.observe(i as f64 * 0.01);
    }
    let qv = s.quantile_values();
    let p50 = qv.iter().find(|(q, _)| (*q - 0.5).abs() < 1e-9).unwrap().1;
    // p50 ~ 0.5
    assert!(p50 > 0.4 && p50 < 0.6, "p50 = {p50}");
}

#[test]
fn test_summary_quantile_p99() {
    let s = Summary::new("s", "h", HashMap::new()).unwrap();
    for i in 1..=1000 {
        s.observe(i as f64 * 0.001);
    }
    let qv = s.quantile_values();
    let p99 = qv.iter().find(|(q, _)| (*q - 0.99).abs() < 1e-9).unwrap().1;
    // p99 ~ 0.99
    assert!(p99 > 0.9 && p99 < 1.1, "p99 = {p99}");
}

// ============================================================================
// §6 Label K-1 强校验
// ============================================================================

#[test]
fn test_label_key_value() {
    let l = Label::new("method", "GET").unwrap();
    assert_eq!(l.key(), "method");
    assert_eq!(l.value(), "GET");
}

#[test]
fn test_label_max_10() {
    // 10 个 label 通过
    let mut m = HashMap::new();
    for i in 0..10 {
        m.insert(format!("k{i}"), format!("v{i}"));
    }
    assert!(validate_labels(&m).is_ok());
    // 第 11 个拒
    m.insert("k10".to_string(), "v10".to_string());
    assert!(matches!(
        validate_labels(&m),
        Err(MetricsError::TooManyLabels { actual: 11 })
    ));
}

#[test]
fn test_label_key_invalid_chars() {
    // 必须 [a-zA-Z_][a-zA-Z0-9_]*
    assert!(validate_label_key("1method").is_err());
    assert!(validate_label_key("method-name").is_err());
    assert!(validate_label_key("foo:bar").is_err()); // label key 不允许 :
    assert!(validate_label_key("foo bar").is_err());
    assert!(validate_label_key("").is_err());
    // 合法
    assert!(validate_label_key("method").is_ok());
    assert!(validate_label_key("status_code").is_ok());
    assert!(validate_label_key("_internal").is_ok());
    assert!(validate_label_key("A1").is_ok());
}

// ============================================================================
// §7 5 Exporter
// ============================================================================

#[tokio::test]
async fn test_exporter_5_kinds() {
    let r = MetricsRegistry::new();
    r.register_counter(Arc::new(
        Counter::new("c", "h", HashMap::new()).unwrap(),
    ))
    .unwrap();

    // 1. Prometheus (完整)
    let p = PrometheusExporter::new("apeireth", "agent");
    let body = p.export(&r).await.unwrap();
    assert!(body.contains("apeireth_agent_c"));

    // 2. Pushgateway (stub)
    let pg = PushgatewayExporter::new("http://localhost:9091", "job");
    assert!(matches!(
        pg.export(&r).await,
        Err(MetricsError::ExporterNotImplemented(_))
    ));

    // 3. OTLP (stub)
    let otlp = OtlpExporter::new("http://localhost:4317", "svc");
    assert!(matches!(
        otlp.export(&r).await,
        Err(MetricsError::ExporterNotImplemented(_))
    ));

    // 4. StatsD (stub)
    let sd = StatsdExporter::new("127.0.0.1", 8125);
    assert!(matches!(
        sd.export(&r).await,
        Err(MetricsError::ExporterNotImplemented(_))
    ));

    // 5. Stdout (完整)
    let so = StdoutExporter::new();
    let body = so.export(&r).await.unwrap();
    assert!(body.contains("apeireth_agent_c"));
}

#[tokio::test]
async fn test_prometheus_format_parseable() {
    let r = MetricsRegistry::new();
    let c = Arc::new(Counter::new("requests_total", "Total", HashMap::new()).unwrap());
    c.inc_by(42);
    r.register_counter(c).unwrap();
    let g = Arc::new(Gauge::new("memory_bytes", "Memory", HashMap::new()).unwrap());
    g.set(1024.0);
    r.register_gauge(g).unwrap();

    let p = PrometheusExporter::new("apeireth", "agent");
    let body = p.export(&r).await.unwrap();

    // 验证格式: 每行 "# HELP name help" 或 "# TYPE name type" 或 "name value"
    let mut help_count = 0;
    let mut type_count = 0;
    let mut data_count = 0;
    for line in body.lines() {
        if line.starts_with("# HELP ") {
            help_count += 1;
        } else if line.starts_with("# TYPE ") {
            type_count += 1;
        } else if !line.is_empty() {
            data_count += 1;
            // data 行格式: name <value> 或 name{label="val"} <value>
            let has_value = line.contains(' ');
            assert!(has_value, "data line malformed: {line}");
        }
    }
    assert_eq!(help_count, 2, "should have 2 HELP lines, body={body}");
    assert_eq!(type_count, 2, "should have 2 TYPE lines, body={body}");
    assert!(data_count >= 2, "should have ≥2 data lines, body={body}");

    // 包含具体值
    assert!(body.contains("apeireth_agent_requests_total 42"));
    assert!(body.contains("apeireth_agent_memory_bytes 1024"));
}

// ============================================================================
// §8 K-1 metric name / help / label 强校验 (per task spec §6 K-1)
// ============================================================================

#[test]
fn test_k1_metric_name_invalid() {
    // 必须 [a-zA-Z_:][a-zA-Z0-9_:]*
    assert!(validate_metric_name("1abc").is_err());
    assert!(validate_metric_name("a-b").is_err());
    assert!(validate_metric_name("a b").is_err());
    // 合法
    assert!(validate_metric_name("abc").is_ok());
    assert!(validate_metric_name("_a").is_ok());
    assert!(validate_metric_name("a:b:c").is_ok());
    assert!(validate_metric_name("A1").is_ok());
}

#[test]
fn test_k1_metric_name_empty() {
    assert!(matches!(
        validate_metric_name(""),
        Err(MetricsError::MetricNameEmpty)
    ));
    // Counter 构造也拒
    assert!(Counter::new("", "h", HashMap::new()).is_err());
    assert!(Gauge::new("", "h", HashMap::new()).is_err());
    assert!(Histogram::new("", "h", HashMap::new()).is_err());
    assert!(Summary::new("", "h", HashMap::new()).is_err());
}

#[test]
fn test_k1_help_required() {
    assert!(Counter::new("c", "", HashMap::new()).is_err());
    assert!(Gauge::new("g", "", HashMap::new()).is_err());
    assert!(Histogram::new("h", "", HashMap::new()).is_err());
    assert!(Summary::new("s", "", HashMap::new()).is_err());
}

#[test]
fn test_k1_label_value_too_long() {
    let long = "a".repeat(257);
    // Label::new 拒
    assert!(Label::new("k", &long).is_err());
    // validate_label_value 拒
    assert!(matches!(
        validate_label_value("k", &long),
        Err(MetricsError::LabelValueTooLong { actual: 257, .. })
    ));
    // 256 字符通过
    let exact = "a".repeat(256);
    assert!(Label::new("k", &exact).is_ok());
}

// ============================================================================
// §9 3 Stub Exporter 各自明确
// ============================================================================

#[tokio::test]
async fn test_pushgateway_exporter_not_implemented() {
    let r = MetricsRegistry::new();
    let pg = PushgatewayExporter::new("http://localhost:9091", "job");
    let err = pg.export(&r).await.unwrap_err();
    assert!(matches!(err, MetricsError::ExporterNotImplemented(_)));
    if let MetricsError::ExporterNotImplemented(s) = err {
        assert!(s.contains("PUSHGATEWAY"));
    }
}

#[tokio::test]
async fn test_otlp_exporter_not_implemented() {
    let r = MetricsRegistry::new();
    let otlp = OtlpExporter::new("http://localhost:4317", "svc");
    let err = otlp.export(&r).await.unwrap_err();
    assert!(matches!(err, MetricsError::ExporterNotImplemented(_)));
    if let MetricsError::ExporterNotImplemented(s) = err {
        assert!(s.contains("OTLP"));
    }
}

#[tokio::test]
async fn test_statsd_exporter_not_implemented() {
    let r = MetricsRegistry::new();
    let sd = StatsdExporter::new("127.0.0.1", 8125);
    let err = sd.export(&r).await.unwrap_err();
    assert!(matches!(err, MetricsError::ExporterNotImplemented(_)));
    if let MetricsError::ExporterNotImplemented(s) = err {
        assert!(s.contains("STATSD"));
    }
}

#[tokio::test]
async fn test_stdout_exporter_write() {
    let r = MetricsRegistry::new();
    r.register_counter(Arc::new(
        Counter::new("c", "h", HashMap::new()).unwrap(),
    ))
    .unwrap();
    let so = StdoutExporter::new();
    let body = so.export(&r).await.unwrap();
    assert!(body.contains("c"));
    assert!(body.contains("counter"));
}

// ============================================================================
// §10 Registry 操作
// ============================================================================

#[test]
fn test_registry_register_duplicate_name() {
    let r = MetricsRegistry::new();
    r.register_counter(Arc::new(
        Counter::new("c", "h", HashMap::new()).unwrap(),
    ))
    .unwrap();
    assert!(matches!(
        r.register_counter(Arc::new(
            Counter::new("c", "h", HashMap::new()).unwrap()
        )),
        Err(MetricsError::MetricAlreadyRegistered(_))
    ));
}

#[test]
fn test_registry_unregister() {
    let r = MetricsRegistry::new();
    r.register_counter(Arc::new(
        Counter::new("c", "h", HashMap::new()).unwrap(),
    ))
    .unwrap();
    assert_eq!(r.len(), 1);
    let removed = r.unregister("c").unwrap();
    assert_eq!(removed.name(), "c");
    assert_eq!(r.len(), 0);
    // 重复 unregister 返 Err
    assert!(matches!(
        r.unregister("c"),
        Err(MetricsError::MetricNotFound(_))
    ));
}

#[test]
fn test_registry_get_returns_metric() {
    let r = MetricsRegistry::new();
    r.register_gauge(Arc::new(Gauge::new("g", "h", HashMap::new()).unwrap()))
        .unwrap();
    let m = r.get("g").unwrap();
    assert_eq!(m.name(), "g");
    assert_eq!(m.type_name(), "gauge");
    // 不存在 → None
    assert!(r.get("nope").is_none());
}

#[test]
fn test_registry_list_all_metrics() {
    let r = MetricsRegistry::new();
    r.register_counter(Arc::new(
        Counter::new("a", "h", HashMap::new()).unwrap(),
    ))
    .unwrap();
    r.register_gauge(Arc::new(Gauge::new("b", "h", HashMap::new()).unwrap()))
        .unwrap();
    r.register_histogram(Arc::new(
        Histogram::new("c", "h", HashMap::new()).unwrap(),
    ))
    .unwrap();
    r.register_summary(Arc::new(
        Summary::new("d", "h", HashMap::new()).unwrap(),
    ))
    .unwrap();
    let list = r.list();
    assert_eq!(list.len(), 4);
    // 按 name 排序
    assert_eq!(list[0].name(), "a");
    assert_eq!(list[3].name(), "d");
}

// ============================================================================
// §11 并发安全
// ============================================================================

#[test]
fn test_concurrent_increment_1000_tasks() {
    use std::thread;
    let c = Arc::new(Counter::new("c", "h", HashMap::new()).unwrap());
    let g = Arc::new(Gauge::new("g", "h", HashMap::new()).unwrap());
    let h = Arc::new(Histogram::new("h", "h", HashMap::new()).unwrap());
    let s = Arc::new(Summary::new("s", "h", HashMap::new()).unwrap());

    let mut handles = vec![];
    for i in 0..1000 {
        let cc = Arc::clone(&c);
        let gg = Arc::clone(&g);
        let hh = Arc::clone(&h);
        let ss = Arc::clone(&s);
        handles.push(thread::spawn(move || {
            cc.inc();
            gg.add(0.1);
            hh.observe(0.05);
            ss.observe(0.1);
            // 防止优化
            let _ = i;
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(c.get(), 1000);
    assert!((g.get() - 100.0).abs() < 1e-9, "g.get() = {}", g.get());
    assert_eq!(h.count(), 1000);
    assert_eq!(s.count(), 1000);
}

// ============================================================================
// §12 默认值守门
// ============================================================================

#[test]
fn test_histogram_default_buckets() {
    // 11 默认 buckets: [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]
    assert_eq!(DEFAULT_BUCKETS.len(), 11);
    assert_eq!(DEFAULT_BUCKETS[0], 0.005);
    assert_eq!(DEFAULT_BUCKETS[5], 0.25);
    assert_eq!(DEFAULT_BUCKETS[10], 10.0);

    // 默认 buckets 升序
    for w in DEFAULT_BUCKETS.windows(2) {
        assert!(w[0] < w[1]);
    }
}

#[test]
fn test_summary_default_quantiles() {
    // 5 默认 quantiles: [0.5, 0.9, 0.95, 0.99, 0.999]
    assert_eq!(DEFAULT_QUANTILES.len(), 5);
    assert_eq!(DEFAULT_QUANTILES[0], 0.5);
    assert_eq!(DEFAULT_QUANTILES[1], 0.9);
    assert_eq!(DEFAULT_QUANTILES[2], 0.95);
    assert_eq!(DEFAULT_QUANTILES[3], 0.99);
    assert_eq!(DEFAULT_QUANTILES[4], 0.999);
}

// ============================================================================
// §13 端到端 (registry → exporter → prom format 校验)
// ============================================================================

#[tokio::test]
async fn test_e2e_full_pipeline() {
    let r = MetricsRegistry::new();

    // 注册 10 个 metric (per task spec §7)
    for i in 0..5 {
        let c = Arc::new(
            Counter::new(
                format!("requests_total_{i}"),
                "Total requests",
                HashMap::new(),
            )
            .unwrap(),
        );
        c.inc_by(i as u64);
        r.register_counter(c).unwrap();
    }
    for i in 0..3 {
        let g = Arc::new(
            Gauge::new(
                format!("memory_bytes_{i}"),
                "Memory used",
                HashMap::new(),
            )
            .unwrap(),
        );
        g.set((i as f64) * 1024.0);
        r.register_gauge(g).unwrap();
    }
    for i in 0..2 {
        let h = Arc::new(
            Histogram::new(
                format!("latency_{i}"),
                "Request latency",
                HashMap::new(),
            )
            .unwrap(),
        );
        h.observe(0.05);
        r.register_histogram(h).unwrap();
    }

    // Prometheus 导出
    let p = PrometheusExporter::new("apeireth", "agent");
    let body = p.export(&r).await.unwrap();

    // 验证 10 个 metric 都出现
    for i in 0..5 {
        assert!(
            body.contains(&format!("apeireth_agent_requests_total_{i}")),
            "missing requests_total_{i}"
        );
    }
    for i in 0..3 {
        assert!(
            body.contains(&format!("apeireth_agent_memory_bytes_{i}")),
            "missing memory_bytes_{i}"
        );
    }
    for i in 0..2 {
        assert!(
            body.contains(&format!("apeireth_agent_latency_{i}")),
            "missing latency_{i}"
        );
    }
}

#[tokio::test]
async fn test_e2e_with_labels() {
    let r = MetricsRegistry::new();

    let mut labels = HashMap::new();
    labels.insert("method".to_string(), "GET".to_string());
    labels.insert("status".to_string(), "200".to_string());

    let c = Arc::new(
        Counter::new("http_requests_total", "HTTP requests", labels).unwrap(),
    );
    c.inc_by(42);
    r.register_counter(c).unwrap();

    let p = PrometheusExporter::new("apeireth", "agent");
    let body = p.export(&r).await.unwrap();

    // 包含 label
    assert!(body.contains("method=\"GET\""));
    assert!(body.contains("status=\"200\""));
    assert!(body.contains("apeireth_agent_http_requests_total"));
}
