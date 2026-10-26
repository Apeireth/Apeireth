#![no_main]

//! fuzz: **repo_map 符号解析面** —— 「Robust safe parser」这条承诺的机器守卫。
//!
//! 被测面（仓内源文件）：`crates/capabilities/tools/src/repo_map.rs`
//! （`SymbolParser::parse_file` + 五个语言解析器：rust / python / js-ts / go / generic）。
//!
//! ## 为什么值得 fuzz
//!
//! - **吃完全不受控输入**：这个解析器扫的是**用户仓库里的源码文件** ⇒ 内容
//!   **完全不可信**（畸形语法、超长行、混合编码、截断的 unicode……）。
//! - **有明确承诺**：类型上的文档原话是「**Robust safe parser** for extracting symbols」
//!   ⇒ **不崩**是**写下来的承诺**（不是"顺手不断言 panic"）。
//! - 它还产出一批**带位置与文本**的结构化符号（`SymbolTag`）⇒ 位置越界 / 不可复现
//!   都会让「仓库地图」失真。
//!
//! ## 判据纪律（每条断言都从被测代码自己写明的契约推出来，**不自造期望**）
//!
//! | # | 判据 | 语义出处 |
//! |---|---|---|
//! | O1 | **不崩**：任意内容 ⇒ 解析器**必须返回**（不得 panic） | 「Robust safe parser」 |
//! | O2 | **位置在界内**：`line_number` 是 **1-based**（源码 `idx + 1`）⇒ `1 ≤ line_number ≤ 行数` | `line_number = idx + 1` |
//! | O3 | **确定性**：同一 (路径, 内容) ⇒ 两次解析结果**完全相同** | 地图要可复现 |
//! | O4 | **路径回填**：每个 tag 的 `file_path` **必须等于**传入路径 | `file_path: file_path.to_path_buf()` |
//! | O5 | **签名预览来自那一行**：`signature_preview` 必须是**该行（去空白后）的前缀/子串** | `signature_preview: trimmed.to_string()` |
//!
//! ## 诚实边界
//!
//! - 只覆盖**单文件解析**面；跨文件图与 PageRank（`RepoDependencyGraph`）**未覆盖**
//!   （它们吃的是多文件组合，另立 target）。
//! - 不判断「符号有没有被漏识别」（那是**召回率**，不是承诺）⇒ 只查**位置/文本/确定性**这类
//!   **可证伪**的性质。
//! - 文件名后缀由输入决定 ⇒ 五个解析器都能被驱动到（`rs`/`py`/`ts`/`js`/`go`/其它=generic）。

use std::path::{Path, PathBuf};

use libfuzzer_sys::fuzz_target;

use apeireth_tools_canonical::repo_map::{SymbolParser, SymbolTag};

/// 后缀池（覆盖五个解析器分支；空后缀 ⇒ generic）。
const EXT_POOL: &[&str] = &["rs", "py", "ts", "tsx", "js", "jsx", "go", "txt", ""];

/// 用任意字节构造一段「源码」。**刻意混入**：
/// - 超长行（无换行的巨行）；
/// - 截断的多字节序列；
/// - 各种语言的关键字片段（让各解析器都有料可吃）。
fn craft_source(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() + 64);
    // 一段像样的多语言源码（固定前缀 ⇒ 解析器有稳定输入）
    out.push_str("fn alpha() { let beta = 1; }\n");
    out.push_str("struct Gamma { field: u32 }\n");
    out.push_str("def delta(x):\n    return x\n");
    out.push_str("class Epsilon { method() {} }\n");
    out.push_str("func zeta() {}\n");
    out.push_str("type Theta = int\n");
    out.push_str("## Omega\n");
    // 再拼上任意字节（可能截断多字节序列）
    out.push_str(&String::from_utf8_lossy(data));
    // 追加一条**超长行**（无换行）⇒ 逼出长度相关的边界
    if data.len() % 3 == 0 {
        out.push('\n');
        out.push_str(&"x".repeat(4096));
        out.push_str(" fn long_tail() {}\n");
    }
    out
}

fn check_tags(path: &Path, content: &str, tags: &[SymbolTag]) {
    let line_count = content.lines().count().max(1);

    for tag in tags {
        // O2 位置在界内（1-based）
        assert!(
            tag.line_number >= 1 && tag.line_number <= line_count,
            "行号越界：{tag:?}（文件共 {line_count} 行）"
        );
        // O4 路径回填
        assert_eq!(
            tag.file_path, path,
            "每个 tag 的 file_path 必须回填成传入路径: {tag:?}"
        );
        // O5 签名预览来自那一行
        let line = content
            .lines()
            .nth(tag.line_number - 1)
            .expect("行号在界内 ⇒ 该行存在")
            .trim();
        assert!(
            line.starts_with(tag.signature_preview.trim()),
            "签名预览必须是该行（去空白）的前缀: preview={:?} line={:?}",
            tag.signature_preview,
            line
        );
    }
}

fuzz_target!(|data: &[u8]| {
    let ext = EXT_POOL[(data.first().copied().unwrap_or(0) as usize) % EXT_POOL.len()];
    let path = if ext.is_empty() {
        PathBuf::from("fuzz_input")
    } else {
        PathBuf::from(format!("fuzz_input.{ext}"))
    };
    let content = craft_source(data);

    // O1 不崩（文档化承诺：Robust safe parser）
    let first = SymbolParser::parse_file(&path, &content);
    // O3 确定性
    let second = SymbolParser::parse_file(&path, &content);
    assert_eq!(first, second, "确定性：同一 (路径, 内容) 两次解析必须完全相同");

    check_tags(&path, &content, &first);
});
