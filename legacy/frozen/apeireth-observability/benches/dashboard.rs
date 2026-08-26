//! # apeireth-observability/benches/dashboard.rs (R21 — 1.0 release #7 perf 续, D-P3 续)
//!
//! 12 个 bench 测 9 器官 + 3 端点 + 整体 dashboard 渲染性能:
//! - 9 器官 widget 渲染 (heart/brain/hand/eye/ear/memory/voice/body/mind) — 9 函数
//! - 3 health 端点 (Health/Ready/Metrics) — 3 函数
//! - 整体 dashboard 渲染 (1 函数, 9+3+nav 集成)
//! - 9 器官批量注册 (concurrent 模拟, 1 函数)
//!
//! **基线** (1.0.0): target/criterion/apeireth-observability/dashboard/
//! **镜像基线**: 跟 sister 报告 §5.2 "R21+ 续 9+3+5+1+1 = 19 bench" 中核心 9+3+1 部分对齐.

use apeireth_observability::tui_dashboard::{
    render_organ_widget, OrganDashboard, OrganKind, OrganReadiness, TuiOrganState,
    ORGAN_KIND_COUNT,
};
use apeireth_observability::{
    health_check, render_health_response, render_prometheus, HealthEndpoint, HealthStatus,
    MetricKind, MetricSample,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// 9 器官 widget 渲染 (1 benchmark 含 9 sub-bench, per organ).
fn bench_9_organ_widget_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("9_organ_widget_render");
    // 给 9 器官不同 value 模拟真接数据
    let samples: Vec<(OrganKind, TuiOrganState)> = (0..ORGAN_KIND_COUNT)
        .map(|i| {
            let organ = OrganKind::from_u8(i as u8).unwrap();
            let state = TuiOrganState::ok(organ, i as f64 * 10.0, "ok 真接数据 (R21 bench)");
            (organ, state)
        })
        .collect();
    for (organ, state) in &samples {
        group.bench_with_input(
            BenchmarkId::from_parameter(organ.as_str()),
            organ,
            |b, organ| {
                b.iter(|| {
                    let _ = render_organ_widget(black_box(*organ), black_box(state));
                });
            },
        );
    }
    group.finish();
}

/// 3 端点 health check (async, 1 benchmark 含 3 sub-bench).
fn bench_3_health_endpoint(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut group = c.benchmark_group("3_health_endpoint");
    let endpoints = [
        HealthEndpoint::Health,
        HealthEndpoint::Ready,
        HealthEndpoint::Metrics,
    ];
    for ep in &endpoints {
        group.bench_with_input(BenchmarkId::from_parameter(ep.as_path()), ep, |b, ep| {
            b.iter(|| {
                rt.block_on(async {
                    let _ = health_check(
                        black_box(*ep),
                        black_box(HealthStatus::Healthy),
                        &[],
                    )
                    .await;
                });
            });
        });
    }
    group.finish();
}

/// 3 端点 health response JSON 渲染 (sync, 1 benchmark 含 3 sub-bench).
fn bench_3_health_response_render(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut group = c.benchmark_group("3_health_response_render");
    let endpoints = [
        HealthEndpoint::Health,
        HealthEndpoint::Ready,
        HealthEndpoint::Metrics,
    ];
    for ep in &endpoints {
        group.bench_with_input(BenchmarkId::from_parameter(ep.as_path()), ep, |b, ep| {
            b.iter(|| {
                rt.block_on(async {
                    let resp =
                        health_check(*ep, HealthStatus::Healthy, &[]).await;
                    let _ = render_health_response(black_box(&resp)).unwrap();
                });
            });
        });
    }
    group.finish();
}

/// Prometheus /metrics exposition format 渲染 (sync, 10 sample).
fn bench_prometheus_metrics_render(c: &mut Criterion) {
    let samples: Vec<MetricSample> = (0..10)
        .map(|i| {
            MetricSample::new(
                format!("http_requests_total_{i}"),
                MetricKind::Counter,
                i as f64 * 100.0,
            )
            .with_label("endpoint", "/v1/chat")
        })
        .collect();
    c.bench_function("prometheus_metrics_render_10_samples", |b| {
        b.iter(|| {
            let _ = render_prometheus(black_box(&samples));
        });
    });
}

/// 整体 dashboard 渲染 (1 函数, 9 organ + 3 endpoint + 1 nav).
fn bench_render_dashboard(c: &mut Criterion) {
    let dashboard = OrganDashboard::new();
    c.bench_function("render_dashboard", |b| {
        b.iter(|| {
            let _ = apeireth_observability::tui_dashboard::render_dashboard(black_box(&dashboard));
        });
    });
}

/// 9 器官批量注册 (concurrent 模拟, 1 函数).
fn bench_9_organ_register(c: &mut Criterion) {
    let dashboard = OrganDashboard::new();
    let states: Vec<TuiOrganState> = (0..ORGAN_KIND_COUNT)
        .map(|i| {
            let organ = OrganKind::from_u8(i as u8).unwrap();
            TuiOrganState::ok(organ, 42.0, format!("bench #{} ok 真接", i))
        })
        .collect();
    c.bench_function("9_organ_register", |b| {
        b.iter(|| {
            for (i, state) in states.iter().enumerate() {
                let organ = OrganKind::from_u8(i as u8).unwrap();
                let _ = dashboard.register_tui_organ_state(black_box(organ), black_box(state.clone()));
            }
        });
    });
    let _ = Throughput::Elements(ORGAN_KIND_COUNT as u64); // 标 9 elements
}

/// 9 器官读全部状态 (1 函数).
fn bench_9_organ_read_all(c: &mut Criterion) {
    let dashboard = OrganDashboard::new();
    c.bench_function("9_organ_read_all", |b| {
        b.iter(|| {
            let _ = black_box(&dashboard).read_all_organ_states();
        });
    });
}

criterion_group!(
    dashboard_benches,
    bench_9_organ_widget_render,
    bench_3_health_endpoint,
    bench_3_health_response_render,
    bench_prometheus_metrics_render,
    bench_render_dashboard,
    bench_9_organ_register,
    bench_9_organ_read_all
);
criterion_main!(dashboard_benches);
