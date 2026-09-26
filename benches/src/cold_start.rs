//! 微内核冷启动基准: 生产组装到 Runtime ready 的单次冷启动耗时。
//!
//! 口径: 计时区间 = SQLite 记忆/会话存储打开 + `ProductionModules::build`
//! 模块组装 + `register_into(Runtime::builder())` + `build().await` 到 ready
//! (详见 [`crate::assembly`] 模块文档的组装清单与不含项)。不计入: tokio 运行时
//! 创建 (提前建好)、临时数据目录创建、Runtime 销毁。每轮使用全新数据目录
//! (真冷路径: 无热缓存、无共享连接池)。

use std::time::Instant;

use crate::assembly::{assemble_production, tokio_runtime};
use crate::support::{cleanup_dir, temp_dir, BenchConfig, Outcome, Target};

/// 运行微内核冷启动基准。
pub fn run(cfg: &BenchConfig) -> Vec<Outcome> {
    let rt = tokio_runtime();
    let samples = cfg.samples(30);
    let warmup = cfg.samples(2);

    for _ in 0..warmup {
        let dir = temp_dir("cold-warm");
        let runtime = rt.block_on(assemble_production(&dir));
        drop(runtime);
        cleanup_dir(&dir);
    }

    let mut samples_ns = Vec::with_capacity(samples);
    for _ in 0..samples {
        let dir = temp_dir("cold");
        let start = Instant::now();
        let runtime = rt.block_on(assemble_production(&dir));
        samples_ns.push(start.elapsed().as_nanos() as f64);
        drop(runtime);
        cleanup_dir(&dir);
    }

    vec![Outcome::from_ns(
        "cold-start",
        "微内核冷启动 (Runtime 组装到 ready)",
        "SQLite 存储打开 + ProductionModules 模块组装 + Runtime build().await 到 ready 单次",
        "ms",
        1,
        warmup,
        samples_ns,
        Some(Target {
            label: "< 10.0 ms".to_string(),
            value: 10.0,
        }),
        "每轮全新数据目录 (真冷); 不含 tokio 运行时创建/临时目录创建/Runtime 销毁; 组装清单与不含项见 harness assembly 模块文档",
    )]
}
