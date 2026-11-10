//! 记忆数据的**脱机归档报告**（真实库 ⇒ 导出 + 自检 + 往返）。
//!
//! # 这个例子回答什么
//!
//! 设计稿 §8 **B4**（脱机归档，保原文）的验收 = 「**脱机可读**」。
//! ⇒ 这个例子在一份**真实记忆库**上跑一遍 [`offline_archive`]，回答三件事：
//!
//! 1. **导得出吗**：库里的记录（含双时间列、出处、链摘要）能不能**逐字**导出；
//! 2. **自洽吗**：导出物能不能**独立校验**（条数 + 内容哈希 + 链账核对）；
//! 3. **读得回吗**：导出的 JSONL 能不能被程序**原样读回来**（往返相等）。
//!
//! # 口径（钉死）
//!
//! * **全量导出**（不限主体、不截断、含未纳管记录）⇒ 报告的是**整库**的账；
//! * **导出时刻注入**（本例子取当前 epoch 毫秒）⇒ 同一库两次导出的**内容**相同，
//!   只有 `created_at_ms` 不同（**如实**：那是导出时刻，不是数据）；
//! * **只读打开**：优先 `SQLITE_OPEN_READ_ONLY` ⇒ 导出动作**改不了**生产库；
//! * **不写库、不删库**（本例子只 SELECT + 在**输出路径**写 JSONL）。
//!
//! # 库从哪来（**如实**）
//!
//! 优先级：`命令行参数` ⇒ 环境变量 `APEIRETH_COGNITIVE_DB`（CLI 同源的那个）
//! ⇒ `APEIRETH_MEMORY_DB` ⇒ `<数据根>/cognitive.sqlite3`（默认域原位）
//! ⇒ `<数据根>/users/<user>/data/cognitive.sqlite3`（多户域）。
//!
//! **数据根取 `apeireth_core::paths::default_data_root()`**（仓内唯一真源，
//! 本例子不自己再拼一遍 `$HOME/.apeireth` ⇒ 两份实现漂移的坑）。
//!
//! **库不在 ⇒ 明确报「没跑」并返回失败**（不许把「没跑」当「没问题」）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）
//!
//! 四级口径：
//!
//! - **IMPLEMENTED** ✔：能在真实库上跑出报告（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：生产主路径 0 调用点（只有人工在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✘：**未在生产记忆库上实测过**（本机没有生产库）
//!   ⇒ 不宣称已验证。

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use apeireth_memory::offline_archive::{
    export_archive, from_jsonl, to_jsonl, verify_archive, ExportConfig,
};
use rusqlite::{Connection, OpenFlags};

/// 报告里展示的记录条数（**样本** ⇒ 够看格式，又不刷屏）。
const SAMPLE_EPISODES: usize = 5;

/// 写 JSONL 的输出路径（环境变量；没给就不落盘，只打印前几行）。
const OUT_ENV: &str = "APEIRETH_ARCHIVE_OUT";

fn main() -> ExitCode {
    let path = match locate_database() {
        Ok(path) => path,
        Err(reason) => {
            eprintln!("· 记忆库**不在**⇒ **没跑**（不是「跑过了」）：{reason}");
            eprintln!("  用法：offline_archive_report [库路径]（或设 {OUT_ENV}）");
            return ExitCode::FAILURE;
        }
    };

    println!("════════ 记忆数据脱机归档报告 ════════");
    println!("库：{}", path.display());

    let (conn, read_only) = match open_for_read(&path) {
        Ok(pair) => pair,
        Err(error) => {
            eprintln!("· **打不开**库（{}）：{error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    println!(
        "打开方式：{}",
        if read_only {
            "只读（SQLITE_OPEN_READ_ONLY ⇒ 导出改不了库）"
        } else {
            "⚠️ 读写（只读打开失败后的退化；本例子仍然只做 SELECT）"
        }
    );

    let created_at_ms = now_ms();
    let archive = match export_archive(&conn, &ExportConfig::new(), created_at_ms) {
        Ok(archive) => archive,
        Err(error) => {
            eprintln!("· **导出失败**：{error}");
            return ExitCode::FAILURE;
        }
    };

    print_manifest(&archive);
    print_chain_heads(&archive);
    print_samples(&archive);
    print_verify(&archive);
    print_roundtrip(&archive);

    ExitCode::SUCCESS
}

/// 打印 manifest（自说明清单）。
fn print_manifest(archive: &apeireth_memory::offline_archive::OfflineArchive) {
    let manifest = &archive.manifest;
    println!();
    println!("── manifest（自说明清单）──");
    println!("  格式版本   ：{}", manifest.format_version);
    println!("  导出时刻   ：{} ms（epoch 毫秒）", manifest.created_at_ms);
    println!("  记录条数   ：{}", manifest.episode_count);
    println!("  链头条数   ：{}", manifest.chain_head_count);
    println!(
        "  未纳管条数 ：{}（这些记录**不在**哈希链保护范围内）",
        manifest.unmanaged_count
    );
    println!("  内容哈希   ：{}", manifest.content_digest);
    println!(
        "  字段说明   ：{} 条（随物走 ⇒ 三年后仍可读）",
        manifest.field_docs.len()
    );
    // 打印几条字段说明（证明「自说明」不是空话）。
    for key in ["content", "chain_hash", "valid_from_ms"] {
        if let Some(text) = manifest.field_docs.get(key) {
            println!("    · {key}：{text}");
        }
    }
}

/// 打印链头账本（每条链一行）。
fn print_chain_heads(archive: &apeireth_memory::offline_archive::OfflineArchive) {
    println!();
    println!("── 链头账本（{} 条）──", archive.chain_heads.len());
    for head in archive.chain_heads.iter().take(SAMPLE_EPISODES) {
        println!(
            "  · 主体 {}：链上 {} 条 · 链头 {}",
            head.subject_id, head.entry_count, head.last_hash
        );
    }
    if archive.chain_heads.len() > SAMPLE_EPISODES {
        println!(
            "  ……（其余 {} 条省略）",
            archive.chain_heads.len() - SAMPLE_EPISODES
        );
    }
}

/// 打印记录样本（**原文逐字** ⇒ 用 `{:?}` 看清转义）。
fn print_samples(archive: &apeireth_memory::offline_archive::OfflineArchive) {
    println!();
    println!(
        "── 记录样本（{} / {} 条；正文是**原文**，转义可见）──",
        SAMPLE_EPISODES.min(archive.episodes.len()),
        archive.episodes.len()
    );
    for episode in archive.episodes.iter().take(SAMPLE_EPISODES) {
        println!(
            "  · {} · 主体 {} · 角色 {} · {} 字（{} 字节）",
            episode.id,
            episode.continuity_id,
            episode.role,
            episode.content.chars().count(),
            episode.content.len()
        );
        println!("    正文：{:?}", episode.content);
        println!(
            "    双时间：valid_from_ms={:?} · valid_until_ms={:?} · created_ms={:?}",
            episode.valid_from_ms, episode.valid_until_ms, episode.created_ms
        );
        println!(
            "    出处={:?} · 链摘要={:?}",
            episode.provenance, episode.chain_hash
        );
    }
}

/// 独立校验（**不依赖库** ⇒ 归档物自己就能证明自洽）。
fn print_verify(archive: &apeireth_memory::offline_archive::OfflineArchive) {
    println!();
    match verify_archive(archive) {
        Ok(()) => println!("── 独立校验：**通过**（条数 + 内容哈希 + 链账核对都相符）──"),
        Err(error) => println!("── 独立校验：**不通过** —— {error} ──"),
    }
}

/// JSONL 往返（⇒ 导出物**可被程序消费**）＋ 落盘（可选）。
fn print_roundtrip(archive: &apeireth_memory::offline_archive::OfflineArchive) -> ExitCode {
    let text = to_jsonl(archive);
    let lines = text.lines().count();
    println!();
    println!("── JSONL ──");
    println!("  行数：{lines}（1 行 manifest + 记录 + 链头）");

    match from_jsonl(&text) {
        Ok(restored) if restored == *archive => {
            println!("  往返：**相等**（读回来的归档物与导出的逐字一致）");
        }
        Ok(_) => {
            println!("  往返：**不相等**（读回来与导出的不一致 ⇒ 有 bug）");
            return ExitCode::FAILURE;
        }
        Err(error) => {
            println!("  往返：**读不回来** —— {error}");
            return ExitCode::FAILURE;
        }
    }

    // 前两行样例（manifest + 第一条记录）⇒ 人工能直接看出「自说明 + 原文」。
    for line in text.lines().take(2) {
        println!("  │ {line}");
    }

    match std::env::var_os(OUT_ENV) {
        Some(raw) => {
            let path = PathBuf::from(raw);
            match std::fs::write(&path, text.as_bytes()) {
                Ok(()) => println!("  已落盘：{}", path.display()),
                Err(error) => {
                    println!("  **落盘失败**（{}）：{error}", path.display());
                    return ExitCode::FAILURE;
                }
            }
        }
        None => println!("  （没设 {OUT_ENV} ⇒ 不落盘）"),
    }
    ExitCode::SUCCESS
}

/// 找记忆库（命令行参数 ⇒ 环境变量 ⇒ 生产默认域路径）。
///
/// **为什么用 `apeireth_core::paths::default_data_root()`**：数据根路径在仓里有
/// **唯一真源**（core/paths.rs）⇒ 本例子**不许**自己再拼一遍 `$HOME/.apeireth`
/// （两份实现漂移 = 体检命令读不到刚写的记忆那种坑）。
fn locate_database() -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(raw) = std::env::args().nth(1) {
        candidates.push(PathBuf::from(raw));
    }
    // `APEIRETH_COGNITIVE_DB` = CLI（`memory verify` / `memory migrate-subject`）
    // 用的那个环境变量 ⇒ 与生产同源。
    if let Some(raw) = std::env::var_os("APEIRETH_COGNITIVE_DB") {
        candidates.push(PathBuf::from(raw));
    }
    if let Some(raw) = std::env::var_os("APEIRETH_MEMORY_DB") {
        candidates.push(PathBuf::from(raw));
    }
    let data_root = apeireth_core::paths::default_data_root();
    // 默认域 = 数据根**原位**（单户兼容零迁移）。
    candidates.push(data_root.join("cognitive.sqlite3"));
    // 多户域 = `<根>/users/<id>/data/cognitive.sqlite3`。
    let user = std::env::var_os("APEIRETH_USER_ID")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("default"));
    candidates.push(
        data_root
            .join("users")
            .join(user)
            .join("data")
            .join("cognitive.sqlite3"),
    );
    candidates.push(
        data_root
            .join("users")
            .join("default")
            .join("data")
            .join("cognitive.sqlite3"),
    );

    for candidate in &candidates {
        if candidate.is_file() {
            return Ok(candidate.clone());
        }
    }
    Err(format!(
        "候选路径都不存在：{}",
        candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(" · ")
    ))
}

/// 打开库（**优先只读**；只读失败 ⇒ 退化读写并如实标注）。
fn open_for_read(path: &Path) -> Result<(Connection, bool), Box<dyn Error>> {
    match Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(conn) => Ok((conn, true)),
        Err(read_only_error) => {
            eprintln!("  （只读打开失败 ⇒ 退化为读写打开：{read_only_error}）");
            let conn = Connection::open(path)?;
            Ok((conn, false))
        }
    }
}

/// 当前时刻（epoch 毫秒）—— 导出时刻是**注入**的（⇒ 导出物可复现）。
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
}
