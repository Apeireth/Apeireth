#![no_main]
// 仓根 `clippy.toml` 禁 `std::fs::write` / `create_dir_all` 等直调 —— 那条规矩是给
// **产品代码**用的（写盘必须走原子写助手，见 `storage_atomic`）。fuzz harness 的
// 职责恰恰是**真的落盘**去喂被测工具的 IO 路径，故在此显式豁免并说明理由。
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

//! fuzz: **文件系统工具面**（`capabilities/tools::filesystem::FilesystemTool`）。
//!
//! **面**：工作区限域的只读文件工具（`read` / `list` / `stat`）。攻击面：
//! 路径逃逸（`..` / 绝对路径 / symlink 跳板）、凭据面读取、读上限、列目录截断、
//! 路径解析歧义（Unicode 归一化 / 分隔符混用 / 超长路径）。
//!
//! **判据全部来自被测模块自己写明的契约**（出处逐条标在断言旁的 `src:` 注释里）：
//!
//! - **限域**：`filesystem.rs` 模块头 —— "Path resolution is canonicalized and
//!   checked for containment inside that root before any operation, so `..` and
//!   symlink traversal do not escape the root for the operations implemented
//!   here."（`src: crates/capabilities/tools/src/filesystem.rs:3-6`）
//!   ⇒ 落在根外的路径必须被拒；成功结果里不得出现根外正文（canary）或根的绝对路径。
//! - **只读**：模块头 "Read-only filesystem tool capability." +
//!   "Write/delete/rename/copy are deliberately not implemented in M2A."
//!   （`src: filesystem.rs:1` / `src: filesystem.rs:13-15`）
//!   ⇒ 任何调用前后工作区快照必须逐项相同；写类操作词必须被拒。
//! - **读上限**：`DEFAULT_MAX_FILE_SIZE` 文档 "Default maximum file size for
//!   `read` (1 MiB)." + `with_max_file_size` 文档 "Override the maximum file
//!   size accepted by `read`." + 既有回归测试 `read_accepts_file_at_exactly_the_limit`
//!   的口径 "恰好等于上限必须放行 (实读 == max 不截断)"（`src: filesystem.rs:37-38`,
//!   `src: filesystem.rs:90-95`, `src: filesystem.rs:581-590`）
//!   ⇒ 恰好等于上限放行且内容完整；超一字节拒绝并回 "too large"。
//! - **凭据面**：`sensitive_path::credential_surface_refusal` 文档 "拒绝信息即帧:
//!   凭据/密钥面读取拒绝的稳定帧文本 (`pipeline.pre_deny` 语义) … 稳定 code + 来源 +
//!   事实原因「凭据面不可读（安全契约）」"（`src: crates/capabilities/tools/src/sensitive_path.rs:135-145`）
//!   ⇒ 拒绝帧必须带 `pipeline.pre_deny` 与该安全契约说明，且不得回显被保护的正文。
//! - **目录边界拒绝**：`escalation::UpgradeHint::out_of_workspace_access` 文档
//!   （`src: crates/capabilities/tools/src/escalation.rs:29-30,50-63`）+ 既有回归测试
//!   `outside_workspace_refusal_carries_the_upgrade_hint`（`src: filesystem.rs:621-642`）
//!   ⇒ 根外绝对路径的拒绝帧必须带四个升级引导字段。
//! - **形态错误文案**：`FilesystemError::message` 的字面量（`src: filesystem.rs:60-71`）
//!   与各分支产出点（not found `:156-159` / is not a file `:223-230` /
//!   is not a directory `:295-305` / path must not be empty `:124-129` /
//!   invalid filesystem parameters `:409-418` / unknown filesystem operation `:420-429`）。
//! - **列目录**：`MAX_LIST_ENTRIES` 文档 "Maximum entries returned by `list` before
//!   truncation is reported."（`src: filesystem.rs:40-41`）⇒ 返回条数 ≤ 上限；
//!   超限时 `truncated` 为真、条数正好等于上限。
//! - **点文件面**：`sensitive_path` 模块头 "This is intentionally narrower than a
//!   blanket dotfile ban. Ordinary project metadata such as `.gitignore` and
//!   `.cargo/config.toml` remains readable"（`src: sensitive_path.rs:3-6`）+ 既有回归
//!   测试 `normal_project_files_and_dotfiles_remain_readable`（`src: filesystem.rs:830-848`）
//!   ⇒ 普通工程文件与点文件仍可读；`list` 必须过滤凭据面条目（`src: filesystem.rs:800-828`）。
//! - **配置脱敏**：模块头 "配置文件可读但密钥字段的字段值脱敏为 `[redacted]`"
//!   （`src: filesystem.rs:9-11`, `src: sensitive_path.rs:147-158`）。
//!
//! **刻意不造的期望**：平台差异（Windows 盘符 / UNC / 保留名 / NTFS ADS / 尾随空格点）
//! 在被测模块里**没有任何显式处理**，其文档契约也**没有承诺**处理
//! ⇒ 本 target 对这些形态**不**断言"必须拒绝"，只断言"不得逃逸 / 不得 panic /
//! 不得副作用"。这条边界在交付报告里如实登记。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Once, OnceLock};

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use apeireth_tools_canonical::filesystem::{FilesystemTool, MAX_LIST_ENTRIES};
use libfuzzer_sys::fuzz_target;

// ===========================================================================
// 固定夹具（每个 fuzz 进程一份；目录名含 pid + 纳秒 ⇒ 本次运行独有）
// ===========================================================================

/// `read` 的上限（用小上限让边界可测；`with_max_file_size` 文档允许覆写）。
const MAX_FILE_SIZE: u64 = 64;
const NORMAL_CONTENT: &str = "hello apeireth\n";
const NESTED_CONTENT: &str = "nested apeireth\n";
const APP_TOML: &str = "name = \"apeireth\"\napi_key = \"sk-live-secret\"\n";
const ENV_CONTENT: &str = "SECRET_TOKEN=env-secret-value\n";
const CREDS_CONTENT: &str = "{\"api_key\":\"sk-live-creds\"}";
const DOTFILE_CONTENT: &str = "node_modules\n";
/// `bulk/` 的条目数：越过 `MAX_LIST_ENTRIES` 一点点 ⇒ 触发截断路径。
const BULK_COUNT: usize = MAX_LIST_ENTRIES + 50;
/// `many/` 的条目数：排序 / 多文件面。
const MANY_COUNT: usize = 64;
/// 单段长文件名（200 字节，未超单段上限，但足够长）。
const LONG_NAME_LEN: usize = 200;
/// 深目录层数。
const DEEP_LEVELS: usize = 12;
/// 内容维度用的文件名（harness 自己写的临时文件；快照刻意跳过它）。
const CONTENT_FILE: &str = "fuzz_content.txt";

struct Fixture {
    /// 传给工具的原始根（与 `FilesystemTool::new` 收的一致）。
    root_raw: PathBuf,
    /// 根的 canonical 形态（工具内部 `canonical_root()` 用的就是这个）。
    root_canonical: PathBuf,
    /// 根外目录（symlink 跳板指向处 + canary 所在）。
    outside_dir: PathBuf,
    /// 根外文件（canary 所在）。
    outside_file: PathBuf,
    /// 只存在于根外的唯一标记：成功结果里出现它 = 逃逸。
    canary: String,
    /// symlink 是否建成功（部分环境无权限）。
    symlinks_ready: bool,
}

fn build_fixture() -> Option<Fixture> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let base =
        std::env::temp_dir().join(format!("apeireth-fuzz-fs-{}-{nanos}", std::process::id()));
    let root = base.join("root");
    let outside = base.join("outside");
    fs::create_dir_all(&root).ok()?;
    fs::create_dir_all(&outside).ok()?;

    // 普通工程文件
    fs::write(root.join("normal.txt"), NORMAL_CONTENT).ok()?;
    fs::write(root.join(".gitignore"), DOTFILE_CONTENT).ok()?;
    fs::create_dir_all(root.join(".cargo")).ok()?;
    fs::write(root.join(".cargo/config.toml"), "ok\n").ok()?;
    fs::create_dir_all(root.join("sub")).ok()?;
    fs::write(root.join("sub/nested.txt"), NESTED_CONTENT).ok()?;

    // 读上限边界：恰好 64 / 超一字节
    fs::write(root.join("s64.txt"), "A".repeat(MAX_FILE_SIZE as usize)).ok()?;
    fs::write(root.join("s65.txt"), "A".repeat(MAX_FILE_SIZE as usize + 1)).ok()?;
    // 非 UTF-8
    fs::write(root.join("bad.bin"), [0xFFu8, 0xFE, 0x00, 0x01]).ok()?;

    // 已知条目数的小目录（截断对照）
    fs::create_dir_all(root.join("plain")).ok()?;
    for name in ["a.txt", "b.txt", "c.txt"] {
        fs::write(root.join("plain").join(name), name).ok()?;
    }
    // 多文件目录（排序面）
    fs::create_dir_all(root.join("many")).ok()?;
    for i in 0..MANY_COUNT {
        fs::write(root.join("many").join(format!("f{i:04}.txt")), "m").ok()?;
    }
    // ⚠️ 越限目录（截断面，10k+ 文件 ≈ 40 MB）**不在这里建**：每次 fuzz 运行留下
    // 40 MB 临时垃圾太贵。它由固定契约检查 F10 现建现删（见 `run_fixed_contracts`）。
    // 深目录
    let mut deep = root.clone();
    for i in 0..DEEP_LEVELS {
        deep = deep.join(format!("d{i}"));
    }
    fs::create_dir_all(&deep).ok()?;
    fs::write(deep.join("leaf.txt"), "deep\n").ok()?;
    // 超长单段文件名
    fs::write(
        root.join(format!("{}.txt", "l".repeat(LONG_NAME_LEN))),
        "long\n",
    )
    .ok()?;

    // 凭据面（政策保护的文件）
    fs::write(root.join(".env"), ENV_CONTENT).ok()?;
    fs::write(root.join("creds.json"), CREDS_CONTENT).ok()?;
    // 配置文件（可读但字段值脱敏）
    fs::write(root.join("app.toml"), APP_TOML).ok()?;

    // 根外 canary（只存在于根外 ⇒ 成功结果里出现它即逃逸）
    let canary = format!("APEA-FS-CANARY-{}-{nanos}", std::process::id());
    let outside_file = outside.join("secret.txt");
    fs::write(&outside_file, format!("{canary}\n")).ok()?;
    fs::write(outside.join("other.txt"), format!("{canary}\n")).ok()?;

    // symlink 跳板：根内建链，指向根外
    let mut symlinks_ready = true;
    let link_file = root.join("link_out");
    let link_dir = root.join("link_dir");
    let link_in = root.join("link_in");
    let link_sensitive = root.join("link_sensitive");
    symlinks_ready &= symlink(&outside_file, &link_file);
    symlinks_ready &= symlink(&outside, &link_dir);
    symlinks_ready &= symlink(&root.join("normal.txt"), &link_in);
    symlinks_ready &= symlink(&root.join(".env"), &link_sensitive);

    let root_canonical = fs::canonicalize(&root).ok()?;
    Some(Fixture {
        root_raw: root,
        root_canonical,
        outside_dir: outside,
        outside_file,
        canary,
        symlinks_ready,
    })
}

#[cfg(unix)]
fn symlink(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn symlink(target: &Path, link: &Path) -> bool {
    // 目录链与文件链要分别建；建不出来就当该形态不可用（不假造期望）。
    if target.is_dir() {
        std::os::windows::fs::symlink_dir(target, link).is_ok()
    } else {
        std::os::windows::fs::symlink_file(target, link).is_ok()
    }
}

#[cfg(not(any(unix, windows)))]
fn symlink(_target: &Path, _link: &Path) -> bool {
    false
}

fn fixture() -> Option<&'static Fixture> {
    static FIXTURE: OnceLock<Option<Fixture>> = OnceLock::new();
    FIXTURE.get_or_init(build_fixture).as_ref()
}

// ===========================================================================
// 极小 block_on（不引新依赖；`Waker::noop()` 无 unsafe、无自造 impl）
// ===========================================================================

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(value) => return value,
            std::task::Poll::Pending => std::thread::yield_now(),
        }
    }
}

// ===========================================================================
// 调用封装
// ===========================================================================

fn new_tool(fx: &Fixture) -> FilesystemTool {
    FilesystemTool::new(fx.root_raw.clone()).with_max_file_size(MAX_FILE_SIZE)
}

fn invoke(tool: &FilesystemTool, operation: &str, path: &str) -> ToolResult {
    let call = ToolCall {
        id: "call_fs_fuzz".to_string(),
        name: "filesystem".to_string(),
        arguments: serde_json::json!({ "operation": operation, "path": path }),
    };
    block_on(tool.invoke(&call))
}

fn invoke_args(tool: &FilesystemTool, arguments: serde_json::Value) -> ToolResult {
    let call = ToolCall {
        id: "call_fs_fuzz".to_string(),
        name: "filesystem".to_string(),
        arguments,
    };
    block_on(tool.invoke(&call))
}

// ===========================================================================
// 独立 oracle：根外判定 + 工作区快照
// ===========================================================================

/// 独立实现的「在根内」判定：按 **组件** 逐段比，不按字符串前缀
/// （字符串前缀会让 `…/root2` 被判成 `…/root` 的子路径）。
fn under_root(root: &Path, real: &Path) -> bool {
    let root_components: Vec<_> = root.components().collect();
    let real_components: Vec<_> = real.components().collect();
    real_components.len() >= root_components.len()
        && root_components
            .iter()
            .zip(real_components.iter())
            .all(|(a, b)| a == b)
}

/// 地面真相：请求路径经 OS 解析后是否**落在根外**。
/// `None` = 路径不存在（canonicalize 失败）⇒ 不作断言。
fn escapes_root(fx: &Fixture, requested: &str) -> Option<bool> {
    let candidate = if Path::new(requested).is_absolute() {
        PathBuf::from(requested)
    } else {
        fx.root_raw.join(requested)
    };
    let real = fs::canonicalize(&candidate).ok()?;
    Some(!under_root(&fx.root_canonical, &real))
}

/// 工作区快照（名称 + 类型 + 文件长度；symlink 不跟随）。
/// `bulk/` / `many/` 只记目录本身（不遍历内容 —— 10k+ 条目的遍历会把每轮 fuzz
/// 拖到几十毫秒；这两个目录的内容由固定契约检查 F8/F10 覆盖）。
///
/// 快照拼成**单个** String（ASAN 下逐条 `format!` 的分配开销是主要瓶颈）。
fn snapshot(root: &Path) -> String {
    let mut out = String::new();
    walk_snapshot(root, "", &mut out, 0);
    out
}

fn walk_snapshot(dir: &Path, prefix: &str, out: &mut String, depth: usize) {
    if depth > 8 {
        return;
    }
    let Ok(reader) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = reader.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == CONTENT_FILE {
            // harness 自己写的临时内容文件（内容每轮变）⇒ 不进快照。
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        out.push_str(prefix);
        out.push('/');
        out.push_str(&name);
        if file_type.is_symlink() {
            out.push_str(":l\n");
            continue;
        }
        if file_type.is_file() {
            let len = fs::symlink_metadata(entry.path())
                .map(|m| m.len())
                .unwrap_or(0);
            out.push_str(":f");
            out.push_str(&len.to_string());
            out.push('\n');
            continue;
        }
        out.push_str(":d\n");
        if file_type.is_dir() && name != "bulk" && name != "many" {
            let mut nested = name.to_string();
            nested.push('/');
            walk_snapshot(&entry.path(), &nested, out, depth + 1);
        }
    }
}

// ===========================================================================
// 随机面：形状变换 + 逃逸/副作用/泄漏断言
// ===========================================================================

const OPERATIONS: [&str; 12] = [
    "read", "list", "stat", "write", "delete", "rename", "copy", "READ", "LiSt", "", "read ",
    "stat\n",
];

const TRAILING_FORMS: [&str; 6] = ["", ".", " ", "  ", ":stream", ":$DATA"];
const DEVICE_NAMES: [&str; 7] = ["CON", "NUL", "COM1", "LPT1", "CON.txt", "PRN", "AUX"];
const KNOWN_PATHS: [&str; 13] = [
    "normal.txt",
    "sub/nested.txt",
    ".env",
    "creds.json",
    "app.toml",
    "plain",
    "many",
    "link_out",
    "link_dir",
    "link_in",
    "s64.txt",
    "s65.txt",
    ".gitignore",
];

/// 路径形状变换：把 fuzz 载荷摊到任务书点名的形态面上
/// （`..` 各种写法 / 绝对路径 / 全角点 / 日文点 / 零宽 / RTL 覆写 /
///  混合分隔符 / UNC / 尾随空格点 / ADS / 设备名 / 超长路径 / 深目录）。
fn shape_path(sel: u8, payload: &str, root: &Path) -> String {
    let unit = if payload.is_empty() { "x" } else { payload };
    match (sel as usize) % 18 {
        0 => payload.to_string(),
        1 => format!("../{payload}"),
        2 => format!("..\\{payload}"),
        3 => format!("....//{payload}"),
        4 => format!("sub/../{payload}"),
        // 全角点 U+FF0E
        5 => payload.replace('.', "\u{FF0E}"),
        // 日文点 U+3002
        6 => payload.replace('/', "\u{3002}"),
        // 零宽字符 U+200B
        7 => format!("{payload}\u{200B}{payload}"),
        // RTL 覆写 U+202E
        8 => format!("\u{202E}{payload}"),
        // 绝对路径（根内拼写）
        9 => root.join(payload).to_string_lossy().into_owned(),
        // 尾随空格 / 点 / ADS
        10 => format!(
            "{payload}{}",
            TRAILING_FORMS[payload.len() % TRAILING_FORMS.len()]
        ),
        // Windows 设备名
        11 => DEVICE_NAMES[payload.len() % DEVICE_NAMES.len()].to_string(),
        // 超长路径（> PATH_MAX）
        12 => {
            let mut s = String::from("../");
            while s.len() < 4200 {
                s.push_str(unit);
            }
            s
        }
        // UNC
        13 => format!("\\\\server\\share\\{payload}"),
        // 混合分隔符
        14 => payload.replace('/', "\\"),
        // 根的兄弟目录
        15 => format!("{}/../{}", root.display(), payload),
        // 已知条目
        16 => KNOWN_PATHS[payload.len() % KNOWN_PATHS.len()].to_string(),
        // 已知条目的组合
        17 => format!(
            "{}/{}",
            KNOWN_PATHS[payload.len() % KNOWN_PATHS.len()],
            KNOWN_PATHS[(payload.len() + 1) % KNOWN_PATHS.len()]
        ),
        _ => payload.to_string(),
    }
}

fn run_case(fx: &Fixture, operation: &str, path: &str) {
    let tool = new_tool(fx);
    let result = invoke(&tool, operation, path);

    // 契约：限域（模块头 "…checked for containment inside that root … so `..` and
    // symlink traversal do not escape the root"）⇒ 地面真相判定为根外的路径，
    // 结果必须不是成功。
    // （只在成功时才需要地面真相：失败已经满足契约，省一次 canonicalize。）
    if result.is_ok() {
        if let Some(true) = escapes_root(fx, path) {
            panic!(
                "路径逃逸未被拒绝: operation={operation:?} path={path:?} result={}",
                result.render()
            );
        }
        let rendered = result.render();
        // 根外正文的唯一标记不得出现在成功结果里。
        assert!(
            !rendered.contains(&fx.canary),
            "根外内容泄漏: operation={operation:?} path={path:?} result={rendered}"
        );
        // 成功结果的路径表述必须是相对路径（`relative_display` 的语义：
        // 落在根内 ⇒ strip_prefix 成功 ⇒ 相对；绝对路径形态只在越界时才出现）。
        assert!(
            !rendered.contains(fx.root_raw.to_string_lossy().as_ref()),
            "成功结果泄漏了根的绝对路径: operation={operation:?} path={path:?}"
        );
        assert!(
            !rendered.contains(fx.outside_dir.to_string_lossy().as_ref()),
            "成功结果泄漏了根外目录路径: operation={operation:?} path={path:?}"
        );
    }
}

// ===========================================================================
// 固定契约检查（每个进程跑一次：这些断言不靠随机输入，靠夹具事实）
// ===========================================================================

fn run_fixed_contracts(fx: &Fixture) {
    // ── F0 oracle 自检（harness 自检：逃逸判据不能是空的） ────────────────
    // 随机面的逃逸断言依赖 `escapes_root` 的地面真相；先证明它对已知跳板
    // 判"根外"、对已知根内文件判"根内" —— 否则那条断言会静默失效。
    assert_eq!(
        escapes_root(fx, "link_out"),
        Some(true),
        "oracle 自检: symlink 跳板必须判成根外"
    );
    assert_eq!(
        escapes_root(fx, "../outside/secret.txt"),
        Some(true),
        "oracle 自检: 词法穿越必须判成根外"
    );
    assert_eq!(
        escapes_root(fx, &fx.outside_file.to_string_lossy()),
        Some(true),
        "oracle 自检: 根外绝对路径必须判成根外"
    );
    assert_eq!(
        escapes_root(fx, "normal.txt"),
        Some(false),
        "oracle 自检: 根内文件必须判成根内"
    );
    assert_eq!(
        escapes_root(fx, "no-such-file"),
        None,
        "oracle 自检: 不存在的路径没有地面真相"
    );

    // ── F1 空路径 / 纯空白路径 ────────────────────────────────────────────
    // src: filesystem.rs:124-129 —— "path must not be empty"（trim 后为空即拒）
    for empty in ["", "   ", "\t", "\n"] {
        let result = invoke(&new_tool(fx), "read", empty);
        assert!(!result.is_ok(), "空路径必须被拒: {empty:?}");
        assert!(
            result.render().contains("path must not be empty"),
            "空路径拒绝文案: {}",
            result.render()
        );
    }

    // ── F2 目标不存在 ─────────────────────────────────────────────────────
    // src: filesystem.rs:64（not found: {m}）、filesystem.rs:156-159
    let result = invoke(&new_tool(fx), "read", "no-such-file.txt");
    assert!(!result.is_ok());
    assert!(result.render().contains("not found"), "{}", result.render());

    // ── F3 读目录 / 列文件 ────────────────────────────────────────────────
    // src: filesystem.rs:223-230（"{path} is not a file"）
    let result = invoke(&new_tool(fx), "read", "sub");
    assert!(!result.is_ok());
    assert!(
        result.render().contains("is not a file"),
        "{}",
        result.render()
    );
    // src: filesystem.rs:295-305（"{path} is not a directory"）
    let result = invoke(&new_tool(fx), "list", "normal.txt");
    assert!(!result.is_ok());
    assert!(
        result.render().contains("is not a directory"),
        "{}",
        result.render()
    );

    // ── F4 读上限边界 ─────────────────────────────────────────────────────
    // src: filesystem.rs:37-38, 90-95, 235-238, 255-264, 581-601
    let tool = new_tool(fx);
    let at_limit = invoke(&tool, "read", "s64.txt");
    assert!(at_limit.is_ok(), "{}", at_limit.render());
    assert_eq!(at_limit.render(), "A".repeat(MAX_FILE_SIZE as usize));
    let over_limit = invoke(&tool, "read", "s65.txt");
    assert!(!over_limit.is_ok());
    assert!(
        over_limit.render().contains("too large"),
        "{}",
        over_limit.render()
    );

    // ── F5 非 UTF-8 ───────────────────────────────────────────────────────
    // src: filesystem.rs:67, 279-282
    let result = invoke(&new_tool(fx), "read", "bad.bin");
    assert!(!result.is_ok());
    assert!(result.render().contains("UTF-8"), "{}", result.render());

    // ── F6 凭据面拒绝（帧） ───────────────────────────────────────────────
    // src: sensitive_path.rs:135-145；filesystem.rs:176-182
    for (path, secret) in [(".env", ENV_CONTENT), ("creds.json", CREDS_CONTENT)] {
        let tool = new_tool(fx);
        for result in [
            invoke(&tool, "read", path),
            invoke(&tool, "stat", path),
            invoke(&tool, "list", path),
        ] {
            assert!(!result.is_ok(), "凭据面必须被拒: {path}");
            let rendered = result.render();
            assert!(
                rendered.contains("pipeline.pre_deny"),
                "{path}: 拒绝帧缺稳定 code: {rendered}"
            );
            assert!(
                rendered.contains("凭据面不可读（安全契约）"),
                "{path}: 拒绝帧缺事实原因: {rendered}"
            );
            assert!(
                !rendered.contains(secret),
                "{path}: 拒绝帧回显了被保护正文: {rendered}"
            );
        }
    }

    // ── F7 词法穿越 / symlink 跳板 / 根外绝对路径 ─────────────────────────
    // src: filesystem.rs:3-6（模块头限域承诺）、filesystem.rs:690-732（既有回归测试）
    let tool = new_tool(fx);
    let traversal = invoke(&tool, "read", "../outside/secret.txt");
    assert!(!traversal.is_ok(), "词法穿越必须被拒");
    assert!(
        traversal.render().contains("permission denied"),
        "{}",
        traversal.render()
    );
    assert!(!traversal.render().contains(&fx.canary));

    if fx.symlinks_ready {
        for path in ["link_out", "link_dir"] {
            let result = invoke(&new_tool(fx), "read", path);
            assert!(!result.is_ok(), "symlink 跳板必须被拒: {path}");
            assert!(
                result.render().contains("permission denied"),
                "{path}: {}",
                result.render()
            );
            assert!(
                !result.render().contains(&fx.canary),
                "{path}: 跳板带出了根外正文"
            );
        }
    }

    // 根外绝对路径 ⇒ 拒绝帧必须带升级引导
    // src: escalation.rs:29-30, 50-63；filesystem.rs:621-642（既有回归测试）
    let absolute_outside = fx.outside_file.to_string_lossy().to_string();
    let result = invoke(&new_tool(fx), "read", &absolute_outside);
    assert!(!result.is_ok(), "根外绝对路径必须被拒");
    let rendered = result.render();
    for expected in [
        "\"missing_mode\":\"relaxed\"",
        "\"justification_field\":\"justification\"",
        "\"grant_scope\":\"one_call\"",
        "\"request_key\":\"sandbox_escalation\"",
    ] {
        assert!(
            rendered.contains(expected),
            "目录边界拒绝缺升级引导 {expected}: {rendered}"
        );
    }
    assert!(!rendered.contains(&fx.canary));

    // ── F8 写类操作词必须被拒且零副作用 ──────────────────────────────────
    // src: filesystem.rs:13-15（"Write/delete/rename/copy are deliberately not
    // implemented in M2A"）、filesystem.rs:420-429（unknown filesystem operation）
    let before = snapshot(&fx.root_raw);
    for operation in ["write", "delete", "rename", "copy", "mkdir", "rm"] {
        let result = invoke(&new_tool(fx), operation, "normal.txt");
        assert!(!result.is_ok(), "未实现的操作必须被拒: {operation}");
        assert!(
            result.render().contains("unknown filesystem operation"),
            "{operation}: {}",
            result.render()
        );
    }
    assert_eq!(before, snapshot(&fx.root_raw), "被拒的写操作留下了副作用");

    // ── F9 参数形状 ───────────────────────────────────────────────────────
    // src: filesystem.rs:43-48（deny_unknown_fields）、filesystem.rs:409-418
    let result = invoke_args(
        &new_tool(fx),
        serde_json::json!({ "operation": "read", "path": "normal.txt", "extra": 1 }),
    );
    assert!(!result.is_ok());
    assert!(
        result.render().contains("invalid filesystem parameters"),
        "{}",
        result.render()
    );
    // src: filesystem.rs:420 —— 操作词先 lower 再匹配 ⇒ 大小写等价。
    let upper = invoke(&new_tool(fx), "READ", "normal.txt");
    assert!(upper.is_ok(), "{}", upper.render());
    assert_eq!(upper.render(), NORMAL_CONTENT);
    let mixed = invoke(&new_tool(fx), "LiSt", "plain");
    assert!(mixed.is_ok(), "{}", mixed.render());

    // ── F10 list 的计数 / 截断 / 排序 ─────────────────────────────────────
    // src: filesystem.rs:40-41（MAX_LIST_ENTRIES 文档）、filesystem.rs:330-340、
    //      filesystem.rs:654-671（既有回归测试：entries should be sorted）
    let listed = invoke(&new_tool(fx), "list", "plain");
    assert!(listed.is_ok(), "{}", listed.render());
    let value: serde_json::Value = serde_json::from_str(&listed.render())
        .unwrap_or_else(|e| panic!("list 结果必须是 JSON: {e}: {}", listed.render()));
    assert_eq!(value["count"], serde_json::json!(3), "{}", listed.render());
    assert_eq!(value["truncated"], serde_json::json!(false));
    let names: Vec<String> = value["entries"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|e| e["name"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        names,
        vec!["a.txt", "b.txt", "c.txt"],
        "{}",
        listed.render()
    );
    assert!(
        value["entries"].as_array().map(|a| a.len()).unwrap_or(0) <= MAX_LIST_ENTRIES,
        "list 返回条数不得超过上限"
    );

    // 越限目录 ⇒ 截断必须被报告，且条数正好等于上限
    // （目录现建现删：10k+ 文件 ≈ 40 MB，留着不划算）
    let bulk_dir = fx.root_raw.join("bulk");
    fs::create_dir_all(&bulk_dir).expect("建越限目录");
    for i in 0..BULK_COUNT {
        fs::write(bulk_dir.join(format!("b{i:05}.txt")), "b").expect("写越限目录条目");
    }
    let bulk = invoke(&new_tool(fx), "list", "bulk");
    let bulk_value: serde_json::Value = serde_json::from_str(&bulk.render())
        .unwrap_or_else(|e| panic!("list 结果必须是 JSON: {e}"));
    assert_eq!(
        bulk_value["count"],
        serde_json::json!(MAX_LIST_ENTRIES),
        "越限目录必须截断到上限: {}",
        bulk_value["count"]
    );
    assert_eq!(bulk_value["truncated"], serde_json::json!(true));
    assert_eq!(
        bulk_value["entries"].as_array().map(|a| a.len()),
        Some(MAX_LIST_ENTRIES)
    );
    let _ = fs::remove_dir_all(&bulk_dir);

    // ── F11 普通工程文件与点文件仍可读 / list 过滤凭据面 ─────────────────
    // src: sensitive_path.rs:3-6；filesystem.rs:800-848（既有回归测试）
    for path in [
        "normal.txt",
        ".gitignore",
        ".cargo/config.toml",
        "sub/nested.txt",
    ] {
        let result = invoke(&new_tool(fx), "read", path);
        assert!(
            result.is_ok(),
            "普通文件被误拦: {path}: {}",
            result.render()
        );
    }
    let listed = invoke(&new_tool(fx), "list", ".");
    assert!(listed.is_ok(), "{}", listed.render());
    let rendered = listed.render();
    for hidden in [".env", "creds.json"] {
        assert!(
            !rendered.contains(hidden),
            "list 泄漏了凭据面条目 {hidden}: {rendered}"
        );
    }
    for visible in ["normal.txt", ".gitignore", "sub"] {
        assert!(rendered.contains(visible), "list 漏了正常条目 {visible}");
    }

    // ── F12 配置文件的字段值脱敏 ─────────────────────────────────────────
    // src: filesystem.rs:9-11；sensitive_path.rs:147-158
    let result = invoke(&new_tool(fx), "read", "app.toml");
    assert!(result.is_ok(), "{}", result.render());
    let rendered = result.render();
    assert!(!rendered.contains("sk-live-secret"), "{rendered}");
    assert!(rendered.contains("[redacted]"), "{rendered}");
    assert!(rendered.contains("apeireth"), "{rendered}");

    // ── F13 stat 的元数据面 ───────────────────────────────────────────────
    // src: filesystem.rs:345-375
    let result = invoke(&new_tool(fx), "stat", "normal.txt");
    assert!(result.is_ok(), "{}", result.render());
    let value: serde_json::Value = serde_json::from_str(&result.render())
        .unwrap_or_else(|e| panic!("stat 结果必须是 JSON: {e}"));
    assert_eq!(value["kind"], serde_json::json!("file"));
    assert_eq!(value["size"], serde_json::json!(NORMAL_CONTENT.len()));
    assert_eq!(value["path"], serde_json::json!("normal.txt"));
    let result = invoke(&new_tool(fx), "stat", "sub");
    let value: serde_json::Value = serde_json::from_str(&result.render())
        .unwrap_or_else(|e| panic!("stat 结果必须是 JSON: {e}"));
    assert_eq!(value["kind"], serde_json::json!("dir"));
}

// ===========================================================================
// fuzz 入口
// ===========================================================================

/// 内容维度的密钥值：只取安全字母表（不含引号/换行/等号 ⇒ 不改变配置行的解析），
/// 并加固定前缀避免与脱敏标签 `[redacted]` 有子串关系。
///
/// ⚠️ 长度必须封顶：内容维度写入的配置文件会被 `read` 读回，而 `read` 有上限
/// （本 target 用 `MAX_FILE_SIZE = 64`）。不封顶时载荷一长就撞上限 —— 那是
/// **harness 的锅**（第一次 90s 实跑就是这么炸的：库正确回了 "file too large"，
/// 是我忘了算模板开销）。
const SECRET_TOKEN_MAX: usize = 8;

fn secret_token(payload: &str) -> String {
    let filtered: String = payload
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(SECRET_TOKEN_MAX)
        .collect();
    format!("SEC{filtered}")
}

/// 内容维度：`read` 读到的正文里，密钥字段的**字段值**必须脱敏为 `[redacted]`，
/// 非敏感字段必须逐字节保留。
///
/// 契约出处：
/// - 模块头 "配置文件可读但密钥字段的字段值脱敏为 `[redacted]`（与启动日志脱敏同一
///   语义）；非敏感字段照读"（`src: crates/capabilities/tools/src/filesystem.rs:9-11`）
/// - `sensitive_path::redact_secret_field_values` 文档 "key/token/secret 等密钥字段的值
///   替换为 [`REDACTED`], 字段名与非敏感字段逐字节保留"（`src: sensitive_path.rs:147-158`）
/// - 字段名 `api_key` 与期望形态取自产品自带回归测试
///   `redacts_secret_field_values_across_config_shapes`（`src: sensitive_path.rs:375-395`）
///   与 `redaction_leaves_non_sensitive_fields_and_prose_untouched`（`src: sensitive_path.rs:397-414`）。
fn run_content_case(fx: &Fixture, payload: &str) {
    let secret = secret_token(payload);
    let content = format!("api_key = \"{secret}\"\nname = \"keep\"\n");
    // harness 自检：内容维度写入的文件必须落在 `read` 上限内，否则下面那句
    // 「普通配置文件读失败」会把 harness 的锅说成库的错。
    assert!(
        content.len() as u64 <= MAX_FILE_SIZE,
        "harness 自检: 内容维度文件超出 read 上限（{}/{}）—— harness 的锅，不是库的",
        content.len(),
        MAX_FILE_SIZE
    );
    // 写文件是 **harness** 的行为（不是被测工具的）⇒ 快照刻意跳过该文件。
    if fs::write(fx.root_raw.join(CONTENT_FILE), &content).is_err() {
        return;
    }
    let result = invoke(&new_tool(fx), "read", CONTENT_FILE);
    if !result.is_ok() {
        // 读不了自己写的普通文件 = 契约外行为，但判据只能来自文档：普通工程文件可读
        // （`src: sensitive_path.rs:3-6`）⇒ 这里直接判失败。
        panic!("普通配置文件读失败: {}", result.render());
    }
    let rendered = result.render();
    assert!(
        !rendered.contains(&secret),
        "密钥字段值未脱敏: secret={secret:?} rendered={rendered:?}"
    );
    assert!(
        rendered.contains("[redacted]"),
        "脱敏标签缺失: rendered={rendered:?}"
    );
    assert!(
        rendered.contains("keep"),
        "非敏感字段被改动: rendered={rendered:?}"
    );
}

fuzz_target!(|data: &[u8]| {
    let Some(fx) = fixture() else {
        return;
    };

    // 固定契约检查（一次性）：不依赖随机输入的事实断言。
    static FIXED: Once = Once::new();
    FIXED.call_once(|| run_fixed_contracts(fx));

    let Some(&op_sel) = data.first() else {
        return;
    };
    let Some(&shape_sel) = data.get(1) else {
        return;
    };
    let payload = String::from_utf8_lossy(&data[2..]);

    let path = shape_path(shape_sel, &payload, &fx.root_raw);
    let op_a = OPERATIONS[(op_sel as usize) % OPERATIONS.len()];
    let op_b = OPERATIONS[((op_sel as usize) + 1) % OPERATIONS.len()];

    // 契约：只读工具（模块头 "Read-only filesystem tool capability." +
    // "Write/delete/rename/copy are deliberately not implemented in M2A."）
    // ⇒ 调用前后工作区快照必须逐项相同。
    let before = snapshot(&fx.root_raw);
    run_case(fx, op_a, &path);
    run_case(fx, op_b, &path);
    let after = snapshot(&fx.root_raw);
    assert_eq!(
        before, after,
        "只读工具产生了副作用: operation=[{op_a:?},{op_b:?}] path={path:?}"
    );

    // 内容维度（`read` 的字段值脱敏面）。
    run_content_case(fx, &payload);
});
