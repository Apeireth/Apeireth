#![no_main]

//! fuzz: 敏感路径判定/拒绝面 (`capabilities/tools::sensitive_path`)。
//!
//! **面**: 本地读工具的**敏感面闸门** —— 判定「某条路径是不是凭据/密钥面」
//! ([`is_sensitive_path`])、产出稳定的拒绝帧 ([`credential_surface_refusal`])、
//! 以及对可读配置件的**字段值脱敏** ([`redact_secret_field_values`])。
//! 这个闸门一旦被绕过 = 模型直接读到 `.env` / `creds.json` / 私钥 ⇒ 安全事件。
//!
//! **被测代码怎么进来**: `sensitive_path` 模块是 `mod sensitive_path;` (crate 私有),
//! `is_sensitive_path` 是 `pub(crate)` —— crate 外拿不到。本 target 用
//! **`#[path]` 零复制纳入同一份源文件** (与 `research/verification/kani` mirror、
//! `crates/capabilities/tools/tests/common/mod.rs` 同一范式): 不复制实现、
//! 不改产品代码、测的就是生产源码本身。
//! 文件里两处 crate 内引用面 (`super::mcp_bridge::config::is_secret_key`、
//! `super::exec_pipeline::PipelineFailure`) 由下面的 shim **转发到真 crate 的公开实现**
//! (`apeireth_tools_canonical::…`), 零语义漂移。
//!
//! **判据（全部来自被测文件自己写明的契约，不自造期望值）**:
//!
//! | # | 断言 | 契约出处 |
//! |---|------|----------|
//! | 1 | 敏感词面必拒（`.env` 族 / 密钥材料 / 凭据存储件 / 钥匙串导出物 / 私钥目录），且**任意深度 + 任意后缀**都必拒 | `sensitive_path.rs:8-13` 路径级拒绝；`sensitive_path.rs:23-27` "contains a known sensitive workspace path"；文件自带单测 `:284-362` |
//! | 2 | 普通项目元数据（`.gitignore` / `.cargo/config.toml` …）**必放行** | `sensitive_path.rs:3-6` "Ordinary project metadata such as `.gitignore` and `.cargo/config.toml` remains readable"；单测 `:326-341` |
//! | 3 | 词法子路径里的冗余分隔符 / `.` / `..` 不得让敏感词面逃逸 | `sensitive_path.rs:25-27` "The helper also works with a lexical child path" |
//! | 4 | 拒绝帧 = 稳定 code + 来源 + 事实原因, 且逐次稳定 | `sensitive_path.rs:135-138` "稳定 code + 来源 + 事实原因「凭据面不可读（安全契约）」" |
//! | 5 | 密钥字段的**字段值**整体替换为 `[redacted]`, 字段名逐字节保留 | `sensitive_path.rs:8-13` 字段级脱敏；`:147-157` "字段值替换为 [`REDACTED`], 字段名与非敏感字段逐字节保留" |
//! | 6 | 空值不脱敏；容器值 (`{` / `[`) 不动；无字段形态的行原样返回 | `sensitive_path.rs:153-157` "存在非空值即脱敏" / "容器值 (`{` / `[`) 不动" / "无字段形态的行 … 原样返回" |
//! | 7 | 任何不可信输入不得 panic | 通用 fail-closed 底线（本面是读工具闸门） |
//!
//! **不写成断言的（刻意放过, 见交付报告）**: 域外路径的判定结果、大小写折叠、
//! Windows 尾随点/空格归一、JSON 转义引号 —— 这些在本文件的书面契约里**没有**写明，
//! 自造期望会变成假阳性。

use std::path::{Path, PathBuf};

use libfuzzer_sys::fuzz_target;

// 零复制纳入 canonical 源码 (见文件头「被测代码怎么进来」)。
#[path = "../../crates/capabilities/tools/src/sensitive_path.rs"]
mod sensitive_path;

// canonical 文件的 crate 内外围引用面 —— shim 转发到真 crate 的公开实现, 零漂移。
mod mcp_bridge {
    pub mod config {
        /// 与 `crates/capabilities/tools/src/mcp_bridge/config.rs::is_secret_key` 同一实现。
        pub fn is_secret_key(key: &str) -> bool {
            apeireth_tools_canonical::mcp_bridge::config::is_secret_key(key)
        }
    }
}

mod exec_pipeline {
    /// 与 `crates/capabilities/tools/src/exec_pipeline/mod.rs::PipelineFailure` 同一实现。
    pub use apeireth_tools_canonical::exec_pipeline::PipelineFailure;
}

use sensitive_path::{
    REDACTED, credential_surface_refusal, is_sensitive_path, redact_secret_field_values,
};

/// 模块文档 §敏感面边界 (`sensitive_path.rs:8-13`) 点名的敏感词面 + 文件自带单测
/// (`protects_known_sensitive_files_and_directories` `:284-324`,
/// `protects_credential_store_and_keychain_export_artifacts` `:343-362`) 锁死的词面。
///
/// 每一条都必须是「相对 root 的路径词面」(目录族写目录名即可)。
const SENSITIVE_RELATIVE_PATHS: &[&str] = &[
    // `.env` 族 (模块文档: "`.env` 族")
    ".env",
    ".env.local",
    ".env.production",
    // 密钥材料 (模块文档: "密钥材料" / "key material")
    "id_rsa",
    "id_ed25519",
    "server.pem",
    "server.key",
    "cert.p12",
    "cert.pfx",
    "vault.jks",
    "vault.kdbx",
    "private-key.pem",
    // 凭据存储件 (模块文档: "凭据存储件 (`creds.json`)"; "credential stores")
    "creds.json",
    "creds.json.bak",
    ".netrc",
    "_netrc",
    ".npmrc",
    ".git-credentials",
    "token.txt",
    "credentials",
    "credentials.json",
    "secret",
    "secrets.production",
    // 钥匙串导出物 (模块文档: "钥匙串导出物")
    "keychain-export.json",
    "keychain-export-2026.txt",
    "apeireth-keyring.bin",
    "backup.keyring",
    // 私钥目录 (模块文档: "private-key directories")
    ".ssh",
    ".aws",
    ".gnupg",
    ".kube",
    ".docker",
    ".secret",
    ".secrets",
    // 目录下的深层词面 (敏感目录 + 其下文件)
    ".ssh/config",
    ".aws/credentials",
    ".kube/config",
    ".docker/config.json",
    ".config/gcloud/application_default_credentials.json",
];

/// 模块文档 (`:3-6`) 点名的普通项目元数据 + 单测 `does_not_block_normal_project_dotfiles`
/// (`:326-341`) 的普通文件名 —— 必放行 (不得误拒)。
const ALLOWED_RELATIVE_PATHS: &[&str] = &[
    ".gitignore",
    ".cargo/config.toml",
    "README.md",
    "Cargo.toml",
    "src/lib.rs",
];

/// 字段名候选词干 —— 全部满足 `is_secret_key` (契约 `:153-157`: "匹配口径与启动日志脱敏
/// 同一语义 ([`is_secret_key`])"), 后缀由 fuzz 输入决定。
const SECRET_KEY_STEMS: &[&str] = &[
    "token",
    "api_key",
    "secret",
    "password",
    "x-token",
    "db.password",
];

/// 「族前缀」词面: 模块文档 (`:9-10`) 的 "`.env` 族" + 文件自带单测里同族的其它成员
/// (`.env.local` / `id_rsa.backup` / `creds.json.bak` / `credentials.json` /
/// `secrets.production`) —— 族前缀 + 任意后缀仍是该族 ⇒ 必拒。
const SENSITIVE_FAMILY_PREFIXES: &[&str] = &[
    ".env",
    "id_rsa",
    "id_ed25519",
    "creds.json",
    "credentials",
    "secrets",
];

/// root 的形态 —— 契约 `:24-25` "`root` and `path` should be canonical paths when available.
/// The helper also works with a lexical child path": 词法形态的 root 也必须能配出
/// 「root 之下的敏感子路径」。
const ROOT_SHAPES: &[&str] = &[
    "workspace",
    "workspace/",
    "./workspace",
    "workspace/.",
    "ws/..",
    "/ws",
    "",
    "work space",
];

/// 把 fuzz 字节滤成给定 ASCII 字符集 —— 避免 harness 自己造出「值边界歧义」
/// (值里混入引号 / 分隔符时, 「值到哪结束」由实现自定, 断言就成了自造期望)。
fn sanitize(raw: &[u8], allowed: &[u8]) -> String {
    raw.iter()
        .filter(|byte| allowed.contains(byte))
        .map(|byte| *byte as char)
        .collect()
}

/// 字段名允许的字符集 (裸标识符形态: 文档 `:153-157` 的 `key` / `dotted.name` 形态)。
const KEY_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_.-";

/// 字段值允许的字符集 (无引号 / 无结构分隔符 ⇒ 值边界唯一)。
const VALUE_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_./+-";

/// 正文行允许的字符集 (无 `:` / `=` ⇒ 文档所说「无字段形态的行」)。
const PROSE_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 _./-";

/// root 下的相对路径 → 绝对拼接 (root 是前缀, path 必在 root 之内)。
fn under(root: &str, relative: &str) -> PathBuf {
    PathBuf::from(format!("{root}/{relative}"))
}

fuzz_target!(|data: &[u8]| {
    // 输入布局: 0x00 分隔 —— [root, 字段名后缀, 字段值, 正文行, 路径段...]
    let mut parts = data.split(|byte| *byte == 0);
    let root_raw = parts.next().unwrap_or(&[]);
    let root = match std::str::from_utf8(root_raw) {
        Ok(text) if !text.is_empty() && !text.contains('/') => text.to_string(),
        _ => "workspace".to_string(),
    };
    let key_suffix = sanitize(parts.next().unwrap_or(&[]), KEY_CHARS);
    let value_raw = sanitize(parts.next().unwrap_or(&[]), VALUE_CHARS);
    let prose = sanitize(parts.next().unwrap_or(&[]), PROSE_CHARS);
    let segments: Vec<String> = parts
        .map(|part| String::from_utf8_lossy(part).into_owned())
        .collect();

    // ── 1. 敏感词面必拒 (契约: `sensitive_path.rs:8-13` 路径级拒绝 + `:23-27` "contains a
    //    known sensitive workspace path") —— 敏感目录/文件在**任意深度**、**带任意后缀**都必拒。
    for relative in SENSITIVE_RELATIVE_PATHS {
        for candidate in [
            (*relative).to_string(),
            format!("a/b/c/{relative}"),
            format!("{relative}/x/y"),
        ] {
            let path = under(&root, &candidate);
            assert!(
                is_sensitive_path(Path::new(&root), &path),
                "敏感面词面未被拒 (契约: sensitive_path.rs:8-13 路径级拒绝 / :23-27 contains): \
                 root={root:?} path={path:?}"
            );
        }
    }

    // ── 1b. 族前缀 + fuzz 后缀仍是该族 ⇒ 必拒 (契约: 模块文档 `:9-10` "`.env` 族";
    //     单测同族成员 `.env.local` / `id_rsa.backup` / `creds.json.bak` /
    //     `credentials.json` / `secrets.production`)。
    let family_suffix = String::from_utf8_lossy(segments.first().map_or(&[][..], |s| s.as_bytes()));
    for prefix in SENSITIVE_FAMILY_PREFIXES {
        let path = under(&root, &format!("{prefix}.{family_suffix}"));
        assert!(
            is_sensitive_path(Path::new(&root), &path),
            "族前缀的变体逃逸 (契约: sensitive_path.rs:9-10 `.env` 族 / 同族单测成员): \
             root={root:?} path={path:?}"
        );
    }

    // ── 2. 普通项目元数据必放行 (契约: `sensitive_path.rs:3-6` "Ordinary project metadata such
    //    as `.gitignore` and `.cargo/config.toml` remains readable")。
    for relative in ALLOWED_RELATIVE_PATHS {
        let path = under(&root, relative);
        assert!(
            !is_sensitive_path(Path::new(&root), &path),
            "普通项目元数据被误拒 (契约: sensitive_path.rs:3-6 remains readable): path={path:?}"
        );
    }

    // ── 3. 词法子路径 (契约: `sensitive_path.rs:25-27` "The helper also works with a lexical
    //    child path") —— 冗余分隔符 / `.` / `..` / 尾随分隔符不得让敏感词面逃逸。
    for relative in [".env", ".ssh", "creds.json"] {
        for template in [
            "{root}/x//y/./{rel}",
            "{root}/./{rel}/",
            "{root}/a/../{rel}",
            "{root}/{rel}//",
        ] {
            let rendered = template.replace("{root}", &root).replace("{rel}", relative);
            let path = PathBuf::from(rendered);
            assert!(
                is_sensitive_path(Path::new(&root), &path),
                "词法子路径里的敏感词面逃逸 (契约: sensitive_path.rs:25-27 lexical child path): \
                 root={root:?} path={path:?}"
            );
        }
    }

    // ── 3b. 词法形态的 root 也要能配出「root 之下的敏感子路径」 (契约: `:24-27`
    //     "`root` and `path` should be canonical paths when available. The helper also
    //     works with a lexical child path")。
    for shape in ROOT_SHAPES {
        for relative in [".env", ".ssh", "creds.json"] {
            let path = under(shape, relative);
            assert!(
                is_sensitive_path(Path::new(shape), &path),
                "词法 root 形态下敏感子路径逃逸 (契约: sensitive_path.rs:24-27): \
                 root={shape:?} path={path:?}"
            );
        }
    }

    // ── 4. fuzz 驱动的任意路径: 不得 panic (不可信输入不得崩)。
    let mut relative = String::new();
    for (index, segment) in segments.iter().enumerate() {
        if index > 0 {
            relative.push_str(if index % 2 == 0 { "//" } else { "/./" });
        }
        relative.push_str(segment);
    }
    let fuzzed = under(&root, &relative);
    let _ = is_sensitive_path(Path::new(&root), &fuzzed);
    // 域外路径同样不得崩 (域外判定不是本 helper 的职责, 只验不崩)。
    let outside = under(&format!("{root}-sibling"), &relative);
    let _ = is_sensitive_path(Path::new(&root), &outside);

    // ── 4b. 非 UTF-8 组件不得遮蔽敏感词面 (契约: `:23-27` "contains a known sensitive
    //     workspace path" —— 路径含 `.env` 就必拒, 与相邻组件能否转 UTF-8 无关)。
    #[cfg(unix)]
    {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let mut path = PathBuf::from(&root);
        path.push(OsStr::from_bytes(b"\xff\xfe"));
        path.push(".env");
        assert!(
            is_sensitive_path(Path::new(&root), &path),
            "非 UTF-8 组件遮蔽了敏感词面 (契约: sensitive_path.rs:23-27 contains): path={path:?}"
        );
    }

    // ── 5. 拒绝帧 (契约: `sensitive_path.rs:135-138` "帧 = 完整、关联、可分类的失败描述
    //    (不是空洞): 稳定 code + 来源 + 事实原因「凭据面不可读（安全契约）」")。
    let frame = credential_surface_refusal();
    assert!(
        frame.contains("pipeline.pre_deny"),
        "拒绝帧缺稳定 code (契约: sensitive_path.rs:135-138): {frame}"
    );
    assert!(
        frame.contains("credential_surface_guard"),
        "拒绝帧缺来源 (契约: sensitive_path.rs:135-138): {frame}"
    );
    assert!(
        frame.contains("凭据面不可读（安全契约）"),
        "拒绝帧缺事实原因 (契约: sensitive_path.rs:135-138): {frame}"
    );
    assert_eq!(
        frame,
        credential_surface_refusal(),
        "拒绝帧必须逐次稳定 (契约: sensitive_path.rs:135-138 '稳定帧文本')"
    );

    // ── 6. 字段级脱敏 (契约: `sensitive_path.rs:8-13` 字段级脱敏 + `:147-157`)。
    //    字段名取自 is_secret_key 判定为密钥字段的词干 + fuzz 后缀; 字段值取自 fuzz 字节
    //    (字符集保证「值到哪结束」唯一, 不断言自造的值边界)。
    let field_key = format!(
        "{}{key_suffix}",
        SECRET_KEY_STEMS[data.len() % SECRET_KEY_STEMS.len()]
    );
    if apeireth_tools_canonical::mcp_bridge::config::is_secret_key(&field_key) {
        let field_value = if value_raw.is_empty() {
            "x".to_string()
        } else {
            value_raw.clone()
        };

        // 6a. 密钥字段的值整体替换为 [redacted], 字段名与分隔符逐字节保留。
        for separator in [':', '='] {
            let content = format!("{field_key}{separator} {field_value}");
            let redacted = redact_secret_field_values(&content);
            assert_eq!(
                redacted,
                format!("{field_key}{separator} {REDACTED}"),
                "密钥字段值未按契约整体脱敏 (契约: sensitive_path.rs:147-157 字段值替换为 \
                 [redacted]): content={content:?} redacted={redacted:?}"
            );
        }

        // 6b. 空值不脱敏 (契约: `:153-157` "存在非空值即脱敏" —— 空值不在其列)。
        for separator in [':', '='] {
            for content in [
                format!("{field_key}{separator}"),
                format!("{field_key}{separator}   "),
            ] {
                assert_eq!(
                    redact_secret_field_values(&content),
                    content,
                    "空值被脱敏 (契约: sensitive_path.rs:153-157 存在非空值即脱敏): \
                     content={content:?}"
                );
            }
        }

        // 6c. 容器值不动 (契约: `:157` "容器值 (`{` / `[`) 不动, 其内层字段由各自的键值对
        //     单独裁决") —— 内层字段名非密钥 ⇒ 整行逐字节不变。
        for separator in [':', '='] {
            let content = format!("{field_key}{separator}{{\"plain\":1}}");
            assert_eq!(
                redact_secret_field_values(&content),
                content,
                "容器值被整体替换 (契约: sensitive_path.rs:157 容器值不动): content={content:?}"
            );
        }
    }

    // ── 7. 无字段形态的行原样返回 (契约: `:157` "无字段形态的行 (正文 / 代码语句 / 纯列表项)
    //    原样返回") —— 该行不含 `:` / `=` ⇒ 无字段形态。
    if !prose.is_empty() {
        assert_eq!(
            redact_secret_field_values(&prose),
            prose,
            "无字段形态的行被改写 (契约: sensitive_path.rs:157 无字段形态的行原样返回): \
             prose={prose:?}"
        );
    }
});
