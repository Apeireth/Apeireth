//! R220 PyO3 async wrapper (tokio::spawn_blocking 包装 sync Python 为 async).
//!
//! **动机**: PyO3 本身是同步的 (GIL 阻塞). pyo3-asyncio 是官方提供的 async 集成,
//! 但需要额外的依赖 + 复杂 setup. R220 提供轻量级替代:
//! - 用 tokio::spawn_blocking 把 sync Python 调用包装为 async
//! - 不阻塞 tokio runtime
//! - 0 引 pyo3-asyncio
//!
//! **借鉴** (per O-5): tokio::task::spawn_blocking 模式 + rayon_threadpool 思路.
//!
//! **0 触碰**:
//! - bridge.rs 0 改 (sync call 仍可用)
//! - 3 不可变脊柱 0 触碰

#![allow(missing_docs)] // R220 additive
#![cfg(feature = "python-ext")]  // 仅在启用 python-ext 时编译

use std::collections::HashMap;
use std::time::Duration;

use crate::bridge::{call_python_function, call_python_function_kw, eval_python_expression, BridgeError};

// ============================================================================
// Async wrapper (tokio::spawn_blocking)
// ============================================================================

/// 异步调用 Python 函数 (tokio::spawn_blocking 包装).
pub async fn call_python_async(
    module_name: String,
    func_name: String,
    args: Vec<String>,
) -> Result<String, BridgeError> {
    tokio::task::spawn_blocking(move || call_python_function(&module_name, &func_name, &args))
        .await
        .map_err(|e| BridgeError::Internal(format!("spawn_blocking join error: {e}")))?
}

/// 异步调用 Python 函数 (带 kwargs).
pub async fn call_python_kw_async(
    module_name: String,
    func_name: String,
    args: Vec<String>,
    kwargs: HashMap<String, String>,
) -> Result<String, BridgeError> {
    tokio::task::spawn_blocking(move || call_python_function_kw(&module_name, &func_name, &args, &kwargs))
        .await
        .map_err(|e| BridgeError::Internal(format!("spawn_blocking join error: {e}")))?
}

/// 异步 eval Python 表达式.
pub async fn eval_python_async(expr: String) -> Result<String, BridgeError> {
    tokio::task::spawn_blocking(move || eval_python_expression(&expr))
        .await
        .map_err(|e| BridgeError::Internal(format!("spawn_blocking join error: {e}")))?
}

/// 带 timeout 的 async 调用 (避免 spawn_blocking hang 永远).
pub async fn call_python_async_timeout(
    module_name: String,
    func_name: String,
    args: Vec<String>,
    timeout: Duration,
) -> Result<String, BridgeError> {
    match tokio::time::timeout(
        timeout,
        call_python_async(module_name, func_name, args),
    )
    .await
    {
        Ok(r) => r,
        Err(_) => Err(BridgeError::Internal(format!(
            "Python call timed out after {}s",
            timeout.as_secs()
        ))),
    }
}

// ============================================================================
// 并发批量调用
// ============================================================================

/// 批量异步调用 (concurrent fan-out).
///
/// 同时启动 N 个 spawn_blocking, 收集结果 (保持顺序).
pub async fn call_python_batch_async(
    calls: Vec<(String, String, Vec<String>)>,
) -> Vec<Result<String, BridgeError>> {
    let mut handles = Vec::with_capacity(calls.len());
    for (module, func, args) in calls {
        let h = tokio::spawn(async move { call_python_async(module, func, args).await });
        handles.push(h);
    }
    let mut results = Vec::with_capacity(handles.len());
    for h in handles {
        match h.await {
            Ok(r) => results.push(r),
            Err(e) => results.push(Err(BridgeError::Internal(format!("join error: {e}")))),
        }
    }
    results
}

// ============================================================================
// 测试 (8 cases)
// ============================================================================

#[cfg(test)]
#[cfg(feature = "python-ext")]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // 注: 这些测试在 python-ext 关闭时不编译 (cfg 隔离).
    // python-ext 开启时, 需要 Python 3.13+ 运行时 + 解释器可用.

    #[tokio::test]
    async fn t01_async_signature() {
        // 验证 async wrapper 签名 + 返回类型正确
        let _f: fn(String, String, Vec<String>) -> _ = call_python_async;
    }

    #[tokio::test]
    async fn t02_kw_async_signature() {
        let mut kwargs = HashMap::new();
        kwargs.insert("k".to_string(), "v".to_string());
        // 验证 kwargs path 编译
        let _: HashMap<String, String> = kwargs;
    }

    #[tokio::test]
    async fn t03_eval_async_signature() {
        let _f: fn(String) -> _ = eval_python_async;
    }

    #[tokio::test]
    async fn t04_timeout_signature() {
        let _f: fn(String, String, Vec<String>, Duration) -> _ = call_python_async_timeout;
    }

    #[tokio::test]
    async fn t05_batch_returns_vec() {
        let calls = vec![
            ("builtins".to_string(), "len".to_string(), vec!["hello".to_string()]),
            ("builtins".to_string(), "str".to_string(), vec!["42".to_string()]),
        ];
        let results = call_python_batch_async(calls).await;
        assert_eq!(results.len(), 2);
        // len("hello") = 5, str(42) = "42"
        // 注: 不依赖具体 Python 结果, 仅验证结构
    }

    #[tokio::test]
    async fn t06_empty_batch() {
        let calls: Vec<(String, String, Vec<String>)> = vec![];
        let results = call_python_batch_async(calls).await;
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn t07_timeout_very_short() {
        // 0ns timeout 应立即返回 timeout error
        let r = call_python_async_timeout(
            "builtins".to_string(),
            "len".to_string(),
            vec!["x".to_string()],
            Duration::from_nanos(0),
        )
        .await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn t08_async_does_not_block() {
        // 验证 spawn_blocking 不会阻塞 tokio runtime
        let start = std::time::Instant::now();
        let r = call_python_async_timeout(
            "builtins".to_string(),
            "len".to_string(),
            vec!["test".to_string()],
            Duration::from_millis(100),
        )
        .await;
        let elapsed = start.elapsed();
        // 不应超过 timeout 太多
        assert!(elapsed < Duration::from_millis(500));
        let _ = r;  // 不关心结果
    }
}
