#![no_main]

//! fuzz: 工具结果**溢出存储** (`spill`) + **对象级访问判定** (`object_access`)。
//!
//! **面 A —— 溢出存储** (`crates/capabilities/tools/src/spill.rs`):
//! 工具输出超阈值 ⇒ 溢出到**会话私有文件**，模型只拿到**会话内相对引用**。
//! `SpillStore` 不拥有会话生命周期，只把调用方给出的**会话标识当作存储授权边界**
//! ⇒ 跨会话不得互相读到对方的溢出文件（授权边界，不是命名约定）。
//!
//! **面 B —— 对象级判定** (`crates/capabilities/tools/src/object_access.rs`):
//! 判定升到**对象级** —— 权力挂在「哪个对象、哪个动作」上，与调用形态无关：
//! 同一对象的同一动作，不管经哪条 API 路径落地，判定逐一相同。
//!
//! **判据（全部来自被测文件自己写明的契约 / 仓内自带单测，不自造期望值）**:
//!
//! | # | 断言 | 契约出处 |
//! |---|------|----------|
//! | A1 | 净化结果只含字母数字与安全符号（无路径分隔符 / 无 `.`）；空名回落 `spill` | `spill.rs:82-83`「只保留字母数字与安全符号, 移除路径分隔符与 `..`」；`:95-99`；自带单测 `:551-555` |
//! | A2 | **阈值语义**：≤ 阈值 ⇒ 原样返回且**不落盘**；> 阈值 ⇒ 落盘一次 + 给引用 | `spill.rs:33-34`「超过该字符数 → spill」；`:249-250`「若内容超出阈值则溢出并返回提示文本, 否则原样返回」；`:252-259` |
//! | A3 | **会话隔离**：异会话标识读对方引用 ⇒ 绝不得到对方内容（可判定后置条件） | `spill.rs:4-6`「读取时必须同时提供同一个会话标识」/「只把调用方给出的会话标识作为存储授权边界」 |
//! | A4 | **引用只含会话内相对路径**：单组件、非绝对、无 `..`、无分隔符 | `spill.rs:146-149`「返回**会话内相对** spill 引用, 而不是绝对路径」；`:10-11`「绝不接受一个可跨会话复用的绝对路径」 |
//! | A5 | **读面不得越出会话目录**：`..` / 多组件 / 绝对路径 / 符号链接 ⇒ 一律拒 | `spill.rs:209-212`「绝对路径、多组件路径、`..` 和符号链接都会被拒绝」；`:219-223` |
//! | A6 | **写面只在会话私有目录内** + 文件 `0600` / 目录 `0700` | `spill.rs:12-13`「spill 文件通过 `create_new` 独占创建；Unix 上文件以 `0o600`、新建目录以 `0o700` 的模式创建」 |
//! | A7 | 策略拒绝时**不得把原始大内容回注上下文** | `spill.rs:249-251`「当有界策略拒绝 spill 时，不会将原始的大内容回退到上下文中」 |
//! | A8 | 会话目录 = 会话 UTF-8 字节的**无碰撞**十六进制编码 ⇒ Unicode 归一化等价标识互不可见 | `spill.rs:10-11`「会话目录由会话 UTF-8 字节的无碰撞十六进制编码组成」 |
//! | A9 | 超长会话标识 ⇒ 拒（不 panic） | `spill.rs:36-40`「`SpillStore` 接受的最大会话标识字节数」；`:424-429` |
//! | A10 | 任意字节（畸形 UTF-8 / NUL / 超长 / 分隔符）⇒ 不 panic | 通用 fail-closed 底线 |
//! | B1 | **判定矩阵**：域内普通对象放行；敏感对象一律拒；域外对象一律拒（与动作种类无关） | `object_access.rs:386-387`「域内普通对象放行; 敏感对象一律拒; 域外对象一律拒」；`:246-249` fail-closed |
//! | B2 | **域界拼写**：`rootish/…` / `root/../escape.txt` ⇒ 域外；`root/sub/../notes.txt` ⇒ 域内 | `object_access.rs:246-249`「域外 / 拼写对不上域根 = OutsideWorkspace (fail-closed)」；仓内自带单测 `crates/capabilities/tools/tests/object_access_judgment.rs:703-718` |
//! | B3 | **对象标识等价类**：同一对象的不同写法（`.` / `..` / 冗余分隔符 / 尾随斜杠）⇒ 域判定与裁决**逐一相同** | `object_access.rs:4-6`「同一对象的同一动作, 不管经哪条 API 路径落地, 判定逐一相同」；`:246-249` 词法归一 |
//! | B4 | **动作词表闭合**：四动作、词面互异；事实差分**只产生前三类**（无枚举） | `object_access.rs:60-63`「对象动作词表 (冻结四动作)」/「事实差分只产生前三类」；`:78` |
//! | B5 | 事实差分 = 纯函数 + 按对象键序确定 + 变更对象 = 域根拼接键 | `object_access.rs:199-202`「纯函数 (零 IO、零策略), 输出按对象键序确定」；自带单测 `:889-918` |
//! | B6 | 事实面越界 ⇒ **失败即帧**（`pipeline.post_block` + source）+ **逐条审计行** | `object_access.rs:18-22`「越界即帧 + 审计」；`:54-58`；`:667-689` |
//! | B7 | 合规对象变更 ⇒ 零违规、零失败 | `object_access.rs:182`「合规执行返回 `None` (结果零变化)」；`:662-665` |
//! | B8 | **形态无关**（事实面）：同一对象同一创建事实，两种 API 形态 ⇒ 报告逐一相同 | `object_access.rs:4-6`；仓内自带单测 `tests/object_access_judgment.rs:271-362` |
//! | B9 | **fail-closed**：判定窗开不出 ⇒ 受控执行**不启动** | `object_access.rs:758-763`「快照不可得 = `ObjectGuardError` (fail-closed)」；`:776-778`；`:619-625` |
//! | B10 | **策略链 deny 优先**：链上有拒绝即拒绝（顺序无关）；全部放行才放行 | `object_access.rs:410-412`「顺序判定, **第一个拒绝即拒绝**, 全部放行才放行」；`:446-452` |
//! | B11 | 域外对象不得被叠加的恒放行策略翻成放行 | `object_access.rs:386-387`（域外一律拒、fail-closed）+ `:446-452` |
//! | B12 | 任意字节（畸形 UTF-8 / NUL / 超长 / 分隔符）⇒ 不 panic | 通用 fail-closed 底线 |
//!
//! **刻意不写成断言的（见交付报告）**：符号链接别名（域内 symlink 指向域外对象）——
//! 被测模块把域判定明写为「**词法**归一判定」，域外访问由「请求时判定 + 沙箱物理围栏」
//! 兜住（`object_access.rs:32-34`）；把「词法 ⇒ 现实层」自行加进来会变成自造期望。

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, UNIX_EPOCH};

use libfuzzer_sys::fuzz_target;

use apeireth_tools_canonical::exec_pipeline::PipelineFailure;
use apeireth_tools_canonical::object_access::{
    OBJECT_JUDGE_SOURCE, OBJECT_VIOLATION_AUDIT_EVENT, ObjectAccessJudge, ObjectAccessPolicy,
    ObjectAccessRequest, ObjectAction, ObjectChange, ObjectDomain, ObjectFingerprint,
    ObjectGuardError, ObjectKind, ObjectPolicyChain, ObjectSnapshot, ObjectSubject, ObjectVerdict,
    ObjectZone, WorkspaceObjectPolicy, diff_object_states,
};
use apeireth_tools_canonical::spill::{
    MAX_SESSION_ID_BYTES, SpillPolicy, SpillStore, safe_segment,
};

/// 每轮迭代的唯一序号 —— 临时目录 / 会话标识都必须本次运行独有
/// （规范 §3.4「往共享目录写固定文件名 ⇒ 撞车」）。
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_counter() -> u64 {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// 畸形字节 ⇒ 字符串（`from_utf8_lossy` 保留替换字符，不 panic）。
fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// 会话标识：fuzz 字节（截断到 ≤16 原始字节 ⇒ ≤48 编码字节）+ 唯一后缀 ⇒ 恒 ≤ 96 字节。
///
/// 契约 `spill.rs:36-40`：`MAX_SESSION_ID_BYTES = 96`；超长必须被拒（单独一条断言），
/// 这里保证「正常路径」始终落在上限内，免得把上限分支混进隔离断言。
fn session_id(raw: &[u8], tag: &str, counter: u64) -> String {
    let head = lossy(&raw[..raw.len().min(16)]);
    format!("{head}-{tag}-{counter}")
}

/// 递归统计根目录下**普通文件**中名字以 `spill-` 开头的个数（= 已落盘的溢出文件数）。
///
/// 符号链接不计入（`DirEntry::metadata` 不跟随符号链接），避免把夹具算进落盘计数。
fn spill_file_count(root: &Path) -> usize {
    let mut count = 0usize;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                stack.push(path);
            } else if entry.file_name().to_string_lossy().starts_with("spill-") {
                count += 1;
            }
        }
    }
    count
}

/// 在根目录下找出内容等于 `needle` 的普通文件（用于定位溢出文件 ⇒ 反推会话目录）。
fn locate_file(root: &Path, needle: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }
            if let Ok(text) = fs::read_to_string(&path) {
                if text == needle {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// 根目录之外落下的普通文件（必须为空 ⇒ 溢出只落会话私有目录内）。
fn files_outside(root: &Path, inside: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }
            if metadata.file_type().is_symlink() {
                continue;
            }
            if !path.starts_with(inside) {
                found.push(path);
            }
        }
    }
    found
}

/// 恒放行策略（策略链对照面）。
struct AlwaysAllow;

impl ObjectAccessPolicy for AlwaysAllow {
    fn name(&self) -> &str {
        "fuzz_always_allow"
    }

    fn decide(&self, _request: &ObjectAccessRequest<'_>) -> ObjectVerdict {
        ObjectVerdict::Allow
    }
}

/// 恒拒绝策略（策略链对照面）。
struct AlwaysDeny;

impl ObjectAccessPolicy for AlwaysDeny {
    fn name(&self) -> &str {
        "fuzz_always_deny"
    }

    fn decide(&self, _request: &ObjectAccessRequest<'_>) -> ObjectVerdict {
        ObjectVerdict::deny("fuzz always deny")
    }
}

// ---------------------------------------------------------------------------
// 面 A: 工具结果溢出存储 (`spill`)
// ---------------------------------------------------------------------------

fn fuzz_spill(data: &[u8]) {
    let parts: Vec<&[u8]> = data.split(|byte| *byte == 0).collect();
    let part = |index: usize| -> &[u8] { parts.get(index).copied().unwrap_or(&[]) };
    let counter = next_counter();

    let base = std::env::temp_dir().join(format!(
        "apeireth-fuzz-spill-{}-{counter}",
        std::process::id()
    ));
    let root = base.join("store");
    let store = SpillStore::with_root(&root);

    // ── A1. 净化契约 (契约: spill.rs:82-83 「只保留字母数字与安全符号, 移除路径分隔符与 `..`」)
    let raw_name = lossy(part(0));
    let safe = safe_segment(&raw_name);
    assert!(
        !safe.is_empty(),
        "净化结果不得为空 (契约: spill.rs:95-99 空名回落 `spill`): {raw_name:?}"
    );
    assert!(
        safe.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_'),
        "净化结果只允许字母数字与安全符号 (契约: spill.rs:82-83): {safe:?}"
    );
    assert!(
        !safe.contains('/') && !safe.contains('\\') && !safe.contains('.'),
        "净化结果不得含路径分隔符或 `.` (契约: spill.rs:82-83 移除路径分隔符与 `..`): {safe:?}"
    );
    assert_eq!(
        safe_segment(""),
        "spill",
        "空名必须回落 `spill` (契约: spill.rs:95-99 + 自带单测 spill.rs:551-555)"
    );

    // ── A2. 阈值语义 (契约: spill.rs:33-34 / :249-259)
    let session_threshold = session_id(part(1), "thr", counter);
    let body = lossy(part(2));
    let content = format!("THR-{counter}:{body}");
    let char_count = content.chars().count();
    let delta = usize::from(part(3).first().copied().unwrap_or(0)) % 5;
    let threshold = char_count.saturating_sub(delta);

    let files_before = spill_file_count(&root);
    let rendered = store.maybe_spill(&session_threshold, "grep_tool", &content, threshold);
    let files_after = spill_file_count(&root);

    if char_count <= threshold {
        assert_eq!(
            rendered, content,
            "≤阈值必须原样返回 (契约: spill.rs:252-259 「否则原样返回」)"
        );
        assert_eq!(
            files_before, files_after,
            "≤阈值不得落盘 (契约: spill.rs:33-34 「超过该字符数 → spill」)"
        );
    } else {
        assert_ne!(
            rendered, content,
            ">阈值必须溢出 (契约: spill.rs:252-259 「若内容超出阈值则溢出」)"
        );
        assert_eq!(
            files_after,
            files_before + 1,
            ">阈值必须落盘一次 (契约: spill.rs:33-34)"
        );
        let reference = locate_file(&root, &content)
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .expect("溢出文件必须存在");
        assert!(
            rendered.contains(&reference),
            "提示文本必须给出会话内相对引用 (契约: spill.rs:4-5 「模型只得到一个会话内相对引用」): {rendered:?}"
        );
        let root_text = root.to_string_lossy().into_owned();
        if !content.contains(&root_text) {
            assert!(
                !rendered.contains(&root_text),
                "模型可见文本不得泄漏绝对路径 (契约: spill.rs:146-149 「返回会话内相对 spill 引用, 而不是绝对路径」)"
            );
        }
    }

    // ── A3. 会话隔离 (契约: spill.rs:4-6 「读取时必须同时提供同一个会话标识」)
    let session_a = session_id(part(4), "a", counter);
    let session_b = session_id(part(5), "b", counter);
    let content_a = format!("A-{counter}:{body}");
    let content_b = format!("B-{counter}:{body}");
    let reference_a = store
        .spill(&session_a, &raw_name, &content_a)
        .expect("会话 A 溢出必须成功");
    let reference_b = store
        .spill(&session_b, &raw_name, &content_b)
        .expect("会话 B 溢出必须成功");

    assert_eq!(
        store
            .read_for_session(&session_a, &reference_a)
            .expect("同会话读取必须成功"),
        content_a,
        "同会话标识必须读得到自己的溢出内容 (契约: spill.rs:146-149)"
    );

    // 越权读的可判定后置条件：**绝不**得到对方的内容（撞名时读到的是对方自己的文件，
    // 那也是对的 ⇒ 判据用「内容」而不是「必然报错」）。
    match store.read_for_session(&session_b, &reference_a) {
        Err(_) => {}
        Ok(leaked) => assert_ne!(
            leaked, content_a,
            "跨会话读取拿到了会话 A 的内容 —— 会话隔离被突破 (契约: spill.rs:4-6 「会话标识作为存储授权边界」)"
        ),
    }
    match store.read_for_session(&session_a, &reference_b) {
        Err(_) => {}
        Ok(leaked) => assert_ne!(
            leaked, content_b,
            "跨会话读取拿到了会话 B 的内容 —— 会话隔离被突破 (契约: spill.rs:4-6 「会话标识作为存储授权边界」)"
        ),
    }

    // ── A4. 引用形态: 会话内相对单文件引用 (契约: spill.rs:146-149 / :10-11)
    for reference in [&reference_a, &reference_b] {
        let path = Path::new(reference);
        assert!(
            !path.is_absolute(),
            "引用不得是绝对路径 (契约: spill.rs:10-11 「绝不接受一个可跨会话复用的绝对路径」): {reference:?}"
        );
        let mut components = path.components();
        let first = components.next().expect("引用非空");
        assert!(
            components.next().is_none(),
            "引用必须是单组件相对路径 (契约: spill.rs:146-149): {reference:?}"
        );
        assert!(
            matches!(first, Component::Normal(_)),
            "引用必须是普通文件名（不得是 `.` / `..` / 根） (契约: spill.rs:146-149): {reference:?}"
        );
        assert!(
            !reference.contains('/') && !reference.contains('\\'),
            "引用不得含路径分隔符 (契约: spill.rs:146-149): {reference:?}"
        );
        assert!(
            !reference.contains(".."),
            "引用不得含父目录 (契约: spill.rs:209-212): {reference:?}"
        );
    }

    // ── A5. 读面不得越出会话目录 (契约: spill.rs:209-212)
    let file_a = locate_file(&root, &content_a).expect("会话 A 溢出文件必须存在");
    for hostile in [
        "../outside",
        "..",
        ".",
        "",
        "a/b",
        "/etc/passwd",
        "spill-x/../spill-y",
    ] {
        assert!(
            store.read_for_session(&session_a, hostile).is_err(),
            "越界引用必须被拒 (契约: spill.rs:209-212 「绝对路径、多组件路径、`..` 和符号链接都会被拒绝」): {hostile:?}"
        );
    }
    assert!(
        store
            .read_for_session(&session_a, &file_a.to_string_lossy())
            .is_err(),
        "绝对路径必须被拒 (契约: spill.rs:209-212)"
    );

    // ── A6. 写面: 只落在会话私有目录 + 权限 (契约: spill.rs:12-13)
    let session_dir_a = file_a.parent().expect("溢出文件必须有父目录").to_path_buf();
    assert!(
        file_a.starts_with(&root),
        "溢出文件必须落在 spill 根目录内 (契约: spill.rs:12-13): {file_a:?}"
    );
    assert_eq!(
        file_a.parent().and_then(|parent| parent.parent()),
        Some(root.as_path()),
        "溢出文件必须落在会话私有目录 (根的下一层) (契约: spill.rs:12-13): {file_a:?}"
    );
    assert!(
        files_outside(&base, &root).is_empty(),
        "溢出不得落在会话私有目录之外 (契约: spill.rs:12-13 「会话私有文件」): {:?}",
        files_outside(&base, &root)
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let file_mode = fs::metadata(&file_a)
            .expect("读取溢出文件元数据必须成功")
            .permissions()
            .mode();
        assert_eq!(
            file_mode & 0o077,
            0,
            "溢出文件不得对组/其他可读写 (契约: spill.rs:12-13 「文件以 `0o600` … 的模式创建」)"
        );
        let dir_mode = fs::metadata(&session_dir_a)
            .expect("读取会话目录元数据必须成功")
            .permissions()
            .mode();
        assert_eq!(
            dir_mode & 0o077,
            0,
            "会话目录不得对组/其他可读写 (契约: spill.rs:12-13 「新建目录以 `0o700` 的模式创建」)"
        );
        let root_mode = fs::metadata(&root)
            .expect("读取 spill 根目录元数据必须成功")
            .permissions()
            .mode();
        assert_eq!(
            root_mode & 0o077,
            0,
            "spill 根目录不得对组/其他可读写 (契约: spill.rs:12-13)"
        );
    }

    // ── A5b. 符号链接必须被拒 (契约: spill.rs:209-212 / :219-223)
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let link_inside = session_dir_a.join("spill-link-inside");
        let _ = fs::remove_file(&link_inside);
        symlink(&file_a, &link_inside).expect("创建符号链接夹具必须成功");
        assert!(
            store
                .read_for_session(&session_a, "spill-link-inside")
                .is_err(),
            "会话内符号链接必须被拒 (契约: spill.rs:209-212 / :219-223 「拒绝读取符号链接 spill 引用」)"
        );

        let outside_target = base.join("outside-secret.txt");
        fs::write(&outside_target, "OUTSIDE-OF-SESSION").expect("写入域外夹具必须成功");
        let link_outside = session_dir_a.join("spill-link-outside");
        let _ = fs::remove_file(&link_outside);
        symlink(&outside_target, &link_outside).expect("创建符号链接夹具必须成功");
        assert!(
            store
                .read_for_session(&session_a, "spill-link-outside")
                .is_err(),
            "指向会话外的符号链接必须被拒 (契约: spill.rs:209-212)"
        );

        let file_b = locate_file(&root, &content_b).expect("会话 B 溢出文件必须存在");
        let link_cross = session_dir_a.join("spill-link-cross");
        let _ = fs::remove_file(&link_cross);
        symlink(&file_b, &link_cross).expect("创建符号链接夹具必须成功");
        assert!(
            store
                .read_for_session(&session_a, "spill-link-cross")
                .is_err(),
            "跨会话符号链接必须被拒 (契约: spill.rs:209-212 / :146-149)"
        );
    }

    // ── A7. 策略拒绝时不得回落原文 (契约: spill.rs:249-251)
    {
        let tight_root = base.join("tight");
        let tight = SpillStore::with_policy(
            &tight_root,
            SpillPolicy {
                max_file_bytes: 1,
                max_session_bytes: 1,
                max_session_files: 1,
                ttl: Duration::from_secs(60),
            },
        );
        let marker = format!("FALLBACK-MARKER-{counter}");
        let big = format!("{marker}-{}", "x".repeat(64));
        let refused = tight.maybe_spill(&session_a, "grep_tool", &big, 4);
        assert!(
            !refused.contains(&marker),
            "策略拒绝时不得把原始大内容回注上下文 (契约: spill.rs:249-251 「不会将原始的大内容回退到上下文中」): {refused:?}"
        );
    }

    // ── A8. Unicode 归一化碰撞 (契约: spill.rs:10-11 「无碰撞十六进制编码」)
    {
        let nfc = format!("caf\u{e9}-u-{counter}");
        let nfd = format!("cafe\u{301}-u-{counter}");
        let content_nfc = format!("NFC-{counter}");
        let content_nfd = format!("NFD-{counter}");
        let reference_nfc = store
            .spill(&nfc, "tool", &content_nfc)
            .expect("NFC 会话溢出必须成功");
        let reference_nfd = store
            .spill(&nfd, "tool", &content_nfd)
            .expect("NFD 会话溢出必须成功");
        match store.read_for_session(&nfd, &reference_nfc) {
            Err(_) => {}
            Ok(leaked) => assert_ne!(
                leaked, content_nfc,
                "Unicode 归一化等价的会话标识互相读到了内容 —— 十六进制编码必须无碰撞 (契约: spill.rs:10-11)"
            ),
        }
        match store.read_for_session(&nfc, &reference_nfd) {
            Err(_) => {}
            Ok(leaked) => assert_ne!(
                leaked, content_nfd,
                "Unicode 归一化等价的会话标识互相读到了内容 (契约: spill.rs:10-11)"
            ),
        }
    }

    // ── A9. 超长会话标识必须被拒 (契约: spill.rs:36-40 / :424-429)
    {
        let too_long = "s".repeat(MAX_SESSION_ID_BYTES + 1);
        assert!(
            store.spill(&too_long, "tool", "x").is_err(),
            "超长会话标识必须被拒 (契约: spill.rs:36-40 「接受的最大会话标识字节数」)"
        );
    }

    // ── A10. 任意字节不得 panic
    {
        let weird = lossy(part(6));
        let _ = store.spill(&weird, &weird, &weird);
        let _ = store.read_for_session(&weird, &weird);
        let _ = store.maybe_spill(&weird, &weird, &weird, 0);
        let _ = store.cleanup_expired(&weird);
    }

    let _ = fs::remove_dir_all(&base);
}

// ---------------------------------------------------------------------------
// 面 B: 对象级访问判定 (`object_access`)
// ---------------------------------------------------------------------------

/// fuzz 字节 ⇒ 一个对象路径（域内 / 域外两种形态）。
fn fuzz_object_path(inside: &Path, outside: &Path, raw: &[u8]) -> PathBuf {
    let text = lossy(raw);
    let base = if raw.first().is_some_and(|byte| byte % 2 == 0) {
        outside
    } else {
        inside
    };
    let mut path = base.to_path_buf();
    for segment in text.split('/').take(4) {
        if segment.is_empty() {
            continue;
        }
        path.push(segment);
    }
    path
}

/// 同一对象的等价写法（词法归一后同键）。
fn equivalent_spellings(root: &Path, name: &str) -> Vec<PathBuf> {
    let root_text = root.to_string_lossy().into_owned();
    vec![
        root.join(name),
        root.join(".").join(name),
        root.join("sub").join("..").join(name),
        PathBuf::from(format!("{root_text}//{name}")),
        PathBuf::from(format!("{root_text}/{name}/")),
        PathBuf::from(format!("{root_text}/./{name}")),
    ]
}

/// fuzz 驱动的快照对（同一键集、不同指纹 ⇒ 覆盖「不变 / 改长度 / 改 mtime」三态）。
fn snapshot_pair(raw: &[u8]) -> (ObjectSnapshot, ObjectSnapshot) {
    let mut before = ObjectSnapshot::new("root");
    let mut after = ObjectSnapshot::new("root");
    for (index, chunk) in raw.chunks(3).take(5).enumerate() {
        let key = format!("obj-{index}");
        let len = u64::from(chunk.first().copied().unwrap_or(0));
        let base = ObjectFingerprint {
            kind: ObjectKind::File,
            len,
            mtime: UNIX_EPOCH,
        };
        let changed = match chunk.get(1).copied().unwrap_or(0) % 3 {
            0 => base,
            1 => ObjectFingerprint {
                kind: ObjectKind::File,
                len: len.wrapping_add(1),
                mtime: UNIX_EPOCH,
            },
            _ => ObjectFingerprint {
                kind: ObjectKind::File,
                len,
                mtime: UNIX_EPOCH + Duration::from_secs(1),
            },
        };
        before.insert(key.clone(), base);
        after.insert(key, changed);
    }
    before.insert(
        "only-before",
        ObjectFingerprint {
            kind: ObjectKind::File,
            len: 1,
            mtime: UNIX_EPOCH,
        },
    );
    after.insert(
        "only-after",
        ObjectFingerprint {
            kind: ObjectKind::File,
            len: 1,
            mtime: UNIX_EPOCH,
        },
    );
    (before, after)
}

/// 该快照对**应当**产生的变更集（由指纹规则推出，不依赖实现）。
fn expected_changes(raw: &[u8]) -> Vec<(String, ObjectAction)> {
    let mut expected = Vec::new();
    for (index, chunk) in raw.chunks(3).take(5).enumerate() {
        if chunk.get(1).copied().unwrap_or(0) % 3 != 0 {
            expected.push((format!("root/obj-{index}"), ObjectAction::Modify));
        }
    }
    expected.push(("root/only-after".to_string(), ObjectAction::Create));
    expected.push(("root/only-before".to_string(), ObjectAction::Delete));
    expected.sort_by(|left, right| left.0.cmp(&right.0));
    expected
}

fn fuzz_object_access(data: &[u8]) {
    let parts: Vec<&[u8]> = data.split(|byte| *byte == 0).collect();
    let raw = parts.first().copied().unwrap_or(&[]);
    let counter = next_counter();

    let base = std::env::temp_dir().join(format!(
        "apeireth-fuzz-obj-{}-{counter}",
        std::process::id()
    ));
    let workspace = base.join("ws");
    fs::create_dir_all(workspace.join("sub")).expect("创建工作区夹具必须成功");
    fs::write(workspace.join("notes.txt"), "n").expect("写入工作区夹具必须成功");

    let domain = ObjectDomain::new(workspace.clone());
    let subject = ObjectSubject::new("fuzz-object-process");
    let policy = WorkspaceObjectPolicy;
    let judge = ObjectAccessJudge::with_defaults();

    // ── B1. 判定矩阵 (契约: object_access.rs:386-387)
    let object = fuzz_object_path(&workspace, &base, raw);
    for action in ObjectAction::ALL {
        let request = domain.request(&subject, &object, action);
        assert_eq!(
            request.zone,
            domain.zone_of(&object),
            "请求面填的域必须与域判定一致 (契约: object_access.rs:263-276)"
        );
        let verdict = policy.decide(&request);
        match request.zone {
            ObjectZone::InsideWorkspace => assert!(
                verdict.is_allow(),
                "域内普通对象必放行 (契约: object_access.rs:386-387): {action:?} {object:?}"
            ),
            ObjectZone::Sensitive => assert!(
                !verdict.is_allow(),
                "敏感对象必拒 (契约: object_access.rs:386-387): {action:?} {object:?}"
            ),
            ObjectZone::OutsideWorkspace => assert!(
                !verdict.is_allow(),
                "域外对象必拒 (契约: object_access.rs:386-387 fail-closed): {action:?} {object:?}"
            ),
        }
        assert_eq!(
            judge.check(&request),
            verdict,
            "判定器与策略必须同判 (契约: object_access.rs:753-756 「与 API 形态无关」)"
        );
    }

    // ── B2. 域界拼写形态 (契约: object_access.rs:246-249 + 仓内自带单测
    //    crates/capabilities/tools/tests/object_access_judgment.rs:703-718)
    {
        let lexical = ObjectDomain::new(Path::new("root"));
        assert_eq!(
            lexical.zone_of(Path::new("rootish/notes.txt")),
            ObjectZone::OutsideWorkspace,
            "拼写对不上域根 ⇒ 域外 (契约: object_access.rs:246-249)"
        );
        assert_eq!(
            lexical.zone_of(Path::new("root/../escape.txt")),
            ObjectZone::OutsideWorkspace,
            "折叠后落到域外 ⇒ 域外 (契约: object_access.rs:246-249)"
        );
        assert_eq!(
            lexical.zone_of(Path::new("root/sub/../notes.txt")),
            ObjectZone::InsideWorkspace,
            "折叠后回落域内 ⇒ 域内 (契约: object_access.rs:246-249 词法归一)"
        );
    }

    // ── B3. 对象标识等价类 (契约: object_access.rs:4-6 / :246-249)
    {
        for (label, root_of_object, name) in [
            ("域内对象", workspace.clone(), "notes.txt"),
            ("域外对象", base.clone(), "outside.txt"),
        ] {
            let spellings = equivalent_spellings(&root_of_object, name);
            let zones: Vec<ObjectZone> =
                spellings.iter().map(|path| domain.zone_of(path)).collect();
            let verdicts: Vec<ObjectVerdict> = spellings
                .iter()
                .map(|path| policy.decide(&domain.request(&subject, path, ObjectAction::Modify)))
                .collect();
            for index in 0..spellings.len() {
                assert_eq!(
                    zones[index], zones[0],
                    "{label}的等价写法域判定必须相同 (契约: object_access.rs:4-6): {:?} vs {:?}",
                    spellings[index], spellings[0]
                );
                assert_eq!(
                    verdicts[index], verdicts[0],
                    "{label}的等价写法裁决必须相同 (契约: object_access.rs:4-6 「判定逐一相同」): {:?} vs {:?}",
                    spellings[index], spellings[0]
                );
            }
        }
    }

    // ── B4. 动作词表闭合 (契约: object_access.rs:60-63 / :78)
    {
        assert_eq!(
            ObjectAction::ALL.len(),
            4,
            "动作词表冻结四动作 (契约: object_access.rs:60-63)"
        );
        let words: Vec<&str> = ObjectAction::ALL
            .iter()
            .map(|action| action.as_str())
            .collect();
        let mut unique = words.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            words.len(),
            "动作词面必须互异 (契约: object_access.rs:80-88): {words:?}"
        );
    }

    // ── B5. 事实差分: 纯函数 + 键序确定 + 只产生前三类 (契约: object_access.rs:62-63 / :199-202)
    {
        let (before, after) = snapshot_pair(raw);
        let changes: Vec<ObjectChange> = diff_object_states(&before, &after);
        assert!(
            changes
                .iter()
                .all(|change| change.action != ObjectAction::Enumerate),
            "事实差分不得产生枚举动作 (契约: object_access.rs:62-63 「事实差分只产生前三类」)"
        );
        let rendered: Vec<(String, ObjectAction)> = changes
            .iter()
            .map(|change| (change.object.to_string_lossy().into_owned(), change.action))
            .collect();
        let mut sorted = rendered.clone();
        sorted.sort_by(|left, right| left.0.cmp(&right.0));
        assert_eq!(
            rendered, sorted,
            "事实差分输出必须按对象键序确定 (契约: object_access.rs:199-202)"
        );
        assert_eq!(
            rendered,
            expected_changes(raw),
            "事实差分的动作分类必须与状态事实一致 (契约: object_access.rs:203-227 + 自带单测 :889-918)"
        );
        assert_eq!(
            diff_object_states(&before, &after),
            changes,
            "事实差分必须是纯函数 (契约: object_access.rs:199-202 「纯函数 (零 IO、零策略)」)"
        );
    }

    // ── B6. 事实面越界 ⇒ 失败即帧 + 审计行 (契约: object_access.rs:18-22 / :54-58 / :667-689)
    {
        let judge = ObjectAccessJudge::with_defaults();
        let sensitive = workspace.join(".env");
        let ((), report) = judge
            .guard(subject.clone(), domain.clone(), || {
                fs::write(&sensitive, "SECRET=1").expect("写入敏感夹具必须成功");
            })
            .expect("判定窗必须开得出");
        assert!(
            report.is_violation(),
            "敏感对象变更必须判违规 (契约: object_access.rs:386-387 「敏感对象一律拒」): {report:?}"
        );
        assert!(report.is_failure(), "违规必须判失败");
        let failure = report.failure().expect("违规必须出帧");
        match failure {
            PipelineFailure::PostBlocked { source, reason } => {
                assert_eq!(
                    source, OBJECT_JUDGE_SOURCE,
                    "失败帧 source 必须是对象判定器 (契约: object_access.rs:54-55 / :686-689)"
                );
                assert!(
                    reason.starts_with("object violation:"),
                    "失败帧理由串必须以 `object violation:` 起头 (契约: object_access.rs:667-668): {reason}"
                );
            }
            other => panic!(
                "失败帧必须走闭合失败词表 `pipeline.post_block` (契约: object_access.rs:686-689): {other:?}"
            ),
        }
        let rows = judge.audit_rows();
        let audit_row = rows
            .iter()
            .find(|row| row.starts_with(OBJECT_VIOLATION_AUDIT_EVENT))
            .expect("违规必须落一行 `sandbox.object_violation` 审计 (契约: object_access.rs:57-58 / :855-857)");
        assert!(
            audit_row.contains("action=create"),
            "审计行必须带动作 (契约: object_access.rs:559-564): {audit_row}"
        );
        assert!(
            audit_row.contains("process=fuzz-object-process"),
            "审计行必须带问责进程 (契约: object_access.rs:559-564): {audit_row}"
        );
        assert!(
            audit_row.contains("zone=sensitive"),
            "审计行必须带域 (契约: object_access.rs:559-564): {audit_row}"
        );
    }

    // ── B7. 合规对象变更零违规 (契约: object_access.rs:182 / :662-665)
    {
        let judge = ObjectAccessJudge::with_defaults();
        let ((), report) = judge
            .guard(subject.clone(), domain.clone(), || {
                fs::write(workspace.join("notes2.txt"), "ok").expect("写入合规夹具必须成功");
            })
            .expect("判定窗必须开得出");
        assert!(
            !report.is_violation(),
            "域内普通对象变更不得判违规 (契约: object_access.rs:386-387): {report:?}"
        );
        assert!(!report.is_failure());
        assert!(report.failure().is_none());
    }

    // ── B8. 形态无关 (事实面): 同一对象同一创建事实, 两种 API 形态 ⇒ 报告逐一相同
    //    (契约: object_access.rs:4-6; 仓内自带单测 tests/object_access_judgment.rs:271-362)
    {
        let judge = ObjectAccessJudge::with_defaults();
        let target = workspace.join("probe.txt");
        let _ = fs::remove_file(&target);
        let ((), form_a) = judge
            .guard(subject.clone(), domain.clone(), || {
                fs::write(&target, "v1").expect("形态 A 写入必须成功");
            })
            .expect("判定窗必须开得出");
        let _ = fs::remove_file(&target);
        let ((), form_b) = judge
            .guard(subject.clone(), domain.clone(), || {
                use std::io::Write as _;
                let mut handle = fs::File::create(&target).expect("形态 B 打开必须成功");
                handle.write_all(b"v1").expect("形态 B 写入必须成功");
            })
            .expect("判定窗必须开得出");
        assert_eq!(
            form_a, form_b,
            "同一对象同一创建事实, 不同 API 形态判定必须逐一相同 (契约: object_access.rs:4-6)"
        );
        let _ = fs::remove_file(&target);
    }

    // ── B9. fail-closed: 判定窗开不出 ⇒ 受控执行不启动 (契约: object_access.rs:758-763 / :776-778)
    {
        let judge = ObjectAccessJudge::with_defaults();
        let gone = base.join("never-created");
        let ran = std::cell::Cell::new(false);
        let outcome = judge.guard(subject.clone(), ObjectDomain::new(gone), || ran.set(true));
        assert!(
            matches!(outcome, Err(ObjectGuardError::Unobservable(_))),
            "判定窗开不出必须 fail-closed (契约: object_access.rs:619-625 / :758-763): {outcome:?}"
        );
        assert!(
            !ran.get(),
            "判定窗开不出 = 受控执行不得启动 (契约: object_access.rs:776-778)"
        );
    }

    // ── B10. 策略链 deny 优先 (契约: object_access.rs:410-412 / :446-452)
    {
        let allow: Arc<dyn ObjectAccessPolicy> = Arc::new(AlwaysAllow);
        let deny: Arc<dyn ObjectAccessPolicy> = Arc::new(AlwaysDeny);
        let notes = workspace.join("notes.txt");
        let request = domain.request(&subject, &notes, ObjectAction::Modify);

        let chain = ObjectPolicyChain::new(vec![Arc::clone(&allow), Arc::clone(&deny)]);
        assert!(
            !chain.decide(&request).is_allow(),
            "链上有拒绝即拒绝 (契约: object_access.rs:410-412 「第一个拒绝即拒绝」)"
        );
        let chain = ObjectPolicyChain::new(vec![Arc::clone(&deny), Arc::clone(&allow)]);
        assert!(
            !chain.decide(&request).is_allow(),
            "链上有拒绝即拒绝, 与判定顺序无关 (契约: object_access.rs:410-412)"
        );
        let chain = ObjectPolicyChain::new(vec![Arc::clone(&allow), Arc::clone(&allow)]);
        assert!(
            chain.decide(&request).is_allow(),
            "全部放行才放行 (契约: object_access.rs:410-412)"
        );
    }

    // ── B11. 域外对象不得被叠加的恒放行策略翻成放行 (契约: object_access.rs:386-387 / :446-452)
    {
        let allow: Arc<dyn ObjectAccessPolicy> = Arc::new(AlwaysAllow);
        let chain = ObjectPolicyChain::new(vec![Arc::new(WorkspaceObjectPolicy), allow]);
        let outside_object = base.join("outside.txt");
        let outside_request = domain.request(&subject, &outside_object, ObjectAction::Modify);
        assert!(
            !chain.decide(&outside_request).is_allow(),
            "域外对象不得被恒放行策略放行 (契约: object_access.rs:386-387 fail-closed)"
        );
    }

    // ── B12. 任意字节不得 panic
    {
        let weird = lossy(raw);
        let _ = domain.zone_of(Path::new(&weird));
        let _ = policy.decide(&domain.request(&subject, Path::new(&weird), ObjectAction::Create));
        let long = "x".repeat(4096);
        let _ = domain.zone_of(Path::new(&long));
    }

    let _ = fs::remove_dir_all(&base);
}

fuzz_target!(|data: &[u8]| {
    let Some((selector, rest)) = data.split_first() else {
        return;
    };
    if *selector % 2 == 0 {
        fuzz_spill(rest);
    } else {
        fuzz_object_access(rest);
    }
});
