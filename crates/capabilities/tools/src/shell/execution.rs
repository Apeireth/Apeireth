//! Native shell execution and scoped resource cleanup.
use super::*;

impl ShellTool {
    pub(super) async fn execute_frozen(
        &self,
        call: &ToolCall,
        frozen: &ShellFrozenInvocation,
        context: Option<&apeireth_plugin::ToolExecutionContext>,
    ) -> ToolResult {
        // 文件写入总闸复核 (freeze→execute 时差): 审批等待期可达分钟级, 期间
        // 总闸可能已关 —— 以执行时的闸状态对冻结脚本再扫一次写意图, 关闸后
        // 冻结载荷同样拒绝即帧 (fail-closed)。
        if !self.config.file_write {
            if let Some(script) = frozen.shell_args.last() {
                if let Some(hit) = crate::write_intent::scan_shell_write_intent(script) {
                    return self.write_gate_refusal(call, &hit);
                }
            }
        }

        // L 组 (2026-09-24 审计): freeze→execute TOCTOU 复核。审批等待期可
        // 达分钟级, 期间 frozen cwd 可能被替换成指向工作区外的 symlink ——
        // 冻结时 (build_frozen) 的包含校验届时已失效。执行前对 frozen cwd
        // 再 canonicalize, 以冻结时固化的 workspace_root 复核包含性; root
        // 自身若被换成 symlink (canonicalize 结果与固化值不一致) 同样拒绝。
        let frozen_root = Path::new(&frozen.workspace_root);
        let cwd_canonical = match std::fs::canonicalize(&frozen.cwd) {
            Ok(p) => p,
            Err(e) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("frozen cwd {} is no longer accessible: {e}", frozen.cwd),
                )
                .with_name("shell")
            }
        };
        let root_now = match std::fs::canonicalize(frozen_root) {
            Ok(p) => p,
            Err(e) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!(
                        "frozen workspace root {} is no longer accessible: {e}",
                        frozen.workspace_root
                    ),
                )
                .with_name("shell")
            }
        };
        if root_now.as_path() != frozen_root || !cwd_canonical.starts_with(frozen_root) {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "frozen cwd {} no longer resolves inside the approved workspace root {}",
                    frozen.cwd, frozen.workspace_root
                ),
            )
            .with_name("shell");
        }

        let request = match Self::process_request_from_frozen(frozen) {
            Ok(request) => request,
            Err(e) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("frozen shell invocation unavailable: {e}"),
                )
                .with_name("shell")
            }
        };

        // 对象级判定窗 (E-P3 · G3): 受控执行前后对象状态对比 ——「动了哪些
        // 对象」的事实判定, 与 API 形态无关。词法面漏放的写入 (编码负载 /
        // 解释器内写入等) 在这里按对象事实兜底; 越界即帧 + 落
        // `sandbox.object_violation` 审计。判定窗开不出 = 对象事实不可核验,
        // fail-closed 不执行。
        let judge = Arc::clone(&self.judge);
        let subject = ObjectSubject::new(frozen.shell_executable.clone());
        let domain = ObjectDomain::new(PathBuf::from(&frozen.workspace_root));
        let cancellation = context
            .map(|ctx| ctx.cancellation.clone())
            .unwrap_or_default();
        let _cancel_on_drop = cancellation.cancel_on_drop();
        let request = request.with_cancellation(cancellation);
        let Ok(native_lease) = context
            .map(|ctx| {
                ctx.module_tasks
                    .lease_with_cancellation(ctx.cancellation.clone())
            })
            .transpose()
        else {
            return ToolResult::permanent_error(&call.id, "shell module is stopping")
                .with_name("shell");
        };
        let judged = crate::process::run_native(move || {
            let _native_lease = native_lease;
            judge.guard(subject, domain, || {
                crate::process::ProcessExecutor::new().execute(&request)
            })
        })
        .await;

        match judged {
            Ok(Err(guard_error)) => {
                let failure = crate::exec_pipeline::PipelineFailure::PreDenied {
                    source: crate::object_access::OBJECT_JUDGE_SOURCE.to_string(),
                    reason: guard_error.to_string(),
                };
                failure.emit(&call.id, Some("shell"))
            }
            Ok(Ok((process_result, report))) => {
                // 越界即帧: 未授权对象操作判失败 (审计已随判定落行)。
                if let Some(failure) = report.failure() {
                    return failure.emit(&call.id, Some("shell"));
                }
                match process_result {
                    Ok(process_result) => {
                        let mut value = Self::format_result(&process_result);
                        // 对象审计可见墙: 本次执行动了哪些对象 (计数口径) ——
                        // 合规执行零帧零审计行, 事实计数随结果可见。
                        value["object_audit"] = serde_json::json!({
                            "changes": report.changes().len(),
                            "violations": report.violations().len(),
                            "coverage": if report.complete() { "complete" } else { "partial" },
                        });
                        ToolResult::ok(&call.id, value).with_name("shell")
                    }
                    Err(process_error) => ToolResult::permanent_error(
                        &call.id,
                        format!("shell process execution failed: {process_error}"),
                    )
                    .with_name("shell"),
                }
            }
            Err(join_error) => ToolResult::retryable_error(
                &call.id,
                format!("shell execution task failed to join: {join_error}"),
            )
            .with_name("shell"),
        }
    }
}
