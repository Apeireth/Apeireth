//! V5-c 守望巡检漏斗的集成证据: 漏斗四段流转 / 复核降噪 (瞬时态与风险漂移剔除)/
//! 增量 diff (新增·消失·未变)/ 成本闸截断如实 (件数闸 + 耗时闸)/ 断点续查
//! (中途死重启续巡, 已做不重做)/ 巡检任务化 (队列同账, 失败留账只能人判)/
//! 脱敏 (精查取证与落库档零明文)/ 被测面只读零写 / 诚实未测零编造。
//!
//! 口径: 全部 fixture 落临时目录; 每个涉密 fixture 都带「不得出现在报告里」
//! 断言 (零明文留痕)。既有 V5-b 套件零触碰 —— 本文件是漏斗层的独立证据面。

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_orchestration::task_queue::{
    EnqueueOutcome, InMemoryTaskStore, TaskCaller, TaskQueue, TaskQueueConfig, TaskQueueError,
    TaskState,
};
use apeireth_tools_canonical::exposure_audit::funnel::{
    run_patrol, DeepEvidenceKind, FunnelBudget, FunnelConfig, JsonPatrolStore, MemoryPatrolStore,
    PatrolCheckpoint, PatrolError, PatrolStateStore, StageBudget, FUNNEL_REPORT_SCHEMA,
    FUNNEL_REPORT_SCHEMA_VERSION, STAGE_ORDER,
};
use apeireth_tools_canonical::exposure_audit::{
    BindingSurface, ExposureAuditConfig, Listener, ListenerScope, ListenerSnapshot,
};

const SECRET_A: &str = "sk-live-must-not-echo-3";
const SECRET_B: &str = "hunter2-must-not-echo";

/// 干净探测口: 空监听面 + 已核对一致的绑定面 (fixture 面防误报的基线)。
fn clean_audit(root: &Path) -> ExposureAuditConfig {
    ExposureAuditConfig::new(root)
        .with_listeners(Arc::new(|| {
            Ok(ListenerSnapshot {
                listeners: Vec::new(),
                unparsed_rows: 0,
                duplicate_rows: 0,
            })
        }))
        .with_bindings(Arc::new(|| BindingSurface::Checked {
            clients: 1,
            users: 1,
            problems: Vec::new(),
        }))
}

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// 有状态监听探测口: 第 N 次给不同快照 (瞬时态/风险漂移的注入点)。
fn stateful_listener_probe(
    calls: Arc<AtomicUsize>,
    first: Vec<Listener>,
    later: Vec<Listener>,
) -> Arc<dyn Fn() -> Result<ListenerSnapshot, String> + Send + Sync> {
    Arc::new(move || {
        let nth = calls.fetch_add(1, Ordering::SeqCst);
        let listeners = if nth == 0 {
            first.clone()
        } else {
            later.clone()
        };
        Ok(ListenerSnapshot {
            listeners,
            unparsed_rows: 0,
            duplicate_rows: 0,
        })
    })
}

fn wildcard_listener() -> Listener {
    Listener {
        proto: "tcp".to_string(),
        local_addr: "0.0.0.0".to_string(),
        port: 8443,
        scope: ListenerScope::Wildcard,
        pid: Some(4242),
    }
}

/// 同一监听词面、不同测量风险 (风险漂移的注入点: 作用域分级变了)。
fn risk_shifted_listener() -> Listener {
    Listener {
        proto: "tcp".to_string(),
        local_addr: "0.0.0.0".to_string(),
        port: 8443,
        scope: ListenerScope::Loopback,
        pid: Some(4242),
    }
}

/// 模拟中途猝死的落库口: 前 N 次正常落盘, 之后失败即帧 (真断点续查的注入点)。
struct InterruptStore {
    inner: JsonPatrolStore,
    saves_left: Mutex<usize>,
}

impl InterruptStore {
    fn new(inner: JsonPatrolStore, saves_left: usize) -> Self {
        Self {
            inner,
            saves_left: Mutex::new(saves_left),
        }
    }
}

impl PatrolStateStore for InterruptStore {
    fn load(&self) -> Result<Option<PatrolCheckpoint>, PatrolError> {
        self.inner.load()
    }

    fn save(&self, checkpoint: &PatrolCheckpoint) -> Result<(), PatrolError> {
        let mut left = self.saves_left.lock().unwrap();
        if *left == 0 {
            return Err(PatrolError::Store {
                operation: "write_checkpoint",
                reason: "模拟中途猝死 (测试注入)".to_string(),
            });
        }
        *left -= 1;
        self.inner.save(checkpoint)
    }
}

/// 每次调用推进 1 秒的可控时钟 (耗时闸的确定性注入点)。
fn ticking_clock() -> Arc<dyn Fn() -> i64 + Send + Sync> {
    let now = Arc::new(Mutex::new(1_700_000_000_000i64));
    Arc::new(move || {
        let mut guard = now.lock().unwrap();
        let current = *guard;
        *guard += 1_000;
        current
    })
}

/// 盘面快照 (相对路径 + 字节): 只读零写的比对基准。
fn snapshot(root: &Path) -> Vec<(String, u64)> {
    fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, u64)>) {
        let mut entries: Vec<fs::DirEntry> =
            fs::read_dir(dir).unwrap().filter_map(Result::ok).collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                collect(root, &path, out);
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = fs::read(&path).unwrap();
            out.push((rel, bytes.len() as u64));
        }
    }
    let mut out = Vec::new();
    collect(root, root, &mut out);
    out
}

// ── 1. 漏斗四段流转 ──────────────────────────────────────────────────────

#[test]
fn funnel_flows_screen_deep_recheck_report_in_order() {
    let data = tempfile::tempdir().expect("temp dir");
    write(
        &data.path().join("app.toml"),
        &format!("name = \"apeireth\"\napi_key = \"{SECRET_A}\"\n"),
    );
    write(&data.path().join("creds.json"), r#"{"token":"raw-2"}"#);

    let store = MemoryPatrolStore::new();
    let report =
        run_patrol(&FunnelConfig::new(clean_audit(data.path())), &store).expect("patrol runs");

    // 四段固定顺序, 逐段账首尾相接。
    let names: Vec<&str> = report
        .stages
        .iter()
        .map(|stage| stage.stage.as_str())
        .collect();
    assert_eq!(names, STAGE_ORDER, "漏斗四段固定顺序");
    assert_eq!(report.stages[0].candidates_in, 2, "六项速扫发现 2 条");
    assert_eq!(report.noise.screened, 2, "筛查段候选进漏斗");
    assert_eq!(report.stages[1].candidates_in, 2, "两条候选都过精查线");
    assert_eq!(report.stages[2].candidates_in, 2, "复核段复验全部候选");
    assert_eq!(report.stages[3].candidates_in, report.confirmed.len());
    assert!(report.completed, "无截断时一次跑完");
    assert!(report.diff.is_some(), "完成巡检产出增量");

    // 每条上报候选都带精查取证 (脱敏后呈现)。
    assert_eq!(report.confirmed.len(), 2);
    for candidate in &report.confirmed {
        assert!(candidate.deep_checked, "过精查线的候选必须深查过");
        assert!(candidate.evidence.is_some());
    }
    assert_eq!(report.task.state, TaskState::Done, "巡检=任务, 完成即 Done");
}

// ── 2/3. 复核降噪 (瞬时态 / 风险漂移剔除) ────────────────────────────────

#[test]
fn recheck_drops_transient_probe_findings_as_noise() {
    let data = tempfile::tempdir().expect("temp dir");
    write(
        &data.path().join("app.toml"),
        &format!("api_key = \"{SECRET_A}\"\n"),
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let audit = clean_audit(data.path()).with_listeners(stateful_listener_probe(
        Arc::clone(&calls),
        vec![wildcard_listener()],
        Vec::new(), // 二次测量: 监听面消失 = 瞬时态
    ));

    let store = MemoryPatrolStore::new();
    let report = run_patrol(&FunnelConfig::new(audit), &store).expect("patrol runs");

    assert!(report.completed);
    assert_eq!(report.noise.dropped_transient, 1, "复核段剔除瞬时态");
    assert_eq!(report.noise.dropped_risk_shift, 0);
    assert_eq!(report.noise.confirmed, 1, "只有二次测量一致的才报");
    assert!((report.noise.noise_reduction_rate - 0.5).abs() < f64::EPSILON);
    let keys: Vec<&str> = report
        .confirmed
        .iter()
        .map(|candidate| candidate.candidate.key.as_str())
        .collect();
    assert_eq!(keys, vec!["plaintext_points|app.toml"]);
    assert!(
        !keys.iter().any(|key| key.starts_with("listening_ports|")),
        "瞬时态监听面不得上报"
    );
}

#[test]
fn recheck_drops_risk_shift_findings_as_noise() {
    let data = tempfile::tempdir().expect("temp dir");
    write(
        &data.path().join("app.toml"),
        &format!("api_key = \"{SECRET_A}\"\n"),
    );
    let audit = clean_audit(data.path()).with_listeners(stateful_listener_probe(
        Arc::new(AtomicUsize::new(0)),
        vec![wildcard_listener()],     // 一次测量: 全接口监听 = 中风险
        vec![risk_shifted_listener()], // 二次测量: 同词面风险漂移 (不一致不报)
    ));

    let store = MemoryPatrolStore::new();
    let report = run_patrol(&FunnelConfig::new(audit), &store).expect("patrol runs");

    assert_eq!(report.noise.dropped_risk_shift, 1, "二次测量不一致不报");
    assert_eq!(report.noise.dropped_transient, 0);
    assert_eq!(report.noise.confirmed, 1);
}

// ── 4. 增量 diff (新增 / 消失 / 未变) ────────────────────────────────────

#[test]
fn diff_marks_new_gone_and_unchanged_between_patrols() {
    let data = tempfile::tempdir().expect("temp dir");
    let state = tempfile::tempdir().expect("state dir");
    let store = JsonPatrolStore::in_dir(state.path());
    let config = FunnelConfig::new(clean_audit(data.path()));

    // 巡检一: 只有 a.toml → 全部新增 (首次无基线)。
    write(
        &data.path().join("a.toml"),
        &format!("api_key = \"{SECRET_A}\"\n"),
    );
    let first = run_patrol(&config, &store).expect("patrol 1");
    let diff = first.diff.as_ref().expect("完成巡检有增量");
    assert_eq!(diff.baseline_patrol_id, None, "首次巡检无基线");
    assert_eq!(diff.new, vec!["plaintext_points|a.toml"]);
    assert!(diff.gone.is_empty());
    assert!(diff.unchanged.is_empty());

    // 巡检二: + b.toml → b 新增, a 未变。
    write(
        &data.path().join("b.toml"),
        &format!("token = \"{SECRET_B}\"\n"),
    );
    let second = run_patrol(&config, &store).expect("patrol 2");
    let diff = second.diff.as_ref().expect("完成巡检有增量");
    assert_eq!(diff.new, vec!["plaintext_points|b.toml"]);
    assert_eq!(diff.unchanged, vec!["plaintext_points|a.toml"]);
    assert!(diff.gone.is_empty());

    // 巡检三: − b.toml → b 消失, a 仍未变。
    fs::remove_file(data.path().join("b.toml")).unwrap();
    let third = run_patrol(&config, &store).expect("patrol 3");
    let diff = third.diff.as_ref().expect("完成巡检有增量");
    assert_eq!(diff.gone, vec!["plaintext_points|b.toml"]);
    assert_eq!(diff.unchanged, vec!["plaintext_points|a.toml"]);
    assert!(diff.new.is_empty());
}

// ── 5/6. 成本闸截断如实 (件数闸 + 耗时闸) ───────────────────────────────

#[test]
fn cost_gate_truncates_deep_stage_and_stays_honest() {
    let data = tempfile::tempdir().expect("temp dir");
    write(
        &data.path().join("a.toml"),
        &format!("api_key = \"{SECRET_A}\"\n"),
    );
    write(
        &data.path().join("b.toml"),
        &format!("token = \"{SECRET_B}\"\n"),
    );
    write(&data.path().join("creds.json"), r#"{"token":"raw-3"}"#);

    let budget = FunnelBudget {
        deep: StageBudget {
            max_candidates: 1,
            max_duration_ms: 120_000,
        },
        ..FunnelBudget::default()
    };
    let store = MemoryPatrolStore::new();
    let config = FunnelConfig::new(clean_audit(data.path())).with_budget(budget);
    let report = run_patrol(&config, &store).expect("patrol runs");

    // 超限截断如实: 处理 1 件、余量 2 件留账、显式截断、不冒充完成。
    assert_eq!(report.stages[1].processed, 1, "精查段按闸只处理 1 件");
    assert_eq!(report.stages[1].held_back, 2, "余量如实留账");
    assert!(report.stages[1].truncated, "截断显式标注");
    assert!(!report.completed, "被截断的会话不冒充完成");
    assert!(report.diff.is_none(), "未完成不与上次比 (如实)");
    let evidence_count = report
        .confirmed
        .iter()
        .filter(|candidate| candidate.evidence.is_some())
        .count();
    assert_eq!(evidence_count, 1, "只有截断前深查过的带取证");
    assert_eq!(report.budget.deep.max_candidates, 1, "口径回显可复核");
}

#[test]
fn time_budget_truncates_screen_stage_and_stays_honest() {
    let data = tempfile::tempdir().expect("temp dir");
    write(
        &data.path().join("a.toml"),
        &format!("api_key = \"{SECRET_A}\"\n"),
    );
    write(
        &data.path().join("b.toml"),
        &format!("token = \"{SECRET_B}\"\n"),
    );

    let clock = ticking_clock();
    let budget = FunnelBudget {
        screen: StageBudget {
            max_candidates: 1_024,
            max_duration_ms: 2_000,
        },
        ..FunnelBudget::default()
    };
    let store = MemoryPatrolStore::new();
    let config = FunnelConfig::new(clean_audit(data.path()))
        .with_budget(budget)
        .with_clock(clock);
    let report = run_patrol(&config, &store).expect("patrol runs");

    assert!(report.stages[0].truncated, "耗时闸截断如实标注");
    assert!(report.stages[0].held_back >= 1, "未准入候选留账不丢");
    assert!(report.noise.screened >= 1, "截断前已准入的照常走漏斗");
    assert!(report.noise.screened < 2, "确有候选没轮到");
    assert!(!report.completed, "截断不冒充完成");
    assert!(report.diff.is_none());
}

// ── 7. 断点续查 (中途死重启续巡, 已做不重做) ────────────────────────────

#[test]
fn checkpoint_resumes_after_mid_run_death_without_rework() {
    let data = tempfile::tempdir().expect("temp dir");
    write(&data.path().join("creds.json"), r#"{"token":"raw-4"}"#);
    write(
        &data.path().join("a.toml"),
        &format!("api_key = \"{SECRET_A}\"\n"),
    );
    write(
        &data.path().join("b.toml"),
        &format!("token = \"{SECRET_B}\"\n"),
    );
    let state = tempfile::tempdir().expect("state dir");
    let config = FunnelConfig::new(clean_audit(data.path()));

    // 会话一: 落库 6 次后中途猝死 (错误即帧, 不静默)。
    let dying = InterruptStore::new(JsonPatrolStore::in_dir(state.path()), 6);
    let died = run_patrol(&config, &dying);
    assert!(
        matches!(died, Err(PatrolError::Store { .. })),
        "中途死必须错误即帧: {died:?}"
    );

    // 死前进度已落库: 精查已做 2 件 (不重做账)。
    let store = JsonPatrolStore::in_dir(state.path());
    let mid = store.load().expect("load").expect("已落库");
    assert!(!mid.completed);
    assert_eq!(mid.deep.done_keys.len(), 2, "死前已精查 2 件落了库");
    assert_eq!(mid.task.attempts, 1);

    // 会话二: 重启续巡 —— 同一巡检、只做余下、一次跑完。
    let report = run_patrol(&config, &store).expect("resume runs");
    assert!(report.resumed, "续跑显式标注");
    assert_eq!(report.patrol_id, mid.patrol_id, "同一巡检接着跑");
    assert_eq!(report.attempts, 2, "每个执行段计一次 (崩溃恢复同款)");
    assert_eq!(report.stages[1].processed, 1, "已精查不重做, 只做余下");
    assert!(report.completed, "续跑把巡检跑完");
    assert!(report.diff.is_some(), "完成巡检产出增量");
    for candidate in &report.confirmed {
        assert!(candidate.evidence.is_some(), "完成时逐件取证齐整");
    }
}

// ── 8. 巡检任务化 (队列同账, 失败留账只能人判) ──────────────────────────

#[test]
fn patrol_task_shares_queue_account_and_keeps_human_judge_only_failure() {
    let data = tempfile::tempdir().expect("temp dir");
    let store = MemoryPatrolStore::new();
    let report =
        run_patrol(&FunnelConfig::new(clean_audit(data.path())), &store).expect("patrol runs");
    assert_eq!(report.task.state, TaskState::Done);

    // 巡检 = 任务: 入队规格是队列既有写面吃的形状。
    let spec = report.task.to_task_spec();
    assert!(spec.title.contains("守望巡检"));
    assert!(spec.idempotency_key.is_some(), "同键幂等同款");

    let queue = TaskQueue::open(
        Arc::new(InMemoryTaskStore::new()),
        TaskQueueConfig::default(),
    )
    .expect("queue opens");
    let outcome = queue.enqueue(spec).expect("enqueue via existing face");
    let EnqueueOutcome::Enqueued { task: record, .. } = outcome else {
        panic!("首次入队应为新任务");
    };

    // 队列同账: 侧读回来的是同一本账 (id / 幂等键 / 状态 / 执行段计数)。
    let mut account = report.task.clone();
    account
        .load_from_queue(&queue, &record.id)
        .expect("read-only sync");
    assert_eq!(account.task_id.as_deref(), Some(record.id.as_str()));
    assert_eq!(
        account.idempotency_key,
        record.idempotency_key.clone().unwrap()
    );
    assert_eq!(account.state, record.state);
    assert_eq!(account.attempts, record.attempts);

    // 失败留账人判同款: 队列本体与巡检账都是 agent 碰不到的失败面。
    let agent = TaskCaller::agent("exposure-patrol");
    let owner = TaskCaller::human("owner");
    assert!(matches!(
        queue.human_judge_fail(&record.id, &agent, "boom"),
        Err(TaskQueueError::HumanOnly { .. })
    ));
    assert!(matches!(
        account.human_judge_fail(&agent, "boom", 1),
        Err(TaskQueueError::HumanOnly { .. })
    ));
    queue
        .human_judge_fail(&record.id, &owner, "boom")
        .expect("人判失败入账");
    account
        .human_judge_fail(&owner, "boom", 1)
        .expect("人判失败入账");
    assert_eq!(account.state, TaskState::Failed);
    assert!(account.failure.as_ref().expect("失败留账").human_judge_only);

    // 失败绝不自动重试: 重试是显式人判动作, agent 结构性拒绝。
    assert!(matches!(
        account.human_judge_retry(&agent),
        Err(TaskQueueError::HumanOnly { .. })
    ));
    assert_eq!(account.state, TaskState::Failed);
    account.human_judge_retry(&owner).expect("人判重试回队");
    assert_eq!(account.state, TaskState::Pending);
}

// ── 9. 脱敏 (精查取证与落库档零明文) ────────────────────────────────────

#[test]
fn deep_evidence_and_checkpoint_carry_zero_plaintext() {
    let data = tempfile::tempdir().expect("temp dir");
    write(
        &data.path().join("app-config.toml"),
        &format!("name = \"apeireth\"\napi_key = \"{SECRET_A}\"\npassword = \"{SECRET_B}\"\n"),
    );
    let state = tempfile::tempdir().expect("state dir");
    let store = JsonPatrolStore::in_dir(state.path());
    let report =
        run_patrol(&FunnelConfig::new(clean_audit(data.path())), &store).expect("patrol runs");

    let candidate = report
        .confirmed
        .iter()
        .find(|candidate| candidate.candidate.key == "plaintext_points|app-config.toml")
        .expect("明文点候选");
    let evidence = candidate.evidence.as_ref().expect("字段级取证");
    assert_eq!(evidence.kind, DeepEvidenceKind::PlaintextFields);
    let lines = evidence.lines.join("\n");
    assert!(lines.contains("[redacted]"), "取证行脱敏后呈现: {lines}");
    assert!(!lines.contains(SECRET_A), "取证行零明文: {lines}");
    assert!(!lines.contains(SECRET_B), "取证行零明文: {lines}");

    // 报告与落库档双双零明文 (零明文留痕贯穿到底)。
    let rendered = report.to_json();
    assert!(!rendered.contains(SECRET_A), "报告零明文");
    assert!(!rendered.contains(SECRET_B), "报告零明文");
    let checkpoint_text = fs::read_to_string(store.path()).expect("落库档可读");
    assert!(!checkpoint_text.contains(SECRET_A), "落库档零明文");
    assert!(!checkpoint_text.contains(SECRET_B), "落库档零明文");
}

// ── 10. 被测面只读零写 (状态落库与被测面分开) ───────────────────────────

#[test]
fn audited_face_stays_read_only_while_state_lands_outside() {
    let data = tempfile::tempdir().expect("temp dir");
    write(
        &data.path().join("app.toml"),
        &format!("api_key = \"{SECRET_A}\"\n"),
    );
    write(&data.path().join("creds.json"), r#"{"token":"raw-5"}"#);
    let state = tempfile::tempdir().expect("state dir");
    let store = JsonPatrolStore::in_dir(state.path());

    let before = snapshot(data.path());
    let report =
        run_patrol(&FunnelConfig::new(clean_audit(data.path())), &store).expect("patrol runs");
    let after = snapshot(data.path());

    assert!(report.completed);
    assert_eq!(before, after, "被测面盘面零变化 (只读零写)");
    assert!(store.path().is_file(), "巡检状态落库在自己的工作区");
}

// ── 11. 备份排除项逐件核对 (精查段深查) ─────────────────────────────────

#[test]
fn backup_exclusion_deep_check_audits_bundle_item_by_item() {
    let data = tempfile::tempdir().expect("temp dir");
    write(&data.path().join("users.json"), r#"{"users":[]}"#);
    let bundle = tempfile::tempdir().expect("bundle dir");
    write(
        &bundle.path().join("manifest.json"),
        &format!(
            r#"{{
            "format_version": 1,
            "product_version": "2.0.0-test",
            "created_at_ms": 0,
            "data_root": "C:/data",
            "files": [
                {{"category": "data", "path": "files/data/users/client-tokens.json", "origin": "C:/data/users/client-tokens.json", "relpath": "users/client-tokens.json", "bytes": 2, "sha256": "00", "mtime_ms": null}},
                {{"category": "data", "path": "files/data/notes.json", "origin": "C:/data/notes.json", "relpath": "notes.json", "bytes": 2, "sha256": "00", "mtime_ms": null}}
            ],
            "excluded": [
                {{"path": "creds.json", "reason": "凭据面不随行"}},
                {{"path": "server.pem", "reason": ""}}
            ]
            }}"#
        ),
    );
    write(
        &bundle.path().join("files/data/users/client-tokens.json"),
        "{}",
    );
    write(&bundle.path().join("files/data/creds.json"), "{}"); // 显式排除项泄漏进包

    let audit = clean_audit(data.path()).with_bundle(bundle.path());
    let store = MemoryPatrolStore::new();
    let report = run_patrol(&FunnelConfig::new(audit), &store).expect("patrol runs");
    assert!(report.completed);

    let audits: Vec<_> = report
        .confirmed
        .iter()
        .filter_map(|candidate| candidate.evidence.as_ref())
        .filter(|evidence| evidence.kind == DeepEvidenceKind::ManifestAudit)
        .collect();
    assert!(!audits.is_empty(), "备份排除项走逐件核对取证");
    let joined = audits
        .iter()
        .map(|evidence| format!("{} {}", evidence.summary, evidence.lines.join("\n")))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("逐件核对"), "{joined}");
    assert!(
        joined.contains("client-tokens.json") && joined.contains("违规"),
        "清单行逐件对词表: {joined}"
    );
    assert!(
        joined.contains("排除项出现在包内") && joined.contains("creds.json"),
        "显式排除项泄漏逐件揪出: {joined}"
    );
}

// ── 12. 诚实未测零编造 ──────────────────────────────────────────────────

#[test]
fn clean_surface_fabricates_no_candidates() {
    let data = tempfile::tempdir().expect("temp dir");
    write(&data.path().join("users.json"), r#"{"users":[]}"#);
    let store = MemoryPatrolStore::new();
    let report =
        run_patrol(&FunnelConfig::new(clean_audit(data.path())), &store).expect("patrol runs");

    assert_eq!(report.stages[0].candidates_in, 0, "干净面零发现");
    assert_eq!(report.noise.screened, 0);
    assert_eq!(report.noise.noise_reduction_rate, 0.0, "零除零记 0, 不编数");
    assert!(report.confirmed.is_empty(), "零候选零上报");
    assert!(report.completed, "空巡检也是完成的巡检");
    let diff = report.diff.as_ref().expect("完成巡检有增量");
    assert!(diff.new.is_empty() && diff.gone.is_empty() && diff.unchanged.is_empty());
}

// ── 13. 漏斗报告 JSON 合同 ──────────────────────────────────────────────

#[test]
fn funnel_report_json_contract_is_stable() {
    let data = tempfile::tempdir().expect("temp dir");
    write(
        &data.path().join("a.toml"),
        &format!("api_key = \"{SECRET_A}\"\n"),
    );
    let store = MemoryPatrolStore::new();
    let report =
        run_patrol(&FunnelConfig::new(clean_audit(data.path())), &store).expect("patrol runs");
    let value = report.to_json_value();

    assert_eq!(value["schema"], FUNNEL_REPORT_SCHEMA);
    assert_eq!(value["schema_version"], FUNNEL_REPORT_SCHEMA_VERSION);
    assert_eq!(value["completed"], true);
    assert_eq!(value["attempts"], 1);
    assert_eq!(value["task"]["state"], "done");
    let stages = value["stages"].as_array().expect("stages array");
    assert_eq!(stages.len(), 4);
    for (index, name) in STAGE_ORDER.iter().enumerate() {
        assert_eq!(stages[index]["stage"], *name);
    }
    assert!(value["noise"]["noise_reduction_rate"].is_number());
    assert_eq!(value["confirmed"][0]["key"], "plaintext_points|a.toml");
    assert_eq!(value["diff"]["new"][0], "plaintext_points|a.toml");
    assert!(value["budget"]["deep"]["max_candidates"].is_number());
}
