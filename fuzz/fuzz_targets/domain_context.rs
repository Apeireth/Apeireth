#![no_main]

//! fuzz: **域门自称域判定面** (`domain_context::check_domain_claim`)。
//!
//! 判的是**跨域越权 / 冒充**的判定面: 客户端自称的域 (`x-apeireth-domain`
//! 头 / `?domain=` 查询, 口径唯一) 与服务端已登记的域上下文是否一致。
//!
//! ## 契约 (全部取自被测函数**自己写明的文档**, 不自造期望)
//!
//! 被测函数的文档 (`crates/adapters/gateway/src/domain_context.rs:312-315`):
//!
//! ```text
//! 自称域校验 (域级点名): 头 `x-apeireth-domain` 与查询 `?domain=` 同一语义
//! (口径唯一) —— 自称必须与解析域逐字一致, 否则 403 `domain_forbidden`
//! **恒定帧** (目标域存在与否同帧, 不泄露他域存在); 域面未接线时只有默认域
//! 可自称 (无域绑定可校验, 自称即事实)。
//! ```
//!
//! 由此派生出四条**可判**的断言:
//!
//! | # | 断言 | 出处 |
//! |---|---|---|
//! | C1 | 自称必须与解析域**逐字一致**: 逐字相等 ⇒ 放行; 不等 ⇒ 拒 | `domain_context.rs:312-315` |
//! | C2 | 拒帧**恒定**: 403 `domain_forbidden`, 形态不随自称值变化 | `domain_context.rs:26` (越权语义表) + `:107-109` + `:143-145` |
//! | C3 | 域面未接线 ⇒ **只有默认域可自称** | `domain_context.rs:314-315` |
//! | C4 | 登记域 id 的词面 = `DomainUserId` 目录安全校验 (1..=64 字节, `[a-z0-9_-]`, 首字符小写字母或数字) | `crates/foundation/core/src/user_domain.rs:70-111` |
//!
//! C4 说清一件要紧事: **本面的「域」不是 DNS 名**, 是目录安全的 id 词面。
//! 所以「后缀像不像」「裸域名 label 合不合法」**不在**本面契约内 —— 本面的
//! 契约是**逐字一致**。伪装族 (后缀挂靠 / 用户信息 / 端口 / 路径 / 片段 /
//! 查询 / 大小写 / 前后点 / IDN 同形 / 超长) 全部落在「不等 ⇒ 拒」这一条里,
//! 由 [`masquerade_vectors`] 逐条显式钉住。
//!
//! ## 输入为什么要「拧」
//!
//! 放行支路要求自称**逐字**等于登记域 id (≥8 字节的字符串) —— 盲 fuzz 几乎
//! 猜不中, 放行支路会长期不覆盖。故输入拆成「登记域样本 + 变异指令」:
//! 变异算子含**恒等**算子 (打到放行支路) 与各类伪装算子 (打到拒支路),
//! 另留一路「任意攻击者原文」(含非 UTF-8 字节 → 有损替换)。
//!
//! ## 已知边界 (诚实交代)
//!
//! 响应体**逐字**恒定这一条, 本 target 断不了: 取响应体要
//! `axum::body::to_bytes` (async) ⇒ 要往 fuzz 子 workspace 加 `axum` 这个
//! **外部依赖**, 违反「不引新外部依赖」的纪律。故本 target 断言的是拒帧的
//! **状态码 + content-type** 与常量帧同形; 响应体逐字恒定由产品测试钉住
//! (`crates/adapters/gateway/tests/user_domain_isolation.rs:303`
//! `a_claimed_domain_refusal_never_leaks_target_existence`)。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use apeireth_core::user_domain::{DEFAULT_DOMAIN_ID, DomainRole, DomainUserId, UserDomain};
use apeireth_gateway::domain_context::{
    DOMAIN_FORBIDDEN_CLAIM_MESSAGE, DomainContext, check_domain_claim, domain_forbidden_frame,
};
use apeireth_gateway::{GatewayServices, GatewayState, build_gateway_state_with_services};
use libfuzzer_sys::fuzz_target;

/// 真实调用点的端点 (自称域校验的两个调用面: 头面 = 请求路径, 查询面 = 事件面)。
const ENDPOINTS: &[&str] = &["/v1/apeireth/events", "/v1/sessions/s-1/settings"];

/// 拒帧的可观察形态 (状态码 + content-type)。
///
/// 之所以用宏而不是函数: 响应体类型 (`axum::response::Response`) 在本
/// 子 workspace 里**不可命名** (引 `axum` = 引新外部依赖), 而类型推导
/// 不需要写出类型名。
macro_rules! frame_shape {
    ($response:expr) => {{
        let response = $response;
        (
            response.status().as_u16(),
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok())
                .map(str::to_string),
        )
    }};
}

/// 极简同步阻塞器 (只用 std, 不引新依赖)。
///
/// 用途只有两处: ① 装配运行时的 `Runtime::builder().build()`; ② 定期把
/// 观测攒批面 flush 掉 (免得拒帧审计行在 fuzz 进程里无界堆积)。
/// 这两条路径在「无插件 / 无归档端口」的装配下**首次 poll 即 Ready**
/// (`build()` 的唯一 await 点是空插件表的 `start_all`), 不需要 reactor,
/// 因此一个 no-op waker 的轮询循环就够。
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::pin::pin;
    use std::sync::Arc as StdArc;
    use std::task::{Context, Poll, Wake, Waker};

    struct NoopWaker;
    impl Wake for NoopWaker {
        fn wake(self: StdArc<Self>) {}
    }

    let waker = Waker::from(StdArc::new(NoopWaker));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    for _ in 0..100_000 {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
        std::thread::yield_now();
    }
    panic!("装配期 future 超过轮询上限仍未完成 (该路径本应无挂起)");
}

/// 一份可用的网关装配 (域面未接线 = 兼容路径)。
///
/// `check_domain_claim` 只用到 `state.observations` (拒帧审计行), 但
/// `GatewayState` 是整体装配面 —— 故这里装一份最小可用的。
fn gateway_state() -> &'static GatewayState {
    static STATE: OnceLock<GatewayState> = OnceLock::new();
    STATE.get_or_init(|| {
        let runtime = Arc::new(
            block_on(apeireth_runtime::canonical::Runtime::builder().build()).expect("运行时装配"),
        );
        build_gateway_state_with_services(runtime, GatewayServices::default())
    })
}

/// 已登记的域上下文样本 (服务端侧登记的域)。
///
/// 登记域 id 的词面 = `DomainUserId::new` 的目录安全校验
/// (`crates/foundation/core/src/user_domain.rs:70-111`): 1..=64 字节、
/// `[a-z0-9_-]`、首字符小写字母或数字。样本刻意覆盖: 默认域 / 连字符 / 下划线 /
/// 单字符 / 数字开头 / 连续 `--` / 上界 64 字节。
fn registered_contexts() -> Vec<(DomainContext, Option<String>)> {
    let mut ids: Vec<String> = vec![
        DEFAULT_DOMAIN_ID.to_string(),
        "user-alice".to_string(),
        "user_alice".to_string(),
        "user-bob".to_string(),
        "a".to_string(),
        "0".to_string(),
        "a--b".to_string(),
        "x".repeat(64),
    ];
    ids.push("9".repeat(64));
    ids.into_iter()
        .map(|id| {
            let user_id =
                DomainUserId::new(id.clone()).expect("登记域 id 必须过目录安全校验 (C4 的词面)");
            let domain = UserDomain::new(user_id, "Fuzz Domain", 0, DomainRole::Owner)
                .expect("域文档字段必须合法");
            (
                DomainContext {
                    domain: Some(domain),
                },
                Some(id),
            )
        })
        .collect()
}

/// 登记域样本 (进程内只装一次: 每轮重建纯属浪费)。
fn registered_contexts_cached() -> &'static [(DomainContext, Option<String>)] {
    static CONTEXTS: OnceLock<Vec<(DomainContext, Option<String>)>> = OnceLock::new();
    CONTEXTS.get_or_init(registered_contexts)
}

/// 域面未接线的上下文 (无域绑定可校验)。
fn unbound_context() -> (DomainContext, Option<String>) {
    (DomainContext::default(), None)
}

/// 契约判定 (独立写法, 不抄被测实现的分支形状):
/// 自称必须与解析域**逐字一致** (C1); 域面未接线时只有默认域可自称 (C3)。
fn verdict(bound: Option<&str>, claimed: &str) -> bool {
    match bound {
        Some(id) => claimed.as_bytes() == id.as_bytes(),
        None => claimed.as_bytes() == DEFAULT_DOMAIN_ID.as_bytes(),
    }
}

/// 断言一次自称域判定符合契约。
fn assert_verdict(
    state: &GatewayState,
    context: &DomainContext,
    bound: Option<&str>,
    claimed: &str,
    endpoint: &str,
) {
    // 前置自检: 上下文的域 id 与登记样本一致
    // (`DomainContext::user_id` 的文档: 「当前域 id (域面未接线 = None)」,
    //  crates/adapters/gateway/src/domain_context.rs:136-139)。
    assert_eq!(
        context.user_id(),
        bound,
        "上下文的域 id 与登记样本不一致 (harness 自检)"
    );

    let expected_ok = verdict(bound, claimed);
    match check_domain_claim(state, endpoint, claimed, context) {
        Ok(()) => assert!(
            expected_ok,
            "自称域与本域**不一致**却放行 (跨域冒充 / 限域绕过): \
             claimed={claimed:?} bound={bound:?}"
        ),
        Err(response) => {
            assert!(
                !expected_ok,
                "自称域与本域**逐字一致**却遭拒 (可用性回归): \
                 claimed={claimed:?} bound={bound:?}"
            );
            // C2: 恒定拒帧 —— 状态码 403, 形态与常量帧同形。
            let shape = frame_shape!(&response);
            let reference = frame_shape!(&domain_forbidden_frame(DOMAIN_FORBIDDEN_CLAIM_MESSAGE));
            assert_eq!(
                shape, reference,
                "拒帧形态必须是恒定的 domain_forbidden 帧 (不随自称值变化)"
            );
            assert_eq!(shape.0, 403, "拒帧状态码必须是 403 (domain_forbidden)");
        }
    }
}

/// 伪装族向量 (攻击者可控自称域的常见伪装形态)。
///
/// 全部落在 C1 的反面: **不是**逐字等于本域的形态, 一律必须被拒。
/// 词面覆盖: 后缀挂靠 / 用户信息 / 端口 / 路径 / 片段 / 查询 / 大小写 /
/// 前后点 / 空白 / 控制字符 / NUL / 子串 vs 标签 / IDN 同形 / 超长 /
/// 空声明 / DNS 形态 (裸域名 label 合法性 —— 攻击者会这么自称)。
fn masquerade_vectors(bound: &str) -> Vec<String> {
    vec![
        // ── 后缀挂靠 (域名尾巴拼到别的主机上) ─────────────────────────────
        format!("{bound}.evil.com"),
        format!("evil.com.{bound}"),
        format!("evil.com/{bound}"),
        format!("evil.com#{bound}"),
        format!("evil.com?{bound}"),
        format!("evil.com\\{bound}"),
        format!("evil.com%2f{bound}"),
        format!("{bound}.evil.com"),
        // ── 用户信息 / 端口 ───────────────────────────────────────────────
        format!("user@{bound}"),
        format!("{bound}@evil.com"),
        format!("{bound}:443"),
        format!("{bound}:"),
        // ── 路径 / 片段 / 查询 ────────────────────────────────────────────
        format!("{bound}/"),
        format!("{bound}/x"),
        format!("{bound}#x"),
        format!("{bound}?x"),
        format!("{bound}?domain={bound}"),
        // ── 大小写 / 前后点 / 空白 / 控制字符 ──────────────────────────────
        bound.to_ascii_uppercase(),
        format!(".{bound}"),
        format!("{bound}."),
        format!("..{bound}"),
        format!("{bound}.."),
        format!(" {bound}"),
        format!("{bound} "),
        format!("\t{bound}"),
        format!("{bound}\n"),
        format!("{bound}\r\n"),
        format!("{bound}\0"),
        format!("{bound}\u{7f}"),
        // ── 子串 vs 标签 (标签边界: `notexample.com` 不得命中 `example.com`) ─
        format!("not{bound}"),
        format!("{bound}not"),
        format!("x.{bound}"),
        format!("{bound}.x"),
        // ── IDN / 同形异码 / 双向控制符 ───────────────────────────────────
        format!("xn--{bound}"),
        format!("{bound}。evil"),
        format!("{bound}．evil"),
        format!("\u{202e}{bound}"),
        format!("{bound}evil\u{202e}"),
        // ── 空 / 超长 / 纯数字 ────────────────────────────────────────────
        String::new(),
        "a".repeat(64),
        "a".repeat(65),
        "123".to_string(),
        "0".to_string(),
        // ── 未登记域 (自称一个服务端没登记的域) ───────────────────────────
        "nobody".to_string(),
        "no-such-domain".to_string(),
        "user-nobody-9f3a".to_string(),
        // ── DNS 形态 (裸域名 label 合法性: 空 label / `-` 起止 / 连续 `--`) ──
        "evil..com".to_string(),
        "-evil.com".to_string(),
        "evil-.com".to_string(),
        "ev--il.com".to_string(),
        ".example.com".to_string(),
        "example.com.".to_string(),
    ]
}

/// 归一化等价但**非逐字一致**的形态 (大小写 / 前后点 / 空白)。
///
/// C1 说的是「逐字一致」⇒ 任何归一化 (大小写折叠 / 去空白 / 去前后点)
/// 都**不得**被当成同一个域。这正是「后缀伪装那一族」在另一条路径上的变体:
/// 实现若偷偷做归一化, 自称 `USER-ALICE` / `.user-alice` / `user-alice.`
/// 就会被当成 `user-alice` 放行 —— 属跨域冒充。
fn normalization_variants(bound: &str) -> Vec<String> {
    vec![
        bound.to_ascii_lowercase(),
        bound.to_ascii_uppercase(),
        format!(".{bound}"),
        format!("{bound}."),
        format!(" {bound}"),
        format!("{bound} "),
        format!("\n{bound}"),
        format!("{bound}\n"),
        format!("{bound}\0"),
    ]
}

/// 一次性静态对抗批: 每个登记域 × 伪装族 × 归一化族, 全部必须被拒。
fn static_adversarial_batch(state: &GatewayState, contexts: &[(DomainContext, Option<String>)]) {
    for (context, bound) in contexts {
        let Some(id) = bound.as_deref() else {
            continue;
        };
        for masquerade in masquerade_vectors(id) {
            if masquerade == id {
                continue;
            }
            assert_verdict(state, context, bound.as_deref(), &masquerade, ENDPOINTS[0]);
        }
        for variant in normalization_variants(id) {
            if variant == id {
                continue;
            }
            assert_verdict(state, context, bound.as_deref(), &variant, ENDPOINTS[1]);
        }
    }

    // C3 (域面未接线 ⇒ 只有默认域可自称) 的静态钉:
    // 「默认域可自称」放行, 其余 (他域 / 空 / 大小写 / 前后点) 一律拒。
    let (unbound, none) = unbound_context();
    for accepted in [DEFAULT_DOMAIN_ID] {
        assert_verdict(state, &unbound, none.as_deref(), accepted, ENDPOINTS[0]);
    }
    for refused in [
        "",
        "Default",
        "DEFAULT",
        "default.",
        ".default",
        " default",
        "user-alice",
        "user_alice",
        "nobody",
        "default/evil",
        "default:443",
    ] {
        assert_verdict(state, &unbound, none.as_deref(), refused, ENDPOINTS[1]);
    }
}

/// 变异算子表 (id → 语义)。0 号是**恒等** —— 它是放行支路的入口。
const OPS: u8 = 12;

/// 自称域的构造: 「登记域 id」+ 「攻击者可控的变异指令」。
///
/// 恒等算子让放行支路可覆盖; 其余算子把攻击者能做的伪装一一做出来
/// (追加 / 前置 / 改写 / 截断 / 切尾 / 大小写 / 插入 / 删除 / 挂后缀 /
/// 整段原文)。
fn mutate(bound: &str, instruction: &[u8]) -> String {
    if instruction.is_empty() {
        return bound.to_string();
    }
    let op = instruction[0] % OPS;
    let arg = instruction.get(1).copied().unwrap_or(0);
    let mut bytes = bound.as_bytes().to_vec();
    match op {
        0 => {}
        1 => bytes.push(arg),
        2 => bytes.insert(0, arg),
        3 => {
            if !bytes.is_empty() {
                let index = arg as usize % bytes.len();
                bytes[index] = arg;
            }
        }
        4 => {
            let keep = arg as usize % (bytes.len() + 1);
            bytes.truncate(keep);
        }
        5 => {
            let keep = arg as usize % (bytes.len() + 1);
            bytes.drain(keep..);
        }
        6 => bytes = bytes.to_ascii_lowercase(),
        7 => bytes = bytes.to_ascii_uppercase(),
        8 => {
            if !bytes.is_empty() {
                let index = arg as usize % bytes.len();
                bytes.insert(index, arg);
            }
        }
        9 => {
            if !bytes.is_empty() {
                let index = arg as usize % bytes.len();
                bytes.remove(index);
            }
        }
        10 => {
            const SUFFIXES: &[&str] = &[
                ".evil.com",
                "/",
                "?",
                "#",
                ":443",
                "@evil.com",
                ".",
                " ",
                "\n",
                "\0",
                "%0a",
                "\u{3002}",
                "..",
                "-",
                "\u{202e}",
                "\u{7f}",
            ];
            bytes.extend_from_slice(SUFFIXES[arg as usize % SUFFIXES.len()].as_bytes());
        }
        _ => {
            // 任意攻击者原文 (含非 UTF-8 字节 → 有损替换, 不 panic)。
            return String::from_utf8_lossy(&instruction[1..]).into_owned();
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fuzz_target!(|data: &[u8]| {
    let state = gateway_state();
    let contexts = registered_contexts_cached();
    let unbound = unbound_context();

    // 一次性静态对抗批 (伪装族 × 全部登记域)。
    static STATIC_DONE: OnceLock<()> = OnceLock::new();
    STATIC_DONE.get_or_init(|| static_adversarial_batch(state, &contexts));

    if data.is_empty() {
        return;
    }

    // 输入切分: 第 1 字节选登记域样本, 其余是变异指令。
    let selector = data[0] as usize;
    let instruction = &data[1..];
    let (context, bound) = &contexts[selector % contexts.len()];
    let bound_id = bound.as_deref().expect("样本登记域必有域 id");

    // ① 主契约 (C1): 自称必须与解析域逐字一致。
    let claimed = mutate(bound_id, instruction);
    assert_verdict(state, context, bound.as_deref(), &claimed, ENDPOINTS[0]);

    // ② 域面未接线分支 (C3): 只有默认域可自称。
    assert_verdict(state, &unbound.0, None, &claimed, ENDPOINTS[0]);

    // ③ 整段原文当自称值 (任意字节空间 → 有损 UTF-8): 空 / 畸形 / 非 ASCII。
    let raw = String::from_utf8_lossy(instruction);
    assert_verdict(state, context, bound.as_deref(), &raw, ENDPOINTS[1]);
    assert_verdict(state, &unbound.0, None, &raw, ENDPOINTS[1]);

    // 观测攒批面定期 flush (无 tokio runtime ⇒ 无 flush 循环 ⇒ 审计行会无界堆积)。
    static ROUNDS: AtomicU64 = AtomicU64::new(0);
    if ROUNDS.fetch_add(1, Ordering::Relaxed) % 512 == 0 {
        block_on(state.observations.flush());
    }
});
