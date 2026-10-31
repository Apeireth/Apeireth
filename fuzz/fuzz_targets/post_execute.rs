#![no_main]

//! fuzz: **执行后纠错通道面**（工具执行五段流水线的第 4 段）——
//! `PostExecuteWaterfall` / `PostVerdict` / `PostDecision` / `SupersededResult`，
//! 以及它在 `ToolExecutionPipeline::run_in_session` 里的**落点**
//! （含与 pre 段、normalize 段的交互）。
//!
//! ## 为什么这个面值得 fuzz
//!
//! post 段决定「**工具已经跑完之后**，结果要不要被重新裁定/纠正/打回」。
//! * **误放行** = 失败/危险的结果被当成成功返回给人；
//! * **误拒** = 正常结果被丢弃。
//!
//! 它与 pre 段的**不对称**尤其值得看：**pre 拒 ⇒ 不执行；post 拒 ⇒ 已经执行了**
//! ⇒ post 的错误**代价更高**（副作用已发生）。
//!
//! **本仓已有 target `exec_pipeline` 覆盖的是第 1 段（pre-execute）**，
//! 本 target 只做**第 4 段（post-execute）**及其与其它段的交互 ⇒ 不重复 pre 段。
//!
//! ## 契约来源（断言逐条取自被测代码自己写明的契约 —— **不自行发明期望值**）
//!
//! | # | 契约 | 出处（文件:行 + 原文片段） |
//! |---|---|---|
//! | C1 | **首个决断判定赢**（`replace` / `block`），**后面的纠错器不再被咨询**；后到的 hook **不得翻转**前一个的判定 | `crates/capabilities/tools/src/exec_pipeline/post.rs:12-13`「The first decisive verdict (`replace` / `block`) wins and later correctors are not consulted.」 |
//! | C2 | 空通道：**每个结果都原样接受** | `post.rs:110-111`「An empty channel: every result is accepted unchanged.」+ `post.rs:14`「with none installed every result is accepted unchanged」 |
//! | C3 | 咨询顺序 = 登记顺序（`with` 追加，`names()` 按登记顺序列出） | `post.rs:115-116`「Append one corrector. Consultation order equals insertion order.」+ `post.rs:132-135` |
//! | C4 | `decided_by` = 决断的纠错器，**全部接受时缺席**；`consulted` = **实际被咨询**的纠错器名、按咨询顺序 | `post.rs:86-90`「The corrector that decided, absent when every corrector accepted.」+「Corrector names actually consulted, in consultation order.」 |
//! | C5 | 判定标签：`accept` / `replace` / `block`；`is_accept()` ⟺ 标签为 `accept` | `post.rs:42-54` |
//! | C6 | 请求事实**只读**（`PostExecuteRequest` 持 `&ToolCall` + `&ToolResult`）⇒ 每个纠错器看到的**完全一致** | `post.rs:57-64`「The facts a corrector may judge.」+ 字段类型 `pub call: &'a ToolCall` / `pub result: &'a ToolResult` |
//! | C7 | **确定性**：通道自身无状态（只持 `Vec<Arc<dyn PostExecuteHook>>`，`evaluate(&self, …)`）⇒ 同一组 hook + 同一请求 ⇒ 两次 `evaluate` 结果相同 | `post.rs:103-107`（结构体定义）+ `post.rs:138`（`pub fn evaluate(&self, …)`） |
//! | C8 | `replace` ⇒ **换入纠正结果**，**原结果被归档**在阶段记录里（且在原结果被处置之前） | `post.rs:8-9`「`replace` — swap in a corrected result; the original is archived in the stage record before anything else happens to it」 |
//! | C9 | `block` ⇒ **判为失败**，原结果**同样归档** | `post.rs:10-11`「`block` — judge the call a failure; the original is archived the same way.」 |
//! | C10 | 归档条目四要素齐备：阶段 / actor / 原结果 / 理由；**nothing is ever thrown away silently** | `post.rs:159-172`「Nothing is ever thrown away silently: a replaced or blocked result is kept here with the stage, the actor, and the stated reason.」 |
//! | C11 | **pre 拒 ⇒ 什么都不执行**（关闭成一次完成运行，不执行任何东西）⇒ post 段**从未被咨询** | `exec_pipeline/mod.rs:534`「Close a refusal into a completed run without executing anything.」+ `mod.rs:420-427` |
//! | C12 | 阶段顺序**固定**，与调用方无关；记录按访问顺序列出 | `exec_pipeline/mod.rs:76-77`「The canonical order. Stages always run in exactly this order.」+ `mod.rs:22-23` |
//! | C13 | 失败帧**与调用 id 相关联**、带稳定码 | `exec_pipeline/mod.rs:264-267`「The result always carries the stable code and stays correlated to the call id; a failure is a frame, not a hole.」 |
//! | C14 | `replace` 之后**被顶替的那次失败不再是活失败** | `exec_pipeline/mod.rs:486-489`「The corrector produced the final result; whatever the superseded one suffered is history, not a live failure.」 |
//! | C15 | normalize 段：违反声明的 schema ⇒ **报错，不吞、不悄悄改形**；纠正结果被顶替时它也进归档 | `exec_pipeline/contract.rs:5-9`「A shape that violates its schema is a **normalization error** reported in the open, never a silently coerced or silently dropped value.」+ `mod.rs:505-524` |
//! | C16 | 结果的**相关 id** 必须是被回答的那个调用的 id | `crates/foundation/protocol/src/canonical/tool_result.rs:55-59`「The `id` of the [`ToolCall`] this answers. Providers match results to calls by this id.」 |
//!
//! ## **顺序敏感性**的如实记录（不自行发明期望值）
//!
//! 契约 C1 明写 post 通道是**顺序敏感**的（`The first decisive verdict … wins`
//! + `later correctors are not consulted`）⇒ **本 target 不断言**「任意排列结果相同」。
//! 本 target 断言的是契约真正承诺的两件事：
//!
//! * **顺序敏感面**：给定登记顺序，合并结果 = 该顺序上**第一个**决断判定；
//! * **顺序无关面**：**只要**纠错器里有一个决断判定（`replace` / `block`），
//!   **任何排列**下合并结果**必是决断**（不可能回到 `accept`）
//!   （C1 的「首个决断赢 / 后面的不被咨询」在任意排列下的推论）。
//!
//! ## 已知**不**断言的边界（避免自造期望值 → 假阳性）
//!
//! * **回滚副作用**：post 段在工具**跑完之后**才裁定；契约**没有**承诺回滚
//!   ⇒ 本 target **只断言事实**（工具确实执行了、原结果确实进了归档），
//!   **不断言**「副作用被撤销」。
//! * **失败理由文本的渲染格式**：`PipelineFailure::message()` 的具体措辞无契约
//!   ⇒ 只断言**稳定码**（`PipelineFailure::code()`），不断言整串措辞。
//! * **`summary()` 的排版**：只承诺「一行决策记录」⇒ 只断言它**含判定标签**。
//! * **纠正结果的 `name` 字段**：契约只承诺**相关 id**（C16），没说 `name`
//!   ⇒ 不断言 `name`。
//! * **`failure` 是否为 `Some`（工具自身失败、且未配 retry 时）**：
//!   `PipelineFailure` 词表里只有「重试耗尽」而无「工具报错」⇒ 该行为
//!   **无文档承诺** ⇒ **不断言**（只断言「不是 `RetriesExhausted`」这类弱口径）。
//! * **空 schema 的口径歧义**：`OutputSchema::new()` 的文档写「A schema with no
//!   required fields: every structured value conforms.」，而 `validate` 对**非对象**
//!   一律报 `NotStructured` ⇒ 两种读法都说得通 ⇒ **不断言**空 schema 下的
//!   归一结果（见交付报告里的「观察」条目）。
//! * **timeout 臂**：`around.rs` 的 timeout 走 `tokio::runtime::Handle::try_current`
//!   （`crates/foundation/core/src/deadline.rs:150`）⇒ fuzz 子工作区**没有 tokio**
//!   ⇒ 本 target 只覆盖 **retry 臂**（timeout 臂由仓内 tokio 测试覆盖）。

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{
    NormalizedTool, ToolCall, ToolOutcome as WireToolOutcome, ToolParameters, ToolResult,
};
use apeireth_tools_canonical::{
    AroundPolicy, ExecutionStage, NormalizationError, OutputSchema, PipelineFailure,
    PostExecuteHook, PostExecuteRequest, PostExecuteWaterfall, PostVerdict, PreExecuteHook,
    PreExecuteRequest, PreVerdict, RetryPolicy, SchemaKind, ToolExecutionPipeline,
};
use libfuzzer_sys::fuzz_target;
use serde_json::{Value, json};

// ── 固定夹具 ────────────────────────────────────────────────────────────────

/// 合法能力 id（`CapabilityId::new` 会校验稳定 id 文法，非法则回落到它）。
const FALLBACK_CAPABILITY: &str = "tool.demo";

/// 纠错器名字池（覆盖空名 / 重名 / 怪字符）。
const CORRECTOR_NAMES: [&str; 8] = [
    "c_alpha",
    "c_beta",
    "",
    "c_alpha",
    "c 名字",
    "c\u{200B}beta",
    "c\nbeta",
    "c\u{0}beta",
];

/// 工具结果的成功载荷池（含非对象形态 —— 用来打 normalize 段）。
const PAYLOADS: [&str; 6] = [
    r#"{"summary":"fine","count":1}"#,
    r#"{"summary":"fine"}"#,
    r#"{"count":1}"#,
    r#"[1,2,3]"#,
    r#""just a string""#,
    r#"{"summary":7,"count":1}"#,
];

// ── 脚本模型 ────────────────────────────────────────────────────────────────

/// 工具结果的成功/失败形态。
#[derive(Debug, Clone)]
enum ResultSpec {
    /// 成功，携带一个结构化载荷。
    Ok(Value),
    /// 失败，携带一条错误消息。
    Error(String),
}

/// 纠正结果脚本（`PostVerdict::Replace` 里的 `corrected`）。
#[derive(Debug, Clone)]
struct CorrectedSpec {
    /// 纠正结果是否带**错误的相关 id**（用来打 C16：管线必须把它钉回调用 id）。
    wrong_id: bool,
    /// 纠正结果的成功/失败形态。
    outcome: ResultSpec,
}

/// 一个纠错器的脚本。
#[derive(Debug, Clone)]
struct CorrectorSpec {
    /// 纠错器名（`decided_by` / `consulted` / 归档条目的 `hook` 里出现的就是它）。
    name: String,
    /// 脚本判定。
    verdict: PostVerdict,
}

/// 输出 schema 脚本（声明哪些字段、什么 kind）。
#[derive(Debug, Clone)]
struct SchemaSpec {
    /// 声明的必填字段。
    fields: Vec<(String, SchemaKind)>,
}

/// 一次待判定的场景。
struct Scenario {
    /// 纠错器脚本（登记顺序 = 咨询顺序）。
    correctors: Vec<CorrectorSpec>,
    /// 被执行的调用。
    call: ToolCall,
    /// 工具自身产出的结果形态。
    tool_result: ResultSpec,
    /// 工具能力身份。
    capability: CapabilityId,
    /// pre 段判定（`None` = 不挂 pre hook）。
    pre: Option<PreVerdict>,
    /// retry 臂的总尝试次数。
    retry_attempts: u32,
    /// 输出 schema（`None` = 不挂 schema）。
    schema: Option<SchemaSpec>,
    /// 专供 normalize 交互面用的纠正结果脚本。
    corrected: CorrectedSpec,
}

// ── 脚本纠错器 / pre hook / 工具桩 ──────────────────────────────────────────

/// 一个纠错器实际看到的事实快照。
#[derive(Debug, Clone, PartialEq)]
struct SeenFacts {
    call_id: String,
    tool_name: String,
    arguments: Value,
    result_render: String,
    result_is_ok: bool,
}

/// 一个按脚本回答的纠错器。
struct ScriptedCorrector {
    /// 登记序号（用来还原咨询顺序）。
    index: usize,
    /// 纠错器名。
    name: String,
    /// 脚本判定。
    verdict: PostVerdict,
    /// 被真正调用过的纠错器序号（按调用先后）。
    log: Arc<Mutex<Vec<usize>>>,
    /// 每个纠错器实际看到的事实快照。
    seen: Arc<Mutex<Vec<SeenFacts>>>,
}

impl PostExecuteHook for ScriptedCorrector {
    fn name(&self) -> &str {
        &self.name
    }

    fn post_verdict(&self, request: &PostExecuteRequest<'_>) -> PostVerdict {
        self.log.lock().unwrap().push(self.index);
        self.seen.lock().unwrap().push(SeenFacts {
            call_id: request.call.id.clone(),
            tool_name: request.call.name.clone(),
            arguments: request.call.arguments.clone(),
            result_render: request.result.render(),
            result_is_ok: request.result.is_ok(),
        });
        self.verdict.clone()
    }
}

/// 一个按脚本回答的 pre hook（只为打**交互面**，不重复 pre 段本身的判定）。
struct ScriptedPre {
    name: String,
    verdict: PreVerdict,
    log: Arc<Mutex<Vec<String>>>,
}

impl PreExecuteHook for ScriptedPre {
    fn name(&self) -> &str {
        &self.name
    }

    fn pre_verdict(&self, _request: &PreExecuteRequest<'_>) -> PreVerdict {
        self.log.lock().unwrap().push(self.name.clone());
        self.verdict.clone()
    }
}

/// 一个只计数的工具桩。
struct StubTool {
    id: CapabilityId,
    runs: AtomicUsize,
    outcome: ResultSpec,
}

impl StubTool {
    fn new(id: CapabilityId, outcome: ResultSpec) -> Self {
        Self {
            id,
            runs: AtomicUsize::new(0),
            outcome,
        }
    }

    fn runs(&self) -> usize {
        self.runs.load(Ordering::SeqCst)
    }

    /// 工具自身产出的结果（管线在 accept 路径上必须**原样**透出它）。
    fn result_for(&self, call: &ToolCall) -> ToolResult {
        build_result(&self.outcome, &call.id)
    }
}

/// 按脚本形态造一个结果（相关 id 固定为 `call_id`）。
fn build_result(spec: &ResultSpec, call_id: &str) -> ToolResult {
    match spec {
        ResultSpec::Ok(value) => ToolResult::ok(call_id, value.clone()),
        ResultSpec::Error(message) => ToolResult::permanent_error(call_id, message.clone()),
    }
}

/// 按脚本形态造一个**纠正结果**（可带错误的相关 id）。
fn build_corrected(spec: &CorrectedSpec, call_id: &str) -> ToolResult {
    let id = if spec.wrong_id {
        format!("{call_id}__wrong")
    } else {
        call_id.to_string()
    };
    build_result(&spec.outcome, &id)
}

/// `ToolCapability` 是 `#[async_trait]` 声明的；fuzz 子工作区**不引入新依赖**
/// ⇒ 这里手写 async-trait 的脱糖签名（生命周期名可任意取）。
impl ToolCapability for StubTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "fuzz_post_execute".into(),
            description: Some("fuzz stub for the post-execute correction channel".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    fn invoke<'a, 'b, 'async_trait>(
        &'a self,
        call: &'b ToolCall,
    ) -> Pin<Box<dyn Future<Output = ToolResult> + Send + 'async_trait>>
    where
        'a: 'async_trait,
        'b: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            self.runs.fetch_add(1, Ordering::SeqCst);
            self.result_for(call)
        })
    }
}

/// 最小 `block_on`（fuzz 子工作区没有 tokio 依赖 ⇒ 自己写一个）。
///
/// 只用于驱动**同步完成**的管线 future（工具桩立即返回；不装 timeout 臂，
/// 因为 `Deadline::after` 需要 tokio runtime handle）。
fn block_on<F: Future>(future: F) -> F::Output {
    struct Parker {
        thread: std::thread::Thread,
        notified: std::sync::atomic::AtomicBool,
    }

    impl Wake for Parker {
        fn wake(self: Arc<Self>) {
            self.notified.store(true, Ordering::SeqCst);
            self.thread.unpark();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            Arc::clone(self).wake();
        }
    }

    let parker = Arc::new(Parker {
        thread: std::thread::current(),
        notified: std::sync::atomic::AtomicBool::new(false),
    });
    let waker = Waker::from(Arc::clone(&parker));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => {
                parker.notified.store(false, Ordering::SeqCst);
                std::thread::park();
            }
        }
    }
}

// ── 期望值计算（独立于被测实现）──────────────────────────────────────────────

/// 按契约 C1 从脚本算出**期望的**合并结果。
///
/// C1（`post.rs:12-13`）：首个决断判定赢，后面的不再被咨询。
fn expected_merge(correctors: &[CorrectorSpec]) -> (PostVerdict, Option<String>, Vec<String>) {
    let mut consulted = Vec::new();
    for spec in correctors {
        consulted.push(spec.name.clone());
        if !spec.verdict.is_accept() {
            return (spec.verdict.clone(), Some(spec.name.clone()), consulted);
        }
    }
    (PostVerdict::Accept, None, consulted)
}

/// 按脚本建一条 post 通道，并挂上咨询日志与事实快照。
fn build_waterfall(
    correctors: &[CorrectorSpec],
    log: &Arc<Mutex<Vec<usize>>>,
    seen: &Arc<Mutex<Vec<SeenFacts>>>,
) -> PostExecuteWaterfall {
    let mut waterfall = PostExecuteWaterfall::new();
    for (index, spec) in correctors.iter().enumerate() {
        let hook = Arc::new(ScriptedCorrector {
            index,
            name: spec.name.clone(),
            verdict: spec.verdict.clone(),
            log: Arc::clone(log),
            seen: Arc::clone(seen),
        });
        let hook: Arc<dyn PostExecuteHook> = hook;
        waterfall = waterfall.with(hook);
    }
    waterfall
}

/// 按脚本装一条 schema。
fn build_schema(spec: &SchemaSpec) -> OutputSchema {
    let mut schema = OutputSchema::new();
    for (name, kind) in &spec.fields {
        schema = schema.require(name.clone(), *kind);
    }
    schema
}

/// JSON kind 判定（独立于被测的 `SchemaKind::matches`）。
fn kind_matches(kind: SchemaKind, value: &Value) -> bool {
    match kind {
        SchemaKind::String => value.is_string(),
        SchemaKind::Number => value.is_number(),
        SchemaKind::Boolean => value.is_boolean(),
        SchemaKind::Object => value.is_object(),
        SchemaKind::Array => value.is_array(),
    }
}

/// JSON kind 标签（独立于被测的私有 `kind_label`）。
fn kind_label(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// **独立复算**归一段的结果。
///
/// 依据 `contract.rs:150-153`「Every declared field is required with its exact
/// kind; fields not declared are left alone.」+ `contract.rs:212-216`
/// 「The structured output is not a JSON object at all.」⇒ 非对象即违反；
/// 字段按**声明顺序**检查，报**第一个**不符的字段（`contract.rs:189-204`）。
///
/// 返回值：
/// * `None` = **口径有歧义，不判定**（空 schema：见文件头「已知不断言的边界」）；
/// * `Some(Ok(()))` = 合规；
/// * `Some(Err(error))` = 违规，且给出**期望的**归一错误帧。
fn expect_normalization(
    spec: Option<&SchemaSpec>,
    value: &Value,
) -> Option<Result<(), NormalizationError>> {
    let Some(spec) = spec else {
        // 没挂 schema ⇒ 根本不校验（`contract.rs:76-80`）。
        return Some(Ok(()));
    };
    if spec.fields.is_empty() {
        return None;
    }
    let Some(object) = value.as_object() else {
        return Some(Err(NormalizationError::NotStructured {
            found: kind_label(value).to_string(),
        }));
    };
    for (name, kind) in &spec.fields {
        match object.get(name) {
            None => {
                return Some(Err(NormalizationError::SchemaMismatch {
                    field: name.clone(),
                    expected: kind.label().to_string(),
                    found: "missing".to_string(),
                }));
            }
            Some(found) if !kind_matches(*kind, found) => {
                return Some(Err(NormalizationError::SchemaMismatch {
                    field: name.clone(),
                    expected: kind.label().to_string(),
                    found: kind_label(found).to_string(),
                }));
            }
            Some(_) => {}
        }
    }
    Some(Ok(()))
}

// ── ① 通道面契约（C1 – C7）──────────────────────────────────────────────────

/// 通道面契约（C1 / C2 / C3 / C4 / C5 / C6 / C7）。
fn check_channel(scenario: &Scenario) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let waterfall = build_waterfall(&scenario.correctors, &log, &seen);

    let original = build_result(&scenario.tool_result, &scenario.call.id);
    let request = PostExecuteRequest::new(&scenario.call, &original);
    let decision = waterfall.evaluate(&request);

    let (want_verdict, want_decided_by, want_consulted) = expected_merge(&scenario.correctors);

    // C1：首个决断判定赢。
    assert_eq!(
        decision.verdict,
        want_verdict,
        "C1：合并判定与「首个决断判定即赢」不符: correctors={:?}",
        scenario
            .correctors
            .iter()
            .map(|c| (c.name.as_str(), c.verdict.label()))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        decision.decided_by, want_decided_by,
        "C1/C4：决断者与「首个决断判定所在的纠错器」不符"
    );
    assert_eq!(
        decision.consulted, want_consulted,
        "C1/C4：被咨询的纠错器名序列与「咨询到决断为止」不符"
    );

    // C1：决断之后**不再咨询**后面的纠错器（后到的 hook 不得翻转前一个的判定）。
    let invoked = log.lock().unwrap().clone();
    let expected_invoked: Vec<usize> = (0..want_consulted.len()).collect();
    assert_eq!(
        invoked, expected_invoked,
        "C1：被真正咨询的纠错器序号与「决断即停」不符"
    );

    // C2：空通道 ⇒ 一切原样接受。
    if scenario.correctors.is_empty() {
        assert!(decision.verdict.is_accept(), "C2：空通道却给出了非接受判定");
        assert_eq!(decision.decided_by, None, "C2：空通道的决断者必须缺席");
        assert!(
            decision.consulted.is_empty(),
            "C2：空通道不得咨询任何纠错器"
        );
        assert!(invoked.is_empty(), "C2：空通道不得调用任何纠错器");
    }

    // C4：`decided_by` 与 `consulted` 的最后一项必须一致；两者同现同灭。
    match &decision.decided_by {
        Some(hook) => {
            assert!(!decision.verdict.is_accept(), "C4：全部接受时不得有决断者");
            assert_eq!(
                decision.consulted.last().map(String::as_str),
                Some(hook.as_str()),
                "C4：决断者必须是最后被咨询的那个纠错器"
            );
        }
        None => assert!(decision.verdict.is_accept(), "C4：有决断判定却查不到决断者"),
    }

    // C3：登记顺序 = 咨询顺序。
    assert_eq!(
        waterfall.names(),
        scenario
            .correctors
            .iter()
            .map(|corrector| corrector.name.as_str())
            .collect::<Vec<_>>(),
        "C3：纠错器名序列与登记顺序不符"
    );
    assert_eq!(
        waterfall.len(),
        scenario.correctors.len(),
        "C3：纠错器计数与登记数不符"
    );
    assert_eq!(
        waterfall.is_empty(),
        scenario.correctors.is_empty(),
        "C3：空通道的判定与登记数不符"
    );

    // C5：标签与 `is_accept` 的口径。
    assert_eq!(
        decision.verdict.label(),
        want_verdict.label(),
        "C5：判定标签与期望不符"
    );
    assert_eq!(
        decision.verdict.is_accept(),
        decision.verdict.label() == "accept",
        "C5：is_accept 与标签口径不符"
    );
    assert!(
        decision.summary().contains(decision.verdict.label()),
        "C5：决策摘要必须含判定标签: summary={:?}",
        decision.summary()
    );

    // C6：请求事实只读 ⇒ 每个纠错器看到的**完全一致**。
    let observations = seen.lock().unwrap().clone();
    assert_eq!(
        observations.len(),
        invoked.len(),
        "C6：事实快照数与被咨询的纠错器数不符"
    );
    for facts in &observations {
        assert_eq!(
            facts.call_id, scenario.call.id,
            "C6：纠错器看到的 call id 与请求不符"
        );
        assert_eq!(
            facts.tool_name, scenario.call.name,
            "C6：纠错器看到的工具名与请求不符"
        );
        assert_eq!(
            facts.arguments, scenario.call.arguments,
            "C6：纠错器看到的入参与请求不符"
        );
        assert_eq!(
            facts.result_render,
            original.render(),
            "C6：纠错器看到的结果与请求不符"
        );
        assert_eq!(
            facts.result_is_ok,
            original.is_ok(),
            "C6：纠错器看到的结果成败与请求不符"
        );
    }
    if let Some(first) = observations.first() {
        for facts in &observations {
            assert_eq!(
                facts, first,
                "C6：后到的纠错器看到的事实与前面的不同（请求被改写了）"
            );
        }
    }

    // C7：确定性 —— 同一组 hook + 同一请求 ⇒ 两次结果相同。
    let again = waterfall.evaluate(&request);
    assert_eq!(decision, again, "C7：同一组 hook + 同一请求，两次结果不同");
}

/// 顺序面契约（C1 的顺序敏感面 + 顺序无关面）。
///
/// * **顺序敏感**：给定登记顺序，结果 = 该顺序上第一个决断判定。
/// * **顺序无关**：只要有一个决断判定，**任何排列**下结果必是决断。
fn check_order_faces(scenario: &Scenario) {
    if scenario.correctors.len() < 2 {
        return;
    }
    let original = build_result(&scenario.tool_result, &scenario.call.id);
    let request = PostExecuteRequest::new(&scenario.call, &original);
    let has_decisive = scenario
        .correctors
        .iter()
        .any(|corrector| !corrector.verdict.is_accept());

    for permutation in permutations(scenario.correctors.len().min(5)) {
        let specs: Vec<CorrectorSpec> = permutation
            .iter()
            .map(|index| scenario.correctors[*index].clone())
            .collect();

        let log = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::new(Mutex::new(Vec::new()));
        let waterfall = build_waterfall(&specs, &log, &seen);
        let decision = waterfall.evaluate(&request);
        let (want_verdict, want_decided_by, want_consulted) = expected_merge(&specs);

        // 顺序敏感面：结果由该排列上的「第一个决断判定」决定。
        assert_eq!(
            decision.verdict, want_verdict,
            "C1：排列后的合并判定与「该排列上首个决断判定」不符"
        );
        assert_eq!(
            decision.decided_by, want_decided_by,
            "C1：排列后的决断者不符"
        );
        assert_eq!(
            decision.consulted, want_consulted,
            "C1：排列后被咨询的纠错器名序列不符"
        );
        assert_eq!(
            *log.lock().unwrap(),
            (0..want_consulted.len()).collect::<Vec<_>>(),
            "C1：排列后被真正咨询的纠错器序号不符"
        );

        // 顺序无关面：**任一**排列下，只要有决断判定就必是决断。
        if has_decisive {
            assert!(
                !decision.verdict.is_accept(),
                "C1：存在决断判定，某个排列却把合并结果翻成了接受"
            );
        }
    }
}

// ── ② 管线端到端面（C8 – C16）──────────────────────────────────────────────

/// 管线端到端面契约。
fn check_pipeline(scenario: &Scenario) {
    let tool = StubTool::new(scenario.capability.clone(), scenario.tool_result.clone());
    let corrector_log = Arc::new(Mutex::new(Vec::new()));
    let corrector_seen = Arc::new(Mutex::new(Vec::new()));
    let pre_log = Arc::new(Mutex::new(Vec::new()));

    let mut pipeline = ToolExecutionPipeline::new();
    if let Some(verdict) = &scenario.pre {
        let hook = Arc::new(ScriptedPre {
            name: "pre".to_string(),
            verdict: verdict.clone(),
            log: Arc::clone(&pre_log),
        });
        let hook: Arc<dyn PreExecuteHook> = hook;
        pipeline = pipeline.with_pre_hook(hook);
    }
    // retry 臂（timeout 臂需要 tokio runtime ⇒ 不装；见文件头「已知不断言的边界」）。
    pipeline = pipeline
        .with_around(AroundPolicy::new().with_retry(RetryPolicy::new(scenario.retry_attempts)));
    for (index, spec) in scenario.correctors.iter().enumerate() {
        let hook = Arc::new(ScriptedCorrector {
            index,
            name: spec.name.clone(),
            verdict: spec.verdict.clone(),
            log: Arc::clone(&corrector_log),
            seen: Arc::clone(&corrector_seen),
        });
        let hook: Arc<dyn PostExecuteHook> = hook;
        pipeline = pipeline.with_post_hook(hook);
    }

    let executed = block_on(pipeline.run(&tool, &scenario.call));

    let original = tool.result_for(&scenario.call);
    let tool_is_ok = original.is_ok();
    let retry_attempts = RetryPolicy::new(scenario.retry_attempts).attempts();
    let expected_runs = if tool_is_ok {
        1
    } else {
        // retry 臂：失败就重试到 `attempts()` 次（`around.rs:161-163`）。
        retry_attempts as usize
    };

    // C11：pre 拒 ⇒ 什么都不执行 ⇒ post 段**从未被咨询**。
    if let Some(verdict) = &scenario.pre {
        if !verdict.is_allowed() {
            assert_eq!(tool.runs(), 0, "C11：被 pre 拒的调用仍然执行了工具");
            assert!(
                corrector_log.lock().unwrap().is_empty(),
                "C11：被 pre 拒的调用仍然咨询了 post 段"
            );
            assert_eq!(
                executed.record.stages(),
                vec![ExecutionStage::PreExecute],
                "C11/C12：被 pre 拒的调用必须停在 pre-execute 段"
            );
            assert!(
                executed.failure.is_some(),
                "C11：被 pre 拒的调用必须产出失败帧"
            );
            assert!(
                !executed.result.is_ok(),
                "C11：被 pre 拒的调用不得产出成功结果"
            );
            assert!(
                executed.record.superseded().is_empty(),
                "C11：被 pre 拒的调用不得产生归档条目（什么都没执行）"
            );
            return;
        }
    }

    // 工具**确实执行了**（post 段在执行之后才裁定 ⇒ 副作用已经发生）。
    assert_eq!(
        tool.runs(),
        expected_runs,
        "C8/C9：工具的执行次数与「retry 臂跑满 attempts」不符"
    );

    // C12：阶段顺序固定。
    assert_eq!(
        executed.record.stages(),
        ExecutionStage::ALL.to_vec(),
        "C12：放行的调用必须按固定顺序走完五段"
    );

    // post 段在一次运行里**只被咨询一次**，且咨询序列 = 「首个决断判定即停」。
    let (want_verdict, want_decided_by, want_consulted) = expected_merge(&scenario.correctors);
    assert_eq!(
        *corrector_log.lock().unwrap(),
        (0..want_consulted.len()).collect::<Vec<_>>(),
        "C1：管线里的 post 段咨询序列与「首个决断判定即停」不符"
    );

    match &want_verdict {
        // C8 / C2：`accept` ⇒ 结果原样透出（一个字节都不改）。
        PostVerdict::Accept => {
            assert_eq!(
                executed.result, original,
                "C2/C8：post 接受后结果必须与工具产出逐字相同"
            );
            assert!(
                executed.record.superseded().is_empty(),
                "C8：post 接受时不得产生归档条目"
            );
            if tool_is_ok {
                assert_eq!(
                    executed.failure, None,
                    "C8：工具成功 + post 接受时不得产出失败帧"
                );
            }
        }
        // C8：replace ⇒ 换入纠正结果，原结果进归档。
        PostVerdict::Replace { corrected, note } => {
            assert_eq!(
                executed.result.outcome, corrected.outcome,
                "C8：replace 之后发出的结果必须就是纠正结果"
            );
            // C16：相关 id 被钉回被回答的那个调用。
            assert_eq!(
                executed.result.tool_call_id, scenario.call.id,
                "C16：replace 之后的结果没有与调用 id 相关联"
            );
            let archived = executed.record.superseded();
            assert_eq!(
                archived.len(),
                1,
                "C8/C10：replace 必须恰好归档一条（原结果）"
            );
            assert_eq!(
                archived[0].stage,
                ExecutionStage::PostExecute,
                "C10：归档条目的阶段必须是 post-execute"
            );
            assert_eq!(
                archived[0].original, original,
                "C8/C10：被顶替的原结果必须逐字归档"
            );
            assert_eq!(
                archived[0].hook,
                want_decided_by.clone().unwrap_or_default(),
                "C10：归档条目的 actor 必须是决断的纠错器"
            );
            assert_eq!(archived[0].note, *note, "C10：归档条目必须带判定给出的理由");
            // C14：被顶替的那次失败不再是活失败。
            assert!(
                !matches!(
                    executed.failure,
                    Some(PipelineFailure::RetriesExhausted { .. })
                ),
                "C14：replace 之后仍留着被顶替结果的重试耗尽失败帧"
            );
        }
        // C9：block ⇒ 判为失败，原结果同样归档。
        PostVerdict::Block { reason } => {
            let source = want_decided_by.clone().unwrap_or_default();
            assert_eq!(
                executed.failure,
                Some(PipelineFailure::PostBlocked {
                    source: source.clone(),
                    reason: reason.clone(),
                }),
                "C9：block 必须产出 PostBlocked 失败帧"
            );
            assert!(!executed.result.is_ok(), "C9：block 之后不得产出成功结果");
            // C13 / C16：失败帧与调用 id 相关联、带稳定码。
            assert_eq!(
                executed.result.tool_call_id, scenario.call.id,
                "C13/C16：失败帧没有与调用 id 相关联"
            );
            let code = executed.failure.as_ref().expect("C9：失败帧存在").code();
            assert_eq!(code, "pipeline.post_block", "C13：失败码与失败族不符");
            let message = match &executed.result.outcome {
                WireToolOutcome::Error { message, .. } => message.clone(),
                other => panic!("C9：失败帧必须是错误结果: {other:?}"),
            };
            assert!(
                message.starts_with(&format!("{code}: ")),
                "C13：失败帧的消息必须以稳定码开头: {message}"
            );
            assert!(!executed.outcome.ok, "C13：失败帧的归一形状必须标为失败");
            let archived = executed.record.superseded();
            assert_eq!(
                archived.len(),
                1,
                "C9/C10：block 必须恰好归档一条（原结果）"
            );
            assert_eq!(
                archived[0].stage,
                ExecutionStage::PostExecute,
                "C10：归档条目的阶段必须是 post-execute"
            );
            assert_eq!(
                archived[0].original, original,
                "C9/C10：被打回的原结果必须逐字归档"
            );
            assert_eq!(
                archived[0].hook, source,
                "C10：归档条目的 actor 必须是决断的纠错器"
            );
            assert_eq!(
                archived[0].note, *reason,
                "C10：归档条目必须带判定给出的理由"
            );
        }
    }

    // C7（管线级）：同一组 hook + 同一请求 ⇒ 两次运行结果相同。
    let second_tool = StubTool::new(scenario.capability.clone(), scenario.tool_result.clone());
    let second_pipeline = rebuild_pipeline(scenario);
    let again = block_on(second_pipeline.run(&second_tool, &scenario.call));
    assert_eq!(
        executed.result, again.result,
        "C7：同一组 hook + 同一请求，两次管线运行的结果不同"
    );
    assert_eq!(
        executed.failure, again.failure,
        "C7：同一组 hook + 同一请求，两次管线运行的失败帧不同"
    );
    assert_eq!(
        executed.record, again.record,
        "C7：同一组 hook + 同一请求，两次管线运行的记录不同"
    );
    assert_eq!(
        executed.outcome, again.outcome,
        "C7：同一组 hook + 同一请求，两次管线运行的归一形状不同"
    );

    // 单一来源面：管线第 4 段用的必须**就是**它暴露出来的那条通道
    // （`mod.rs:381-383`「The post-execute channel in force.」），
    // 且阶段记录里的详情 = 那条通道的判定摘要（`mod.rs:472-475`）。
    let exposed = pipeline
        .post()
        .evaluate(&PostExecuteRequest::new(&scenario.call, &original));
    let stage_detail = executed
        .record
        .entries()
        .iter()
        .find(|entry| entry.stage == ExecutionStage::PostExecute)
        .map(|entry| entry.detail.clone())
        .expect("前置事实：记录里必有 post_execute 段");
    assert_eq!(
        stage_detail,
        exposed.summary(),
        "单一来源面：管线的 post 段判定与它暴露的那条通道对不上"
    );
    assert_eq!(
        exposed.verdict, want_verdict,
        "单一来源面：暴露出来的通道判定与脚本期望不符"
    );
    assert_eq!(
        exposed.decided_by, want_decided_by,
        "单一来源面：暴露出来的通道决断者与脚本期望不符"
    );
    assert!(
        stage_detail.contains(want_verdict.label()),
        "C5：post 段的阶段详情必须含判定标签: detail={stage_detail}"
    );
}

/// 按脚本重建一条等价管线（用于确定性对照）。
fn rebuild_pipeline(scenario: &Scenario) -> ToolExecutionPipeline {
    let mut pipeline = ToolExecutionPipeline::new();
    if let Some(verdict) = &scenario.pre {
        let hook = Arc::new(ScriptedPre {
            name: "pre".to_string(),
            verdict: verdict.clone(),
            log: Arc::new(Mutex::new(Vec::new())),
        });
        let hook: Arc<dyn PreExecuteHook> = hook;
        pipeline = pipeline.with_pre_hook(hook);
    }
    pipeline = pipeline
        .with_around(AroundPolicy::new().with_retry(RetryPolicy::new(scenario.retry_attempts)));
    for (index, spec) in scenario.correctors.iter().enumerate() {
        let hook = Arc::new(ScriptedCorrector {
            index,
            name: spec.name.clone(),
            verdict: spec.verdict.clone(),
            log: Arc::new(Mutex::new(Vec::new())),
            seen: Arc::new(Mutex::new(Vec::new())),
        });
        let hook: Arc<dyn PostExecuteHook> = hook;
        pipeline = pipeline.with_post_hook(hook);
    }
    pipeline
}

/// post 段与 normalize 段的交互面契约（C15）。
///
/// * 纠正结果**违反**声明的 schema ⇒ 产出 `ContractViolation` 失败帧，
///   且纠正结果本身进归档（`mod.rs:511-524`）；
/// * 纠正结果**符合** schema ⇒ 不产出合同违规。
fn check_contract_interaction(scenario: &Scenario) {
    let Some(schema_spec) = &scenario.schema else {
        return;
    };
    // 空 schema 的口径有歧义 ⇒ 不断言。
    if schema_spec.fields.is_empty() {
        return;
    }
    let tool = StubTool::new(scenario.capability.clone(), scenario.tool_result.clone());
    let mut pipeline = ToolExecutionPipeline::new();
    pipeline = pipeline
        .with_around(AroundPolicy::new().with_retry(RetryPolicy::new(scenario.retry_attempts)));
    pipeline = pipeline.with_output_schema(build_schema(schema_spec));

    // 只挂**一个** replace 纠错器（纠正结果由脚本给定）。
    let corrected = build_corrected(&scenario.corrected, &scenario.call.id);
    let hook = Arc::new(ScriptedCorrector {
        index: 0,
        name: "corrector".to_string(),
        verdict: PostVerdict::Replace {
            corrected: corrected.clone(),
            note: "repair".to_string(),
        },
        log: Arc::new(Mutex::new(Vec::new())),
        seen: Arc::new(Mutex::new(Vec::new())),
    });
    let hook: Arc<dyn PostExecuteHook> = hook;
    pipeline = pipeline.with_post_hook(hook);

    let executed = block_on(pipeline.run(&tool, &scenario.call));

    // 只有**成功**的结果才进 schema 校验（`contract.rs:67-71`）。
    let expectation = match &corrected.outcome {
        WireToolOutcome::Ok { value } => expect_normalization(Some(schema_spec), value),
        WireToolOutcome::Error { .. } => Some(Ok(())),
    };
    let Some(expectation) = expectation else {
        return;
    };

    match expectation {
        Err(expected_error) => {
            assert_eq!(
                executed.failure,
                Some(PipelineFailure::ContractViolation(expected_error.clone())),
                "C15：纠正结果违反声明的 schema，却没产出预期的合同违规失败帧"
            );
            assert!(!executed.result.is_ok(), "C15：合同违规不得被当成成功结果");
            let archived = executed.record.superseded();
            assert_eq!(
                archived.len(),
                2,
                "C15/C10：违反 schema 时必须归档两条（原结果 + 被顶替的纠正结果）"
            );
            assert_eq!(
                archived[1].stage,
                ExecutionStage::Normalize,
                "C15/C10：第二条归档的阶段必须是 normalize"
            );
            assert_eq!(
                archived[1].original.outcome, corrected.outcome,
                "C15/C10：被顶替的纠正结果必须逐字归档"
            );
        }
        Ok(()) => {
            assert!(
                !matches!(
                    executed.failure,
                    Some(PipelineFailure::ContractViolation(_))
                ),
                "C15：纠正结果符合 schema，却产出了合同违规失败帧"
            );
        }
    }
}

// ── ③ 固定对抗电池 ──────────────────────────────────────────────────────────

/// 固定对抗电池：把关键边界固定下来，避免它们退化成盲区。
fn battery(scenario: &Scenario) {
    let base_call = scenario.call.clone();
    let base_result = scenario.tool_result.clone();
    let base_capability = scenario.capability.clone();
    let base_corrected = scenario.corrected.clone();

    let mut base = Scenario {
        correctors: Vec::new(),
        call: base_call.clone(),
        tool_result: base_result.clone(),
        capability: base_capability.clone(),
        pre: None,
        retry_attempts: 1,
        schema: None,
        corrected: base_corrected.clone(),
    };

    // 空通道（C2）。
    check_channel(&base);
    check_pipeline(&base);

    // 阶段标签 / 规范序号口径（C12）。
    assert_eq!(
        ExecutionStage::PostExecute.as_str(),
        "post_execute",
        "C12：post-execute 段的标签不符"
    );
    assert_eq!(
        ExecutionStage::PostExecute.order(),
        3,
        "C12：post-execute 段的规范序号不符"
    );

    let ok_corrected = CorrectedSpec {
        wrong_id: false,
        outcome: ResultSpec::Ok(json!({ "summary": "fine" })),
    };

    // 单纠错器 × 三种判定（C1 / C8 / C9）。
    for kind in 0..3u8 {
        base.correctors = vec![CorrectorSpec {
            name: "solo".to_string(),
            verdict: verdict_of(kind, "r", &base_call.id, &ok_corrected),
        }];
        check_channel(&base);
        check_pipeline(&base);
        check_order_faces(&base);
    }

    // 「决断 → 决断」与「决断 → 接受」两种排列（C1 的不可翻转）。
    for order in [[0u8, 1u8], [1, 0]] {
        base.correctors = order
            .iter()
            .map(|kind| CorrectorSpec {
                name: format!("h{kind}"),
                verdict: verdict_of(*kind, "r", &base_call.id, &ok_corrected),
            })
            .collect();
        check_channel(&base);
        check_order_faces(&base);
        check_pipeline(&base);
    }

    // 三个纠错器混合（C1：第一个决断赢，后面的接受翻不掉）。
    base.correctors = vec![
        CorrectorSpec {
            name: "a".to_string(),
            verdict: PostVerdict::Accept,
        },
        CorrectorSpec {
            name: "b".to_string(),
            verdict: verdict_of(1, "no", &base_call.id, &ok_corrected),
        },
        CorrectorSpec {
            name: "c".to_string(),
            verdict: PostVerdict::Accept,
        },
    ];
    check_channel(&base);
    check_order_faces(&base);
    check_pipeline(&base);

    // 异常形状的判定（不得 panic 面）：空串、超长串、怪字符。
    for reason in [
        String::new(),
        "x".repeat(64 * 1024),
        "\u{0}\u{200B}\u{202E}换行\n".to_string(),
        "\u{FFFD}".to_string(),
    ] {
        for kind in 1..3u8 {
            base.correctors = vec![CorrectorSpec {
                name: "weird".to_string(),
                verdict: verdict_of(kind, &reason, &base_call.id, &ok_corrected),
            }];
            check_channel(&base);
            check_pipeline(&base);
        }
    }

    // 名字边界：空名 / 重名 / 怪字符（C4：决断者必须是**最后被咨询**的纠错器）。
    for name in CORRECTOR_NAMES {
        base.correctors = vec![
            CorrectorSpec {
                name: name.to_string(),
                verdict: PostVerdict::Accept,
            },
            CorrectorSpec {
                name: "decider".to_string(),
                verdict: verdict_of(2, "no", &base_call.id, &ok_corrected),
            },
        ];
        check_channel(&base);
        check_pipeline(&base);
    }

    // 纠正结果带**错误的相关 id** ⇒ 管线必须把它钉回调用 id（C16）。
    base.correctors = vec![CorrectorSpec {
        name: "corrector".to_string(),
        verdict: verdict_of(
            1,
            "repair",
            &base_call.id,
            &CorrectedSpec {
                wrong_id: true,
                outcome: ResultSpec::Ok(json!({ "summary": "fine" })),
            },
        ),
    }];
    check_pipeline(&base);

    // 纠正结果是**失败**结果（C8：换入的失败结果原样发出）。
    base.correctors = vec![CorrectorSpec {
        name: "corrector".to_string(),
        verdict: verdict_of(
            1,
            "repair",
            &base_call.id,
            &CorrectedSpec {
                wrong_id: false,
                outcome: ResultSpec::Error("corrected into a failure".to_string()),
            },
        ),
    }];
    check_pipeline(&base);

    // pre 拒 ⇒ post 从未被咨询（C11）。
    for verdict in [
        PreVerdict::Deny {
            reason: "no".to_string(),
        },
        PreVerdict::Cancel {
            reason: "going away".to_string(),
        },
        PreVerdict::Ask {
            reason: "human".to_string(),
        },
    ] {
        base.correctors = vec![CorrectorSpec {
            name: "blocker".to_string(),
            verdict: verdict_of(2, "no", &base_call.id, &ok_corrected),
        }];
        base.pre = Some(verdict);
        check_pipeline(&base);
        base.pre = None;
    }

    // retry 臂 × post 判定（C14：replace 之后重试耗尽不再是活失败）。
    base.correctors = vec![CorrectorSpec {
        name: "corrector".to_string(),
        verdict: verdict_of(1, "repair", &base_call.id, &ok_corrected),
    }];
    base.tool_result = ResultSpec::Error("tool failed".to_string());
    base.retry_attempts = 3;
    check_pipeline(&base);
    base.retry_attempts = 1;
    base.tool_result = base_result.clone();

    // normalize 交互面（C15）。
    let schema = SchemaSpec {
        fields: vec![("summary".to_string(), SchemaKind::String)],
    };
    for payload in PAYLOADS {
        let value: Value = serde_json::from_str(payload).expect("固定载荷必须是合法 JSON");
        base.schema = Some(schema.clone());
        base.corrected = CorrectedSpec {
            wrong_id: false,
            outcome: ResultSpec::Ok(value),
        };
        check_contract_interaction(&base);
    }
    base.schema = None;
}

/// 按 kind 造一个判定（kind 0 = 接受，1 = replace，2 = block）。
fn verdict_of(kind: u8, reason: &str, call_id: &str, corrected: &CorrectedSpec) -> PostVerdict {
    match kind % 3 {
        0 => PostVerdict::Accept,
        1 => PostVerdict::Replace {
            corrected: build_corrected(corrected, call_id),
            note: reason.to_string(),
        },
        _ => PostVerdict::Block {
            reason: reason.to_string(),
        },
    }
}

// ── 排列生成 ────────────────────────────────────────────────────────────────

/// 生成 `0..n` 的全部排列（n 很小，调用方已夹到 ≤ 5）。
fn permutations(n: usize) -> Vec<Vec<usize>> {
    let mut items: Vec<usize> = (0..n).collect();
    let mut out = Vec::new();
    permute(&mut items, 0, &mut out);
    out
}

fn permute(items: &mut Vec<usize>, start: usize, out: &mut Vec<Vec<usize>>) {
    if start == items.len() {
        out.push(items.clone());
        return;
    }
    for index in start..items.len() {
        items.swap(start, index);
        permute(items, start + 1, out);
        items.swap(start, index);
    }
}

// ── 字节流游标 ──────────────────────────────────────────────────────────────

/// 字节流游标（把 fuzz 字节当选择器用）。
struct Stream<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Stream<'a> {
    fn byte(&mut self) -> u8 {
        if self.data.is_empty() {
            return 0;
        }
        let byte = self.data[self.position % self.data.len()];
        self.position = self.position.wrapping_add(1);
        byte
    }

    fn flag(&mut self) -> bool {
        self.byte() & 1 == 0
    }

    /// 取一段文本（非法 UTF-8 走有损转换）。
    fn text(&mut self, max: usize) -> String {
        let start = self.position.min(self.data.len());
        let end = (start + max).min(self.data.len());
        let text = String::from_utf8_lossy(&self.data[start..end]).into_owned();
        self.position = end;
        text
    }
}

/// 由 fuzz 字节造一个 JSON 值（规模有界）。
fn json_from(stream: &mut Stream<'_>) -> Value {
    match stream.byte() % 6 {
        0 => Value::Null,
        1 => Value::Bool(stream.flag()),
        2 => Value::Number((stream.byte() as i64).into()),
        3 => Value::String(stream.text(48)),
        4 => {
            let mut map = serde_json::Map::new();
            for index in 0..(stream.byte() % 4) {
                map.insert(format!("k{index}"), Value::String(stream.text(16)));
            }
            Value::Object(map)
        }
        _ => Value::Array(
            (0..(stream.byte() % 4))
                .map(|index| Value::String(format!("v{index}")))
                .collect(),
        ),
    }
}

/// 由 fuzz 字节造一个结果形态。
fn result_spec(stream: &mut Stream<'_>) -> ResultSpec {
    if stream.byte() % 3 == 0 {
        ResultSpec::Error(stream.text(64))
    } else {
        ResultSpec::Ok(json_from(stream))
    }
}

/// 由 fuzz 字节造一个纠正结果脚本。
fn corrected_spec(stream: &mut Stream<'_>) -> CorrectedSpec {
    CorrectedSpec {
        wrong_id: stream.byte() % 2 == 0,
        outcome: result_spec(stream),
    }
}

/// 由 fuzz 字节造一个 schema 脚本。
fn schema_spec(stream: &mut Stream<'_>) -> SchemaSpec {
    let count = (stream.byte() % 3) as usize;
    let kinds = [
        SchemaKind::String,
        SchemaKind::Number,
        SchemaKind::Boolean,
        SchemaKind::Object,
        SchemaKind::Array,
    ];
    let fields = (0..count)
        .map(|index| {
            (
                match stream.byte() % 3 {
                    0 => "summary".to_string(),
                    1 => "count".to_string(),
                    _ => format!("k{index}"),
                },
                kinds[(stream.byte() as usize) % kinds.len()],
            )
        })
        .collect();
    SchemaSpec { fields }
}

/// 由 fuzz 字节造一个场景。
fn scenario_from(stream: &mut Stream<'_>) -> Scenario {
    let corrector_count = (stream.byte() % 6) as usize;
    let mut correctors = Vec::with_capacity(corrector_count);
    for index in 0..corrector_count {
        let kind = stream.byte() % 3;
        let reason = stream.text(96);
        let corrected = corrected_spec(stream);
        let name = match stream.byte() % 5 {
            0 => format!("corrector_{index}"),
            1 => String::new(),
            2 => stream.text(24),
            3 => CORRECTOR_NAMES[index % CORRECTOR_NAMES.len()].to_string(),
            _ => format!("corrector_{index}"),
        };
        let verdict = verdict_of(kind, &reason, "call_1", &corrected);
        correctors.push(CorrectorSpec { name, verdict });
    }

    let call = ToolCall {
        id: stream.text(32),
        name: stream.text(32),
        arguments: json_from(stream),
    };
    let capability_text = stream.text(32);
    let capability = CapabilityId::new(&capability_text)
        .unwrap_or_else(|_| CapabilityId::new(FALLBACK_CAPABILITY).expect("固定能力 id 必须合法"));

    let pre = match stream.byte() % 5 {
        0 => Some(PreVerdict::Allow),
        1 => Some(PreVerdict::Deny {
            reason: stream.text(32),
        }),
        2 => Some(PreVerdict::Cancel {
            reason: stream.text(32),
        }),
        3 => Some(PreVerdict::Ask {
            reason: stream.text(32),
        }),
        _ => None,
    };

    let schema = match stream.byte() % 3 {
        0 => Some(schema_spec(stream)),
        _ => None,
    };

    let corrected = corrected_spec(stream);

    Scenario {
        correctors,
        call,
        tool_result: result_spec(stream),
        capability,
        pre,
        retry_attempts: u32::from((stream.byte() % 4).max(1)),
        schema,
        corrected,
    }
}

// ── fuzz 入口 ───────────────────────────────────────────────────────────────

fuzz_target!(|data: &[u8]| {
    let mut stream = Stream { data, position: 0 };
    let scenario = scenario_from(&mut stream);
    let mode = stream.byte() % 4;

    match mode {
        // ① 通道面 + 管线面。
        0 => {
            check_channel(&scenario);
            check_pipeline(&scenario);
        }
        // ② 顺序面（顺序敏感 + 顺序无关）。
        1 => {
            check_channel(&scenario);
            check_order_faces(&scenario);
        }
        // ③ normalize 交互面。
        2 => {
            check_pipeline(&scenario);
            check_contract_interaction(&scenario);
        }
        // ④ 固定对抗电池。
        _ => {
            battery(&scenario);
        }
    }
});
