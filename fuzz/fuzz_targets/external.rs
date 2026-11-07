#![no_main]

//! fuzz: **外部工具调用面**（`ext.*` 工具族）—— 原始工具参数 → 限域校验 → argv。
//!
//! 被测面：`crates/capabilities/tools/src/external.rs`。
//! 这一族工具吃**未归一化**的原始参数（`object.get("host")` 这种裸字符串），
//! 做限域校验后拼成 argv 交给外部二进制。判据写错就是两类事故：
//!
//! * **限域绕过（SSRF 面）**：host 绕过 [`Scope`] → 连到未授权主机；
//! * **参数篡改 / argv 注入**：未归一化的参数直接进 argv → 选项注入、连错端口。
//!
//! ## 怎么在**不执行外部二进制**的前提下测到「校验层」
//!
//! 每个适配器都有 `with_binary`（测试用缝隙，external.rs:149-153 等）。
//! 这里把二进制置空：`ProcessRequest::new("")` ⇒ `validate_request` 先判
//! 「executable must not be empty」（process/mod.rs:776-781）⇒ **在 fork 之前**
//! 就失败 ⇒ 不 fork、不 exec、**不执行任何外部二进制**；错误经
//! `ExternalError::Spawn` 的 Display（external.rs:30-35）变成
//! `cannot start : executable must not be empty`。
//!
//! 于是「校验层是否放行」可以直接从结果消息读出来：
//! 消息以 `cannot start ` 开头 ⇒ 放行；否则 ⇒ 校验层拒绝。
//!
//! ## 为什么要把「限域面」与「参数类型面」拆成两段
//!
//! 实测踩过的坑（首版 harness 的错，不是库的错）：把 fuzz 出来的随机 host 与
//! 畸形 port 一起喂进去时，工具先被**限域**拒掉，于是「port 没被守门拦」
//! 这件事被限域的拒绝**掩盖**了 —— 断言照样通过，缺陷照样漏。
//! 所以两段必须**解耦**：
//! * **限域面**：只给 host（不给 port/sni）⇒ 放行/拒绝完全由 Scope 决定；
//! * **参数类型面**：host 固定成**范围内**的 ⇒ 放行/拒绝完全由参数类型决定。
//!
//! ## 判据来源（每条断言都取自被测代码自己写明的契约）
//!
//! * 模块文档 external.rs:1-9（`只走 ProcessExecutor，argv 数组、无 shell 插值`、
//!   `工具能力固定`、`网络目标过 Scope`、`文件目标限域到 workspace`）；
//! * 各工具 `declaration()` 里写明的 JSON schema（类型/范围/长度）；
//! * 实现自己的错误消息语义（`port out of range`、`scope refused: ...`、
//!   `target path is outside the workspace scope` 等）；
//! * 同 crate 兄弟工具的同款约定（`re.net.recon` 对非整数端口报
//!   `ports must be integers`，recon_tool.rs:529-531）。
//!
//! **不自己发明期望值**：断言要么是「实现自己声明的约束必须成立」，
//! 要么是「实现的判定必须与 Scope 的公开判定一致」（交叉验证）。

use std::future::Future;
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll, Waker};

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use libfuzzer_sys::fuzz_target;
use serde_json::{Map, Value, json};

use apeireth_tools_canonical::external::{ExifTool, NmapTool, OpenSslTool};
use apeireth_tools_canonical::recon::scope::Scope;

/// 进程边界缝隙：二进制置空 ⇒ 在 fork 之前失败 ⇒ 不执行任何外部二进制。
const EMPTY_BINARY: &str = "";

/// 范围内的 host（用于「参数类型面」：排除限域这个混淆项）。
const IN_SCOPE_HOST: &str = "example.com";

/// 范围内的 target（网段 127.0.0.1/32）。
const IN_SCOPE_TARGET: &str = "127.0.0.1";

/// 校验层的判定结果。
#[derive(Debug)]
enum Decision {
    /// 校验层拒绝（消息来自校验分支）。
    Rejected(String),
    /// 校验层放行，已到进程边界。
    Passed(String),
}

impl Decision {
    fn passed(&self) -> bool {
        matches!(self, Decision::Passed(_))
    }

    fn message(&self) -> &str {
        match self {
            Decision::Rejected(message) | Decision::Passed(message) => message,
        }
    }
}

/// 最小 `block_on`（不引新依赖）。
///
/// `ToolCapability::invoke` 是 async，但 `ext.*` 的执行体里没有真正的 await 点
/// （`run_external` 内部是同步的 `ProcessExecutor::execute`），一次 poll 即可拿到结果。
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = Box::pin(future);
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// 把工具结果分类成「校验拒绝」/「校验放行」。
fn classify(result: &ToolResult) -> Decision {
    let message = match &result.outcome {
        ToolOutcome::Error { message, .. } => message.clone(),
        ToolOutcome::Ok { value } => panic!("预期错误结果，却拿到 ok: {value}"),
    };
    if message.starts_with("cannot start ") {
        Decision::Passed(message)
    } else {
        Decision::Rejected(message)
    }
}

fn call(name: &str, arguments: &Value) -> ToolCall {
    ToolCall {
        id: "fuzz".to_string(),
        name: name.to_string(),
        arguments: arguments.clone(),
    }
}

// ── Scope fixture ───────────────────────────────────────────────────────────

/// 主 fixture：授权一个域名 + 两段网段。
fn main_scope() -> Scope {
    Scope {
        name: "fuzz".into(),
        authorized: true,
        paths: Vec::new(),
        domains: vec!["example.com".into()],
        cidrs: vec!["127.0.0.1/32".into(), "10.0.0.0/8".into()],
    }
}

/// 「域名白名单里塞了一个 IP 字面量」的 fixture。
///
/// `Scope::check_domain("127.0.0.1")` 会**放行**（4 个 label 全是数字 ⇒ 是裸域名，
/// scope.rs:132-143 与 121），所以若实现把 IP 字面量当域名放行，
/// 就是「网段白名单被域名白名单旁路」—— 这正是本面要钉死的路由。
fn ip_in_domain_scope() -> Scope {
    Scope {
        name: "fuzz-ip-in-domain".into(),
        authorized: true,
        paths: Vec::new(),
        domains: vec!["127.0.0.1".into()],
        cidrs: Vec::new(),
    }
}

// ── workspace fixture ───────────────────────────────────────────────────────

/// 本次运行独有的 workspace 根（guide §3.4：共享目录不许写固定文件名）。
fn workspace_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!("apeireth-fuzz-external-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    // 正控：根内的一个真实文件（限域守门不能把合法调用也拒了）。
    let _ = std::fs::write(root.join("inside.bin"), b"fuzz");
    root
}

// ── 判定辅助 ────────────────────────────────────────────────────────────────

fn assert_rejected(decision: &Decision, arguments: &Value, why: &str) {
    assert!(
        !decision.passed(),
        "{why}: 校验层放行了不该放行的参数 arguments={arguments} message={}",
        decision.message()
    );
}

fn assert_rejected_with(decision: &Decision, expected: &str, arguments: &Value) {
    assert!(
        !decision.passed(),
        "校验层放行了不该放行的参数 arguments={arguments} message={}",
        decision.message()
    );
    assert_eq!(
        decision.message(),
        expected,
        "拒绝理由与实现写明的错误消息不一致 arguments={arguments}"
    );
}

// ── ext.openssl ─────────────────────────────────────────────────────────────

fn openssl_decision(scope: &Scope, arguments: &Value) -> Decision {
    let tool = OpenSslTool::new(Arc::new(scope.clone())).with_binary(EMPTY_BINARY);
    classify(&block_on(tool.invoke(&call("ext.openssl", arguments))))
}

/// `ext.openssl` 的**限域面**：只给 host，不给 port/sni。
///
/// 契约出处：模块文档 external.rs:6「网络目标过 Scope」；
/// 实现 external.rs:1605-1613（IP 字面量走 `check_address`，其余走 `check_domain`）。
fn check_openssl_scope(scope: &Scope, host: &str) {
    let arguments = json!({ "host": host });
    let decision = openssl_decision(scope, &arguments);

    // oracle：用 Scope 的**公开判定**重算一遍 —— 实现的放行/拒绝必须与之一致。
    let scope_ok = match host.parse::<IpAddr>() {
        Ok(address) => scope.check_address(address).is_ok(),
        Err(_) => scope.check_domain(host).is_ok(),
    };
    assert_eq!(
        scope_ok,
        decision.passed(),
        "限域判定与 Scope 自身判定不一致: host={host:?} message={}",
        decision.message()
    );
    if !scope_ok {
        // 实现写明的越界理由（external.rs:1607）：「scope refused: {error}」
        assert!(
            decision.message().starts_with("scope refused: "),
            "越界目标的拒绝理由不是 scope refused: host={host:?} message={}",
            decision.message()
        );
    }
}

/// `ext.openssl` 的**参数类型面**：host 固定为范围内域名。
///
/// ⇒ 放行/拒绝完全由 port / sni 的类型与范围决定（排除限域这个混淆项）。
fn check_openssl_types(scope: &Scope, port: Option<Value>, sni: Option<Value>) {
    let mut object = Map::new();
    object.insert("host".to_string(), Value::String(IN_SCOPE_HOST.to_string()));
    if let Some(port) = port {
        object.insert("port".to_string(), port);
    }
    if let Some(sni) = sni {
        object.insert("sni".to_string(), sni);
    }
    let arguments = Value::Object(object);
    let decision = openssl_decision(scope, &arguments);

    // ── port 的类型与范围 ─────────────────────────────────────────────
    // 声明出处：external.rs:1577-1578
    //   「"port": {"type": "integer", "minimum": 1, "maximum": 65535}」
    // 守门出处：external.rs:1599-1603（`!(1..=65535).contains(&port)` → "port out of range"）
    // 兄弟约定：recon_tool.rs:529-531 对非整数端口报「ports must be integers」
    if let Some(port) = arguments.get("port") {
        match port.as_u64() {
            Some(number) if !(1..=65535).contains(&number) => {
                // 声明范围外 ⇒ 必须拒（0 / 65536 / u64::MAX 今天已正确拒）
                assert_rejected_with(&decision, "port out of range", &arguments);
                return;
            }
            Some(_) => {}
            None => {
                // 不是「u64 整数」：负数 / 浮点 / 字符串 / 布尔 / null / 数组 / 对象。
                // 声明写死 type=integer，且实现自己的端口守门就是按整数语义写的
                // ⇒ 必须拒，不能悄悄回落成默认 443（那等于「越界/畸形端口 → 连 443」，
                // 是 fail-open：调用方要的端口和实际连的端口不是一回事）。
                assert_rejected(&decision, &arguments, "port 非整数却未被拒");
                return;
            }
        }
    }

    // ── sni 的类型 ────────────────────────────────────────────────────
    // 声明出处：external.rs:1579-1580「"sni": {"type": "string"}」
    // 读取点：external.rs:1604（`object.get("sni").and_then(Value::as_str)`）
    if let Some(sni) = arguments.get("sni") {
        if !sni.is_string() {
            assert_rejected(&decision, &arguments, "sni 非字符串却未被拒");
            return;
        }
    }

    // 其余参数合法 ⇒ 必须放行到进程边界。
    assert!(
        decision.passed(),
        "合法参数被拒: arguments={arguments} message={}",
        decision.message()
    );
}

// ── ext.nmap ────────────────────────────────────────────────────────────────

fn nmap_decision(scope: &Scope, arguments: &Value) -> Decision {
    let tool = NmapTool::new(Arc::new(scope.clone())).with_binary(EMPTY_BINARY);
    classify(&block_on(tool.invoke(&call("ext.nmap", arguments))))
}

/// `ext.nmap` 的**限域面**：只给 target。
///
/// 契约出处：模块文档 external.rs:6；实现 external.rs:194-201。
fn check_nmap_scope(scope: &Scope, target: &str) {
    let arguments = json!({ "target": target });
    let decision = nmap_decision(scope, &arguments);

    // ── target 必须是具体 IP ──────────────────────────────────────────
    // 出处：external.rs:194-197「target must be a concrete IP address」
    // 声明出处：external.rs:171-172「IPv4/IPv6 address inside the authorized scope」
    let Ok(address) = target.parse::<IpAddr>() else {
        assert_rejected_with(
            &decision,
            "target must be a concrete IP address",
            &arguments,
        );
        return;
    };

    // oracle：IP 一律走**网段**白名单（`check_address`），不走域名白名单。
    let scope_ok = scope.check_address(address).is_ok();
    assert_eq!(
        scope_ok,
        decision.passed(),
        "限域判定与 Scope 自身判定不一致: target={target:?} message={}",
        decision.message()
    );
    if !scope_ok {
        // 实现写明的越界理由（external.rs:199）：「scope refused: {error}」
        assert!(
            decision.message().starts_with("scope refused: "),
            "越界目标的拒绝理由不是 scope refused: target={target:?} message={}",
            decision.message()
        );
    }
}

/// `ext.nmap` 的**参数类型面**：target 固定为范围内 IP。
///
/// ⇒ 放行/拒绝完全由 ports / service_detection / syn 的类型决定。
fn check_nmap_types(
    scope: &Scope,
    ports: Option<Value>,
    service_detection: Option<Value>,
    syn: Option<Value>,
) {
    let mut object = Map::new();
    object.insert(
        "target".to_string(),
        Value::String(IN_SCOPE_TARGET.to_string()),
    );
    if let Some(ports) = ports {
        object.insert("ports".to_string(), ports);
    }
    if let Some(service_detection) = service_detection {
        object.insert("service_detection".to_string(), service_detection);
    }
    if let Some(syn) = syn {
        object.insert("syn".to_string(), syn);
    }
    let arguments = Value::Object(object);
    let decision = nmap_decision(scope, &arguments);

    // ── ports 的类型与内容 ────────────────────────────────────────────
    // 声明出处：external.rs:173-174「"ports": {"type": "string", "maxLength": 128}」
    // 守门出处：external.rs:202-213（空串 / 超长 / 非 [0-9,-] 字符 ⇒ 拒）
    if let Some(ports) = arguments.get("ports") {
        let Some(ports) = ports.as_str() else {
            // 非字符串的 ports 被当成「没给 ports」⇒ 端口约束整个被跳过，
            // 扫描面反而**变宽**（nmap 默认端口集）—— fail-open。
            assert_rejected(&decision, &arguments, "ports 非字符串却未被拒");
            return;
        };
        if ports.is_empty()
            || ports.len() > 128
            || !ports
                .chars()
                .all(|ch| ch.is_ascii_digit() || ch == ',' || ch == '-')
        {
            assert_rejected_with(&decision, "ports must be digits, comma, dash", &arguments);
            return;
        }
    }

    // ── service_detection / syn 的类型 ────────────────────────────────
    // 声明出处：external.rs:175-178（两个都是 "type": "boolean"）
    // 读取点：external.rs:214-218（`as_bool().unwrap_or(默认值)`）
    for key in ["service_detection", "syn"] {
        if let Some(value) = arguments.get(key) {
            if !value.is_boolean() {
                assert_rejected(&decision, &arguments, "布尔参数类型不符却未被拒");
                return;
            }
        }
    }

    // 其余参数合法 ⇒ 必须放行到进程边界。
    assert!(
        decision.passed(),
        "合法参数被拒: arguments={arguments} message={}",
        decision.message()
    );
}

// ── 文件限域（ext.exiftool）─────────────────────────────────────────────────

fn exiftool_decision(root: &Path, arguments: &Value) -> Decision {
    let tool = ExifTool::new(root).with_binary(EMPTY_BINARY);
    classify(&block_on(tool.invoke(&call("ext.exiftool", arguments))))
}

/// 独立 oracle：按**词法**归一化相对路径，判断 `..` 是否越过根目录。
///
/// 实现走的是 `canonicalize`（跟随符号链接，external.rs:99-115），
/// oracle 走的是纯词法组件栈 —— 两条路径不同，互为交叉验证。
/// 只在「词法越界 ⇒ 必须拒」这个方向上断言（反方向不成立：
/// 根内的软链、不可解析的路径都可能被实现正当拒绝）。
fn escapes_root(raw: &str) -> bool {
    let path = Path::new(raw);
    if path.is_absolute() {
        return true;
    }
    let mut depth = 0i64;
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(_) => depth += 1,
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return true;
                }
            }
            Component::RootDir | Component::Prefix(_) => return true,
        }
    }
    false
}

/// `ext.exiftool` 的文件限域契约。
fn check_exiftool(root: &Path, arguments: &Value) {
    let decision = exiftool_decision(root, arguments);

    // ── 契约 1：参数必须是对象 ─────────────────────────────────────────
    // 出处：external.rs:460-466「ext.exiftool arguments must be an object」
    let Some(object) = arguments.as_object() else {
        assert_rejected_with(
            &decision,
            "ext.exiftool arguments must be an object",
            arguments,
        );
        return;
    };

    // ── 契约 2：path 必须是字符串 ──────────────────────────────────────
    // 出处：external.rs:467-470「ext.exiftool requires a string path」
    let Some(path) = object.get("path").and_then(Value::as_str) else {
        assert_rejected_with(&decision, "ext.exiftool requires a string path", arguments);
        return;
    };

    // ── 契约 3：文件目标限域到 workspace ──────────────────────────────
    // 出处：模块文档 external.rs:6「文件目标限域到 workspace」
    //       + 实现 external.rs:111-113「target path is outside the workspace scope」
    if escapes_root(path) {
        assert_rejected(&decision, arguments, "越出 workspace 的路径却未被拒");
    }
}

/// 软链逃逸：根内的软链指向根外 ⇒ 必须拒。
///
/// 实现走 `canonicalize`（跟随符号链接，external.rs:99-115）⇒ 软链目标在根外就被拒。
/// 词法 oracle 抓不到这一类（词法上它在根内），所以单独钉一条。
#[cfg(unix)]
fn check_symlink_escape(root: &Path) {
    let outside =
        std::env::temp_dir().join(format!("apeireth-fuzz-outside-{}", std::process::id()));
    let _ = std::fs::write(&outside, b"outside");
    let link = root.join("escape-link");
    let _ = std::fs::remove_file(&link);
    if std::os::unix::fs::symlink(&outside, &link).is_err() {
        return;
    }
    let arguments = json!({ "path": "escape-link" });
    let decision = exiftool_decision(root, &arguments);
    assert!(
        !decision.passed(),
        "指向 workspace 外的软链被放行: message={}",
        decision.message()
    );
    let _ = std::fs::remove_file(&link);
    let _ = std::fs::remove_dir_all(&outside);
}

// ── 输入构造 ────────────────────────────────────────────────────────────────

/// 造一个「类型各异」的参数值，覆盖参数类型面。
fn typed_value(byte: u8, text: &str) -> Value {
    match byte % 10 {
        0 => Value::Null,
        1 => Value::Bool(byte & 1 == 0),
        2 => json!(byte as i64),
        3 => json!(-(byte as i64) - 1),
        4 => json!(u64::MAX),
        5 => json!(byte as f64 / 2.0),
        6 => Value::String(text.to_string()),
        7 => json!([text.to_string()]),
        8 => json!({ "k": text }),
        _ => json!(i64::MIN),
    }
}

/// 端口取值面：0 / 1 / 443 / 65535 / 65536 / 超大值 / 负数 / 字符串 / 浮点 / 布尔 / null。
fn port_value(byte: u8) -> Value {
    match byte % 12 {
        0 => json!(0),
        1 => json!(1),
        2 => json!(443),
        3 => json!(65535),
        4 => json!(65536),
        5 => json!(u64::MAX),
        6 => json!(-1),
        7 => json!(-(byte as i64) - 1),
        8 => json!("443"),
        9 => json!(443.5),
        10 => Value::Bool(true),
        _ => Value::Null,
    }
}

// ── 内置对抗串清单（确定性回归网）──────────────────────────────────────────

/// host/target 侧的对抗串：选项注入、空白/控制字符、域名伪装、IP 变体、IDN、超长。
const ADVERSARIAL_HOSTS: &[&str] = &[
    "",
    " ",
    "\t",
    "\n",
    "\r\n",
    "\u{0}",
    "-x",
    "--help",
    "-connect",
    "example.com",
    "EXAMPLE.COM",
    ".example.com",
    "example.com.",
    " example.com ",
    "sub.example.com",
    "evil.com/.example.com",
    "evil.com#.example.com",
    "evil.com?.example.com",
    "evil.com@.example.com",
    "evil.com\\.example.com",
    "-\n\n...example.com",
    "example.com:443",
    "example.com/path",
    "example.com,evil.com",
    "example.com;evil.com",
    "127.0.0.1",
    "10.1.2.3",
    "192.168.1.1",
    "169.254.169.254",
    "0.0.0.0",
    "255.255.255.255",
    "::1",
    "::ffff:127.0.0.1",
    "fe80::1",
    "2001:db8::1",
    "0177.0.0.1",
    "2130706433",
    "0x7f000001",
    "127.1",
    "localhost",
    "\u{4f8b}\u{3048}.test",
    "'; rm -rf / #",
    "$(id)",
    "`id`",
    "a|b",
    "a&b",
    "a b",
    "a\tb",
];

/// path 侧的对抗串。
const ADVERSARIAL_PATHS: &[&str] = &[
    "",
    ".",
    "..",
    "../x",
    "a/../../x",
    "a/../b",
    "/etc/hostname",
    "/",
    "a\u{0}b",
    "a b",
    "a\nb",
    "-x",
    "--help",
    "./a",
    "a/./b",
];

/// 超长 host（缓存：battery 每次迭代都跑，别每次都分配 4KB）。
fn long_host() -> String {
    static VALUE: OnceLock<String> = OnceLock::new();
    VALUE.get_or_init(|| "a".repeat(4096)).clone()
}

/// 超长 sni（缓存：同上）。
fn long_sni() -> Value {
    static VALUE: OnceLock<Value> = OnceLock::new();
    VALUE
        .get_or_init(|| Value::String("s".repeat(65536)))
        .clone()
}

/// 跑一遍内置对抗串清单（每次迭代都跑，保证这些边界不会退化成盲区）。
fn battery() {
    let scope = main_scope();
    let root = workspace_root();

    for host in ADVERSARIAL_HOSTS {
        check_openssl_scope(&scope, host);
        check_nmap_scope(&scope, host);
    }

    // 长度面：超长 host（不在范围内 ⇒ 必须拒；scope oracle 也会算出一致结论）。
    let long_host = long_host();
    check_openssl_scope(&scope, &long_host);
    check_nmap_scope(&scope, &long_host);

    // 长度面：超长 sni 仍是**合法字符串**（声明 external.rs:1579-1580 没写 maxLength）
    // ⇒ 不得因为长就被拒（否则就是我方自己发明的期望）。
    check_openssl_types(&scope, Some(json!(443)), Some(long_sni()));

    for path in ADVERSARIAL_PATHS {
        let mut object = Map::new();
        object.insert("path".to_string(), Value::String((*path).to_string()));
        check_exiftool(&root, &Value::Object(object));
    }

    // 正控：根内的真实文件必须放行（限域守门不能把合法调用也拒了）。
    let decision = exiftool_decision(&root, &json!({ "path": "inside.bin" }));
    assert!(
        decision.passed(),
        "workspace 内的文件被拒: message={}",
        decision.message()
    );

    // 软链逃逸必须被拒。
    #[cfg(unix)]
    check_symlink_escape(&root);

    // ── 路由钉死：IP 字面量必须走网段白名单 ─────────────────────────
    // 「域名白名单里塞 IP 字面量」的 fixture：check_domain 会放行 "127.0.0.1"，
    // 但 Scope 的域名表只授权「域名」、网段表才授权「IP」
    // （scope.rs:22-27 的字段语义）⇒ 实现必须走 check_address ⇒ 必须拒。
    let ip_scope = ip_in_domain_scope();
    let decision = openssl_decision(&ip_scope, &json!({ "host": "127.0.0.1" }));
    assert!(
        !decision.passed(),
        "IP 字面量被域名白名单放行（应走网段白名单）: message={}",
        decision.message()
    );

    // 反向：网段表授权 127.0.0.1/32 ⇒ IP 字面量必须放行。
    let decision = openssl_decision(&scope, &json!({ "host": "127.0.0.1" }));
    assert!(
        decision.passed(),
        "网段白名单内的 IP 被拒: message={}",
        decision.message()
    );
}

fuzz_target!(|data: &[u8]| {
    let Some((selector, payload)) = data.split_first() else {
        return;
    };
    let scope = main_scope();
    let root = workspace_root();
    let text = String::from_utf8_lossy(payload).into_owned();

    match *selector % 5 {
        // ① 结构化参数面：fuzz 字节直接当 JSON 参数（覆盖 required/类型/未知键）。
        0 => {
            let arguments = match serde_json::from_slice::<Value>(payload) {
                Ok(value) => value,
                Err(_) => Value::String(text.clone()),
            };
            // 参数不是对象 ⇒ 必须按实现写明的理由拒。
            let Some(object) = arguments.as_object() else {
                let decision = openssl_decision(&scope, &arguments);
                assert_rejected_with(
                    &decision,
                    "ext.openssl arguments must be an object",
                    &arguments,
                );
                let decision = nmap_decision(&scope, &arguments);
                assert_rejected_with(
                    &decision,
                    "ext.nmap arguments must be an object",
                    &arguments,
                );
                check_exiftool(&root, &arguments);
                return;
            };
            // host / target / path 的字符串性。
            match object.get("host") {
                Some(Value::String(host)) => check_openssl_scope(&scope, host),
                Some(_) | None => {
                    let decision = openssl_decision(&scope, &arguments);
                    assert_rejected_with(
                        &decision,
                        "ext.openssl requires a string host",
                        &arguments,
                    );
                }
            }
            match object.get("target") {
                Some(Value::String(target)) => check_nmap_scope(&scope, target),
                Some(_) | None => {
                    let decision = nmap_decision(&scope, &arguments);
                    assert_rejected_with(
                        &decision,
                        "ext.nmap requires a string target",
                        &arguments,
                    );
                }
            }
            // 参数类型面：把 host/target 换成范围内的，只留类型各异的参数。
            check_openssl_types(
                &scope,
                object.get("port").cloned(),
                object.get("sni").cloned(),
            );
            check_nmap_types(
                &scope,
                object.get("ports").cloned(),
                object.get("service_detection").cloned(),
                object.get("syn").cloned(),
            );
            check_exiftool(&root, &arguments);
        }
        // ② ext.openssl 参数类型面（port/sni 的类型与范围）。
        1 => {
            let byte = payload.first().copied().unwrap_or(0);
            let sni_byte = payload.get(1).copied().unwrap_or(0);
            check_openssl_types(
                &scope,
                Some(port_value(byte)),
                Some(typed_value(sni_byte, &text)),
            );
        }
        // ③ ext.nmap 参数类型面（ports/布尔）。
        2 => {
            let byte = payload.first().copied().unwrap_or(0);
            let sd = payload.get(1).copied().unwrap_or(0);
            let syn = payload.get(2).copied().unwrap_or(0);
            check_nmap_types(
                &scope,
                Some(typed_value(byte, "22,80")),
                Some(typed_value(sd, "true")),
                Some(typed_value(syn, "false")),
            );
        }
        // ④ 文件限域面：path 的词法越界。
        3 => {
            let mut object = Map::new();
            object.insert("path".to_string(), Value::String(text.clone()));
            check_exiftool(&root, &Value::Object(object));
        }
        // ⑤ 内置对抗串清单。
        _ => battery(),
    }
});
