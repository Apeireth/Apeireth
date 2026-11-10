#![no_main]

//! fuzz: **补丁应用面**（`TransactionalPatchApplier::parse_patch` / `apply`）——
//! 补丁内容驱动的**路径逃逸** + 事务原子性。
//!
//! # 判据来源（全部取自被测模块自己写明的语义，不自造期望）
//!
//! | # | 断言 | 文档出处（`crates/capabilities/tools/src/apply_patch.rs`） |
//! |---|------|------------------------------------------------------|
//! | C1 | 补丁帧不合式（不以 `*** Begin Patch` 开头 / 不以 `*** End Patch` 结尾）⇒ `ParseError` | L147-151 「补丁必须以 '*** Begin Patch' 开头并以 '*** End Patch' 结尾」+ L134-144「解析失败即 `ApplyPatchError::ParseError`」 |
//! | C2 | `parse_patch` 失败 ⇒ 变体只可能是 `ParseError` | L134-144「解析即合同 … 解析失败即 `ApplyPatchError::ParseError`」 |
//! | C3 | `parse_patch` Ok ⇒ 动作集非空（零动作必须报错，不许「成功但零动作」的谎帧） | L279-284 + L308-314 |
//! | C4 | `parse_patch` Ok ⇒ 每个 `*** Update File` 段至少一个 hunk | L262-267「零 hunk 补丁拒绝」 |
//! | C5 | `parse_patch` Ok ⇒ 每个动作路径非空 | L586-594「补丁指令缺少目标路径」 |
//! | C6 | 绝对路径 / `..` 组件 / 盘符·UNC 前缀 ⇒ `PathViolation` | L494-524「词法层: 拒绝绝对路径、盘符/UNC 前缀与任何 `..` 组件」 |
//! | C7 | root 内指向外部的 symlink ⇒ `PathViolation` | L499-503「root 内指向外部的 symlink 在此现形」+ L51-54「fail-closed, 整个事务在预演阶段即拒绝」 |
//! | C8 | apply 失败 ⇒ 工作区零半成品（预演阶段即拒绝，事务全有或全无） | L4「若任意一个文件的任意一个 Hunk 匹配失败，整个事务立即完全回滚，保证磁盘状态绝对一致」+ L51-54 |
//! | C9 | apply 成功 ⇒ 每个 hunk 在当次内容里恰好唯一命中（strict unique，不许静默首命中替换） | L619-620 + L7-8「require a **strict unique** match … Silent first-match replacement is rejected」 |
//! | C10 | apply 成功 ⇒ 落盘内容 == 动作声明按序合成的结果（每个动作都落地，不许静默吞动作） | L3-4「多文件新增/删除/按上下文精确替换更新」+ L9-10「同一路径按**声明顺序**叠加 … 报告说改了几处, 磁盘就得有几处的效果」+ L462-468 报告账目 |
//! | C11 | apply 成功 ⇒ `Add` 落盘内容 == 解析出的 content（`+` 前缀自动剥离） | L596-617「Add File body … 自动识别: 若所有行都以 `+` 开头, 剥掉此前缀」 |
//! | C12 | apply 成功 ⇒ `Delete` 目标已消失 | L60-75 动作语义 + L441-460 提交删除 |
//! | C13 | 任何输入都不得 panic；任何输入都不得写到工作区之外 | L494-503「H4 … 模型或被 prompt 注入诱导的补丁文本即可任意写/删 —— 在此逐 action 拒绝」 |
//!
//! # 判据写法说明
//!
//! - **词法越界的 oracle 用字符串层独立实现**（按平台分隔符切段查 `..`、查绝对前缀），
//!   不用被测实现的 `Path::components` 口径，避免同义反复。
//! - **「不得写到工作区外」用夹具快照逐项比对**（`outside/` 目录内容 + 符号链接），
//!   这是 C13 的可观测形式：逃逸一旦发生必有痕迹。
//! - **越界目标一律指向本次运行独有的临时目录**（不碰 `/etc` 等真路径）：即使实现有洞，
//!   也只可能损坏本轮自己的夹具。
//! - **怪形不断言变体**：NUL / 尾随分隔符 / 目录 / 保留名等，模块文档没有承诺具体变体，
//!   只断言「不 panic、不越界、失败不留半成品」。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use apeireth_tools_canonical::apply_patch::{
    ApplyPatchError, FilePatchAction, PatchHunk, TransactionalPatchApplier,
};
use libfuzzer_sys::fuzz_target;

// ───────────────────────────── 固定夹具 ─────────────────────────────

const TARGET_CONTENT: &str = "alpha\nbeta\ngamma\n";
const DUP_CONTENT: &str = "dup\ndup\ndup\n";
const KEEP_CONTENT: &str = "keep\n";
const OUTSIDE_SECRET: &str = "SECRET-OUTSIDE\n";
const OUTSIDE_OTHER: &str = "OTHER-OUTSIDE\n";

/// 固定候选 needle：命中 / 不命中 / 多命中（歧义）三类结局都要打到。
const FIXED_NEEDLES: &[&str] = &["beta", "dup", "gamma", "alpha", "keep"];

/// 本次运行独有的临时根（pid + 原子计数）。
fn unique_base(seq: u64) -> PathBuf {
    std::env::temp_dir().join(format!("apeireth-apf-fuzz-{}-{}", std::process::id(), seq))
}

/// 夹具：`<base>/root` 是工作区根，`<base>/outside` 是「工作区外」受害者目录。
struct Fixture {
    base: PathBuf,
    root: PathBuf,
    outside: PathBuf,
    /// 三个 symlink 是否建成（Windows 需特权）。
    symlinks: bool,
}

impl Fixture {
    fn setup(seq: u64) -> Option<Fixture> {
        let base = unique_base(seq);
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("root");
        let outside = base.join("outside");
        std::fs::create_dir_all(root.join("sub")).ok()?;
        std::fs::create_dir_all(&outside).ok()?;
        std::fs::write(root.join("target.txt"), TARGET_CONTENT).ok()?;
        std::fs::write(root.join("dup.txt"), DUP_CONTENT).ok()?;
        std::fs::write(root.join("sub/keep.txt"), KEEP_CONTENT).ok()?;
        std::fs::write(outside.join("secret.txt"), OUTSIDE_SECRET).ok()?;
        std::fs::write(outside.join("other.txt"), OUTSIDE_OTHER).ok()?;
        #[cfg(unix)]
        let symlinks = {
            use std::os::unix::fs::symlink;
            symlink(outside.join("secret.txt"), root.join("link.txt")).is_ok()
                && symlink(&outside, root.join("link_dir")).is_ok()
                && symlink(outside.join("nonexistent.txt"), root.join("dangling")).is_ok()
        };
        #[cfg(not(unix))]
        let symlinks = false;
        Some(Fixture {
            base,
            root,
            outside,
            symlinks,
        })
    }

    fn cleanup(&self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

// ───────────────────────────── 快照 / 比对 ─────────────────────────────

#[derive(PartialEq, Eq, Debug)]
enum Entry {
    File(Vec<u8>),
    Symlink(PathBuf),
    Dir,
}

/// 目录快照（相对路径 → 条目）。符号链接**不跟随**（否则会把 outside 的内容
/// 混进 root 的快照），按链接目标本身记账。
fn snapshot(dir: &Path) -> BTreeMap<String, Entry> {
    let mut map = BTreeMap::new();
    walk_snapshot(dir, "", &mut map);
    map
}

fn walk_snapshot(dir: &Path, prefix: &str, map: &mut BTreeMap<String, Entry>) {
    let Ok(reader) = std::fs::read_dir(dir) else {
        return;
    };
    let mut items: Vec<_> = reader.flatten().collect();
    items.sort_by_key(|item| item.file_name());
    for item in items {
        let name = item.file_name().to_string_lossy().to_string();
        let key = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let path = item.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        let file_type = meta.file_type();
        if file_type.is_symlink() {
            let target = std::fs::read_link(&path).unwrap_or_default();
            map.insert(key, Entry::Symlink(target));
        } else if file_type.is_dir() {
            map.insert(key.clone(), Entry::Dir);
            walk_snapshot(&path, &key, map);
        } else if file_type.is_file() {
            let bytes = std::fs::read(&path).unwrap_or_default();
            map.insert(key, Entry::File(bytes));
        }
    }
}

/// 工作区在「拒绝之后」必须零变化：文件与符号链接逐一相同。
///
/// 目录允许**新增**（`create_dir_all` 半途失败会留下空目录；模块文档把回滚
/// 明确写成 best-effort，见 L453-458），但原有目录必须还在。
fn same_files_and_links(before: &BTreeMap<String, Entry>, after: &BTreeMap<String, Entry>) -> bool {
    for (key, entry) in before {
        if matches!(entry, Entry::Dir) {
            if !after.contains_key(key) {
                return false;
            }
        } else if after.get(key) != Some(entry) {
            return false;
        }
    }
    for (key, entry) in after {
        if !matches!(entry, Entry::Dir) && before.get(key) != Some(entry) {
            return false;
        }
    }
    true
}

/// 快照键归一：与 `walk_snapshot` 的键口径一致（Windows 上分隔符不同）。
fn norm_key(path: &str) -> String {
    #[cfg(windows)]
    {
        path.replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        path.to_string()
    }
}

/// 去掉尾随分隔符。
///
/// ⚠️ harness 自身的坑（第一版就踩了）：linux 上路径尾随 `/` 会强制内核把最后
/// 一个分量解析成**目录**，于是 `root/....//` 与实现真正写入的 `root/....`
/// 不是同一条路径 —— 读回来的是「读失败」而不是文件内容，断言就会假报。
/// 实现侧写盘用的是 canonical 拼写（无尾随分隔符），harness 必须跟同一口径。
fn strip_trailing_sep(text: &str) -> &str {
    #[cfg(windows)]
    {
        text.trim_end_matches(['/', '\\'])
    }
    #[cfg(not(windows))]
    {
        text.trim_end_matches('/')
    }
}

/// 目标落盘路径拼写：按分量拼（自动归一尾随分隔符与 `.` 分量），
/// 与实现的 canonical 拼写落在同一个文件上。
fn target_path(root: &Path, path: &Path) -> PathBuf {
    let mut joined = root.to_path_buf();
    for component in path.components() {
        joined.push(component.as_os_str());
    }
    joined
}

/// 取快照里某文件的原始内容（UTF-8 松散转换：apply 成功 ⇒ 原文件必可 UTF-8 读）。
fn original_of(before: &BTreeMap<String, Entry>, path: &str) -> Option<String> {
    let key = norm_key(path);
    let key = strip_trailing_sep(&key);
    let key = key.strip_prefix("./").unwrap_or(key).to_string();
    match before.get(&key) {
        Some(Entry::File(bytes)) => Some(String::from_utf8_lossy(bytes).to_string()),
        _ => None,
    }
}

// ───────────────────────────── 词法 oracle（字符串层，独立于实现） ─────────────────────────────

/// 绝对路径判据：unix 以 `/` 开头；windows 另加 `\` 开头、盘符前缀、UNC。
fn looks_absolute(path: &str) -> bool {
    if path.starts_with('/') {
        return true;
    }
    #[cfg(windows)]
    {
        if path.starts_with('\\') {
            return true;
        }
        let bytes = path.as_bytes();
        if bytes.len() >= 2 && bytes[1] == b':' && (bytes[0] as char).is_ascii_alphabetic() {
            return true;
        }
    }
    false
}

/// 路径分段（按平台分隔符）。
fn segments(path: &str) -> Vec<&str> {
    #[cfg(unix)]
    {
        path.split('/').collect()
    }
    #[cfg(not(unix))]
    {
        path.split(['/', '\\']).collect()
    }
}

/// `..` 穿越判据：任一段恰为 `..`。
fn has_parent_segment(path: &str) -> bool {
    segments(path).iter().any(|segment| *segment == "..")
}

/// 只有「不可能越界」的路径才允许真的落盘（fuzz 语料可能含真系统路径，
/// 落盘会伤机器 —— 越界目标一律换成本轮临时目录里的副本）。
fn path_is_contained_candidate(path: &str) -> bool {
    !looks_absolute(path) && !has_parent_segment(path)
}

// ───────────────────────────── 路径向量表 ─────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum VecKind {
    /// 词法越界：绝对路径 / `..` 组件 / 盘符·UNC 前缀 ⇒ 必须 `PathViolation`。
    Lexical,
    /// 现实层 symlink 逃逸 ⇒ 必须 `PathViolation`。
    Symlink,
    /// 工作区内、目标不存在（Add 必成功）。
    Fresh,
    /// 工作区内、目标是已存在的普通文件。
    Existing,
    /// 怪形（NUL / 尾随分隔符 / 目录 / 保留名）：不断言变体。
    Odd,
}

struct PathVec {
    text: String,
    kind: VecKind,
}

fn path_vectors(fx: &Fixture) -> Vec<PathVec> {
    let secret_abs = fx.outside.join("secret.txt").to_string_lossy().to_string();
    let tree = fx
        .base
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut vectors: Vec<PathVec> = Vec::new();
    let mut push = |text: String, kind: VecKind| {
        vectors.push(PathVec { text, kind });
    };

    // ── 词法层越界（文档: 拒绝绝对路径、盘符/UNC 前缀与任何 `..` 组件）──
    push("../outside/secret.txt".into(), VecKind::Lexical);
    push(format!("../../{tree}/outside/secret.txt"), VecKind::Lexical);
    push("sub/../../outside/secret.txt".into(), VecKind::Lexical);
    push("sub/./../../outside/secret.txt".into(), VecKind::Lexical);
    push(".././outside/secret.txt".into(), VecKind::Lexical);
    push("..".into(), VecKind::Lexical);
    push("../".into(), VecKind::Lexical);
    push("..//".into(), VecKind::Lexical);
    push(secret_abs.clone(), VecKind::Lexical);
    push("/".into(), VecKind::Lexical);
    push("//".into(), VecKind::Lexical);
    #[cfg(windows)]
    {
        push("C:\\evil.txt".into(), VecKind::Lexical);
        push("C:evil.txt".into(), VecKind::Lexical);
        push("\\\\server\\share\\x".into(), VecKind::Lexical);
    }
    #[cfg(not(windows))]
    {
        // unix 上反斜杠不是分隔符 —— 这只是几个怪文件名（不得误拒，更不得越界）。
        push("C:\\evil.txt".into(), VecKind::Fresh);
        push("C:evil.txt".into(), VecKind::Fresh);
        push("\\\\server\\share\\x".into(), VecKind::Fresh);
    }

    // ── 现实层：root 内指向外部的 symlink ──
    if fx.symlinks {
        push("link.txt".into(), VecKind::Symlink);
        push("link_dir".into(), VecKind::Symlink);
        push("link_dir/secret.txt".into(), VecKind::Symlink);
        // 悬空 symlink：canonicalize 失败 → 取最近存在祖先（root）→ 余量拼回，仍在 root 内。
        push("dangling".into(), VecKind::Fresh);
    }

    // ── URL 编码形：模块不做百分号解码，它们只是合法怪文件名 ──
    push("%2e%2e/outside/secret.txt".into(), VecKind::Fresh);
    push("%2e%2e%2foutside%2fsecret.txt".into(), VecKind::Fresh);
    push("..%2foutside/secret.txt".into(), VecKind::Fresh);
    // ── Unicode 归一化形（全角点 U+FF0E 等）：同样只是怪文件名 ──
    push("\u{FF0E}\u{FF0E}/outside/secret.txt".into(), VecKind::Fresh);
    push(
        "\u{FF0E}\u{FF0E}\u{FF0E}\u{FF0E}/outside/secret.txt".into(),
        VecKind::Fresh,
    );
    push("a\u{FF0E}\u{FF0E}/b".into(), VecKind::Fresh);
    push("\u{202E}gnp.exe".into(), VecKind::Fresh);
    // ── `..` 变体：不是 `..` 的合法名（不得误拒）──
    push("....//".into(), VecKind::Fresh);
    push("....//new.txt".into(), VecKind::Fresh);
    push("..foo".into(), VecKind::Fresh);
    push(".hidden".into(), VecKind::Fresh);
    // `. ./` 形（分量是「点+空格」, 不是 `.` 也不是 `..`）
    #[cfg(windows)]
    push(". ./new.txt".into(), VecKind::Odd);
    #[cfg(not(windows))]
    push(". ./new.txt".into(), VecKind::Fresh);
    // ── NUL 截断 / 尾随分隔符 / 目录 → 怪形 ──
    push("target.txt\u{0}".into(), VecKind::Odd);
    push("outside\u{0}/secret.txt".into(), VecKind::Odd);
    push("target.txt/".into(), VecKind::Odd);
    push("sub/".into(), VecKind::Odd);
    push("sub/keep.txt/".into(), VecKind::Odd);
    push(".".into(), VecKind::Odd);
    push("./".into(), VecKind::Odd);
    // ── 工作区内常规路径 ──
    push("target.txt".into(), VecKind::Existing);
    push("dup.txt".into(), VecKind::Existing);
    push("sub/keep.txt".into(), VecKind::Existing);
    push("./target.txt".into(), VecKind::Existing);
    push("new.txt".into(), VecKind::Fresh);
    push("sub/new.txt".into(), VecKind::Fresh);
    push("sub/deep/new.txt".into(), VecKind::Fresh);
    // ── Windows 保留名 / 大小写折叠（平台相关）──
    #[cfg(windows)]
    {
        push("CON".into(), VecKind::Odd);
        push("NUL".into(), VecKind::Odd);
        push("TARGET.TXT".into(), VecKind::Existing);
    }
    #[cfg(not(windows))]
    {
        push("CON".into(), VecKind::Fresh);
        push("NUL".into(), VecKind::Fresh);
        push("TARGET.TXT".into(), VecKind::Fresh);
    }

    vectors
}

/// 向量表自检：分类必须与词法 oracle 一致（防我自己把向量标错 → 假阳性）。
fn assert_vector_classification(vectors: &[PathVec]) {
    for vector in vectors {
        let lexical = looks_absolute(&vector.text) || has_parent_segment(&vector.text);
        match vector.kind {
            VecKind::Lexical => assert!(
                lexical,
                "向量分类自检: {:?} 标成词法越界，但词法 oracle 说不越界",
                vector.text
            ),
            _ => assert!(
                !lexical,
                "向量分类自检: {:?} 标成非越界，但词法 oracle 判它越界",
                vector.text
            ),
        }
    }
}

// ───────────────────────────── 补丁构造 ─────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Action {
    Add,
    UpdateSearchReplace,
    UpdateLineStyle,
    Delete,
}

fn action_from(byte: u8) -> Action {
    match byte % 4 {
        0 => Action::Add,
        1 => Action::UpdateSearchReplace,
        2 => Action::UpdateLineStyle,
        _ => Action::Delete,
    }
}

/// Add 体的 fuzz 内容：逐行去首空白（`decode_add_file_content` 会 `trim_start`，
/// 保留首空白会让期望内容对不上），并限长（防超大字面量拖垮单轮）。
fn sanitize_body(text: &str) -> String {
    let mut body = String::new();
    for line in text.lines() {
        if body.len() > 8192 {
            break;
        }
        body.push_str(line.trim_start());
        body.push('\n');
    }
    body.trim_end_matches('\n').to_string()
}

/// 构造一个动作片段（不含 Begin/End 帧）。
fn build_fragment(
    path: &str,
    action: Action,
    body: &str,
    needle: &str,
    replacement: &str,
) -> String {
    match action {
        Action::Add => {
            let mut fragment = format!("*** Add File: {path}\n");
            for line in body.lines() {
                fragment.push('+');
                fragment.push_str(line);
                fragment.push('\n');
            }
            fragment
        }
        Action::UpdateSearchReplace => format!(
            "*** Update File: {path}\n<<<<<<< SEARCH\n{needle}\n=======\n{replacement}\n>>>>>>>\n"
        ),
        Action::UpdateLineStyle => {
            format!("*** Update File: {path}\n@@\n-{needle}\n+{replacement}\n")
        }
        Action::Delete => format!("*** Delete File: {path}\n"),
    }
}

fn wrap(fragment: &str) -> String {
    format!("*** Begin Patch\n{fragment}*** End Patch")
}

// ───────────────────────────── 断言实现 ─────────────────────────────

/// C1–C5：解析即合同。
fn assert_parse_contract(patch: &str, ctx: &str) {
    let trimmed = patch.trim();
    let framed = trimmed.starts_with("*** Begin Patch") && trimmed.ends_with("*** End Patch");
    match TransactionalPatchApplier::parse_patch(patch) {
        Ok(actions) => {
            assert!(framed, "{ctx}: 不合帧的补丁竟解析成功: {patch:?}");
            assert!(!actions.is_empty(), "{ctx}: 解析 Ok 却是零动作（谎帧）");
            for action in &actions {
                match action {
                    FilePatchAction::Add { path, .. } => {
                        assert!(!path.as_os_str().is_empty(), "{ctx}: Add 动作路径为空");
                    }
                    FilePatchAction::Delete { path } => {
                        assert!(!path.as_os_str().is_empty(), "{ctx}: Delete 动作路径为空");
                    }
                    FilePatchAction::Update { path, hunks } => {
                        assert!(!path.as_os_str().is_empty(), "{ctx}: Update 动作路径为空");
                        assert!(!hunks.is_empty(), "{ctx}: Update 动作零 hunk 却解析成功");
                    }
                }
            }
        }
        Err(error) => {
            assert!(
                matches!(error, ApplyPatchError::ParseError(_)),
                "{ctx}: 解析失败必须是 ParseError 帧，实得 {error:?}"
            );
        }
    }
}

/// 成功后的账目核对（C9–C12）。
fn check_success(
    fx: &Fixture,
    parsed: &[FilePatchAction],
    report: &apeireth_tools_canonical::apply_patch::PatchReport,
    root_before: &BTreeMap<String, Entry>,
    ctx: &str,
) {
    assert_eq!(
        report.total_actions,
        report.files_added.len() + report.files_updated.len() + report.files_deleted.len(),
        "{ctx}: 报告账目不自洽"
    );

    let mut added: Vec<String> = Vec::new();
    let mut updated: Vec<String> = Vec::new();
    let mut deleted: Vec<String> = Vec::new();
    let mut groups: BTreeMap<String, Vec<PatchHunk>> = BTreeMap::new();
    let mut delete_paths: BTreeSet<String> = BTreeSet::new();

    for action in parsed {
        match action {
            FilePatchAction::Add { path, content } => {
                added.push(path.to_string_lossy().to_string());
                // C11：Add 落盘内容 == 解析出的 content。
                let on_disk = std::fs::read(target_path(&fx.root, path)).unwrap_or_default();
                assert_eq!(
                    String::from_utf8_lossy(&on_disk),
                    *content,
                    "{ctx}: Add 落盘内容 != 声明内容 (path={})",
                    path.display()
                );
            }
            FilePatchAction::Update { path, hunks } => {
                updated.push(path.to_string_lossy().to_string());
                groups
                    .entry(path.to_string_lossy().to_string())
                    .or_default()
                    .extend(hunks.iter().cloned());
                // C9 前置：成功即不得带空搜索上下文（空 needle ⇒ ContextMismatch）。
                for hunk in hunks {
                    assert!(
                        !hunk.search_context.is_empty(),
                        "{ctx}: apply 成功却带空搜索上下文"
                    );
                }
            }
            FilePatchAction::Delete { path } => {
                deleted.push(path.to_string_lossy().to_string());
                delete_paths.insert(path.to_string_lossy().to_string());
                // C12：删除后目标必须消失。
                assert!(
                    !target_path(&fx.root, path).exists(),
                    "{ctx}: Delete 后目标仍在 (path={})",
                    path.display()
                );
            }
        }
    }

    // C10：报告路径逐项 == 解析出的动作路径。
    assert_eq!(report.files_added, added, "{ctx}: files_added 与动作集不符");
    assert_eq!(
        report.files_updated, updated,
        "{ctx}: files_updated 与动作集不符"
    );
    assert_eq!(
        report.files_deleted, deleted,
        "{ctx}: files_deleted 与动作集不符"
    );

    // C9 + C10：每个 hunk 恰好唯一命中；落盘内容 == 动作按序合成结果。
    for (path, hunks) in &groups {
        // Update + Delete 同路径：提交阶段版本 CAS 拒绝，不会走到成功分支。
        if delete_paths.contains(path) {
            continue;
        }
        let Some(original) = original_of(root_before, path) else {
            continue;
        };
        let mut composed = original;
        for hunk in hunks {
            let occurrences = composed.matches(&hunk.search_context).count();
            assert_eq!(
                occurrences, 1,
                "{ctx}: strict-unique 契约: hunk 命中 {occurrences} 次 (path={path})"
            );
            composed = composed.replacen(&hunk.search_context, &hunk.replacement_content, 1);
        }
        let on_disk =
            std::fs::read_to_string(target_path(&fx.root, Path::new(path))).unwrap_or_default();
        assert_eq!(
            on_disk, composed,
            "{ctx}: Update 落盘内容 != 动作按序合成结果（有动作没落地 / 被静默吞掉） path={path}"
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Expect {
    PathViolation,
    FileNotFound,
    FileAlreadyExists,
    Success,
    Any,
}

fn expect_for(kind: VecKind, action: Action) -> Expect {
    match kind {
        VecKind::Lexical | VecKind::Symlink => Expect::PathViolation,
        VecKind::Fresh => match action {
            Action::Add => Expect::Success,
            _ => Expect::FileNotFound,
        },
        VecKind::Existing => match action {
            Action::Add => Expect::FileAlreadyExists,
            Action::Delete => Expect::Success,
            _ => Expect::Any,
        },
        VecKind::Odd => Expect::Any,
    }
}

/// 主检查：跑一次 apply，核对越界不变量、事务原子性与期望变体。
fn apply_and_check(fx: &Fixture, patch: &str, expect: Expect, ctx: &str) {
    let root_before = snapshot(&fx.root);
    let outside_before = snapshot(&fx.outside);

    let parsed = TransactionalPatchApplier::parse_patch(patch);
    let result = TransactionalPatchApplier::apply(&fx.root, patch);

    // C13：工作区外目录必须逐项不变（逃逸一旦发生必有痕迹）。
    let outside_after = snapshot(&fx.outside);
    assert!(
        outside_before == outside_after,
        "{ctx}: 工作区外目录发生变化 —— 路径逃逸！\n前: {outside_before:?}\n后: {outside_after:?}\npatch: {patch:?}"
    );

    match result {
        Ok(report) => {
            let actions = parsed
                .expect("apply 成功 ⇒ parse 必须成功（同一份文本、同一解析器）")
                .clone();
            check_success(fx, &actions, &report, &root_before, ctx);
            match expect {
                Expect::PathViolation => {
                    panic!("{ctx}: 越界路径竟被接受（期望 PathViolation）\npatch: {patch:?}")
                }
                Expect::FileNotFound => {
                    panic!("{ctx}: 期望 FileNotFound 却成功\npatch: {patch:?}\nreport: {report:?}")
                }
                Expect::FileAlreadyExists => panic!(
                    "{ctx}: 期望 FileAlreadyExists 却成功\npatch: {patch:?}\nreport: {report:?}"
                ),
                Expect::Success | Expect::Any => {}
            }
        }
        Err(error) => {
            // 错误帧必须非空（给帧，不静默跳过）。
            let message = error.to_string();
            assert!(!message.trim().is_empty(), "{ctx}: 错误帧为空");

            // C8：预演阶段拒绝 ⇒ 磁盘零变化（全有或全无）。
            let pre_commit = matches!(
                error,
                ApplyPatchError::ParseError(_)
                    | ApplyPatchError::PathViolation(_)
                    | ApplyPatchError::FileNotFound(_)
                    | ApplyPatchError::FileAlreadyExists(_)
                    | ApplyPatchError::ContextMismatch(_)
                    | ApplyPatchError::AmbiguousMatch { .. }
                    | ApplyPatchError::GateDenied(_)
            );
            if pre_commit {
                let root_after = snapshot(&fx.root);
                assert!(
                    same_files_and_links(&root_before, &root_after),
                    "{ctx}: 拒绝后工作区仍有半成品\n前: {root_before:?}\n后: {root_after:?}\npatch: {patch:?}"
                );
            }

            if let ApplyPatchError::AmbiguousMatch { occurrences, .. } = &error {
                assert!(
                    *occurrences >= 2,
                    "{ctx}: AmbiguousMatch 的 occurrences 必须 >= 2，实得 {occurrences}"
                );
            }

            match expect {
                Expect::PathViolation => assert!(
                    matches!(error, ApplyPatchError::PathViolation(_)),
                    "{ctx}: 期望 PathViolation，实得 {error:?}\npatch: {patch:?}"
                ),
                Expect::FileNotFound => assert!(
                    matches!(error, ApplyPatchError::FileNotFound(_)),
                    "{ctx}: 期望 FileNotFound，实得 {error:?}\npatch: {patch:?}"
                ),
                Expect::FileAlreadyExists => assert!(
                    matches!(error, ApplyPatchError::FileAlreadyExists(_)),
                    "{ctx}: 期望 FileAlreadyExists，实得 {error:?}\npatch: {patch:?}"
                ),
                Expect::Success => panic!("{ctx}: 期望成功却失败: {error:?}\npatch: {patch:?}"),
                Expect::Any => {}
            }
        }
    }
}

// ───────────────────────────── fuzz 主体 ─────────────────────────────

static SEQ: AtomicU64 = AtomicU64::new(0);

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let Some(fx) = Fixture::setup(SEQ.fetch_add(1, Ordering::Relaxed)) else {
        return;
    };
    let text = String::from_utf8_lossy(data);
    let body = sanitize_body(&text);
    let vectors = path_vectors(&fx);
    assert_vector_classification(&vectors);

    let mode = data[0] % 5;
    match mode {
        0 => {
            // 单动作：向量 + 动作类型 + 内容都由 fuzz 字节选。
            let vector = &vectors[data[1] as usize % vectors.len()];
            let action = action_from(data[2]);
            let strict = matches!(vector.kind, VecKind::Lexical | VecKind::Symlink);
            let needle = pick_needle(data, &text, strict);
            let replacement = pick_replacement(data, &text, strict);
            let fragment = build_fragment(&vector.text, action, &body, &needle, &replacement);
            let patch = wrap(&fragment);
            assert_parse_contract(&patch, "单动作");
            apply_and_check(
                &fx,
                &patch,
                expect_for(vector.kind, action),
                &format!("单动作 kind={:?} action={:?}", vector.kind, action),
            );
        }
        1 => {
            // fuzz 字节直接当路径：只在「不可能越界」时真的落盘。
            let raw = text.lines().next().unwrap_or("").trim().to_string();
            if raw.is_empty() {
                fx.cleanup();
                return;
            }
            let action = action_from(data[2]);
            let fragment = build_fragment(&raw, action, &body, "beta", "BETA");
            let patch = wrap(&fragment);
            assert_parse_contract(&patch, "原始路径");
            if path_is_contained_candidate(&raw) {
                apply_and_check(&fx, &patch, Expect::Any, "原始路径");
            }
        }
        2 => {
            // 双动作：跨文件交互（事务原子性 / 报告账目）。
            let first = &vectors[data[1] as usize % vectors.len()];
            let second = &vectors[data[2] as usize % vectors.len()];
            let action_a = action_from(data[3]);
            let action_b = action_from(data[3] / 4 + 1);
            let fragment_a = build_fragment(&first.text, action_a, &body, "beta", "BETA");
            let fragment_b = build_fragment(&second.text, action_b, &body, "dup", "DUP");
            let patch = wrap(&format!("{fragment_a}{fragment_b}"));
            assert_parse_contract(&patch, "双动作");
            apply_and_check(&fx, &patch, Expect::Any, "双动作");
        }
        3 => {
            // 同一路径被两个 Update 段声明：每个动作都该落地（不许静默吞一个）。
            let first = ["alpha", "beta", "gamma"][data[1] as usize % 3];
            let second = ["alpha", "beta", "gamma"][data[2] as usize % 3];
            let patch = wrap(&format!(
                "*** Update File: target.txt\n@@\n-{first}\n+FIRST\n\
                 *** Update File: target.txt\n@@\n-{second}\n+SECOND\n"
            ));
            assert_parse_contract(&patch, "重复声明");
            apply_and_check(&fx, &patch, Expect::Any, "重复声明");
        }
        _ => {
            // 原始补丁文本：解析面全覆盖；路径越界的文本只解析不落盘。
            let patch = text.to_string();
            assert_parse_contract(&patch, "原始文本");
            let safe = match TransactionalPatchApplier::parse_patch(&patch) {
                Ok(actions) => actions
                    .iter()
                    .all(|action| path_is_contained_candidate(action_path(action))),
                Err(_) => true,
            };
            if safe {
                apply_and_check(&fx, &patch, Expect::Any, "原始文本");
            }
        }
    }

    fx.cleanup();
});

fn action_path(action: &FilePatchAction) -> &str {
    match action {
        FilePatchAction::Add { path, .. }
        | FilePatchAction::Delete { path }
        | FilePatchAction::Update { path, .. } => path.to_str().unwrap_or(""),
    }
}

/// needle：默认取固定表（保证命中/不命中/歧义三类结局都打到），
/// 非严格向量时偶尔改用 fuzz 文本首行（可能破坏补丁帧 → 只断言不 panic）。
fn pick_needle(data: &[u8], text: &str, strict: bool) -> String {
    if strict || (data[3] & 1) == 0 {
        FIXED_NEEDLES[(data[3] >> 1) as usize % FIXED_NEEDLES.len()].to_string()
    } else {
        text.lines().next().unwrap_or("").trim().to_string()
    }
}

fn pick_replacement(data: &[u8], text: &str, strict: bool) -> String {
    if strict || (data[3] & 2) == 0 {
        "REPL".to_string()
    } else {
        text.lines().nth(1).unwrap_or("").trim().to_string()
    }
}
