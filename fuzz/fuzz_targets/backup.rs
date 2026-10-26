#![no_main]

//! fuzz: **备份/恢复面（`backup.rs`）** —— 三条硬语义的机器守卫。
//!
//! 被测面（仓内源文件）：`crates/capabilities/tools/src/backup.rs`
//! （`export_backup` / `verify_backup` / `restore_backup` + `BackupManifest`）。
//!
//! ## 为什么值得 fuzz（这个面吃**不可信输入**且有**安全承诺**）
//!
//! - **吃不受控输入**：备份包是**磁盘上的目录 + manifest.json** ⇒ 它的路径、版本号、
//!   摘要、文件字节**全部可被构造**（拷贝来的包、被改过的包、来自别处的包）；
//!   恢复面是**破坏性**的（要落位真实文件）。
//! - **有强承诺**（模块头写死的「三条硬语义，钉死，测试为证」）：
//!   1. **凭据不随行** —— 密钥/令牌面**永不进包**，排除项在 manifest `excluded` 里**如实列出**；
//!   2. **宁拒不损** —— 恢复前全量校验（清单可解析 + 版本匹配 + 逐文件摘要），
//!      任一不符**拒绝恢复，一个字节都不落盘**；
//!   3. **版本兼容 = 显式拒** —— `format_version` 不符 ⇒ 显式拒绝并报出双方版本号。
//!
//! ## 判据纪律（每条断言都从上面三条语义推出来，**不自造期望**）
//!
//! | # | 判据 | 语义出处 |
//! |---|---|---|
//! | O1 | 导出的 bundle 里**不得出现**任何凭据排除表里的文件名 | 凭据不随行 |
//! | O2 | 被排除的源文件**必须**出现在 manifest `excluded` 里（不静默丢件），且**不得**出现在 `files` 里 | 凭据不随行 |
//! | O3 | 导出到**已存在**的目录 ⇒ **拒绝**（永不覆盖） | 增量语义 |
//! | O4 | 把 manifest 的 `format_version` 改掉 ⇒ verify/restore **必须拒绝** | 版本兼容 = 显式拒 |
//! | O5 | 把 bundle 里某个文件的字节改掉 ⇒ verify/restore **必须拒绝**，且**恢复目录一个字节都不变** | 宁拒不损 |
//! | O6 | 把 manifest 里某条 `path` 改成 `../` 穿越 ⇒ restore **必须拒绝**，且**不得在恢复根外落任何文件** | 宁拒不损（落位范围 = 成员集） |
//! | O7 | 无篡改时恢复 ⇒ 落位文件与源**逐字节相同** | 恢复的定义 |
//!
//! ## 诚实边界
//!
//! - 只跑**目录快照**这一条导出路径（该仓**不引归档容器依赖**，只有目录快照这一种形态）；
//! - **符号链接**面未覆盖（导出会跳过符号链接，但本 target 不造符号链接）；
//! - **中途失败回滚**那条语义未覆盖（需要制造落位中途失败，构造面更大）。

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use libfuzzer_sys::fuzz_target;
use serde_json::Value;

use apeireth_tools_canonical::backup::{
    export_backup, restore_backup, verify_backup, BackupCategory, BackupHooks, BackupItem,
    BackupSources, RestoreOptions, BACKUP_FORMAT_VERSION, CREDENTIAL_ARTIFACTS_EXCLUDED,
    MANIFEST_FILE_NAME,
};

/// 源文件名的候选池（**刻意混入凭据面名字**，好压 O1/O2）。
const NAME_POOL: &[&str] = &[
    "notes.md",
    "sessions.sqlite3",
    "users.json",
    "skills/card.md",
    "creds.json",
    "id_rsa",
    ".env",
    "credentials.vault",
    "master.key",
    "token.txt",
    "api-key.json",
];

fn write_file(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, bytes);
}

/// 目录快照（相对路径 → 字节），用于比较「恢复前后是否变化」。
fn snapshot_dir(root: &Path) -> BTreeSet<(String, Vec<u8>)> {
    let mut out = BTreeSet::new();
    fn walk(root: &Path, dir: &Path, out: &mut BTreeSet<(String, Vec<u8>)>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else if let Ok(bytes) = fs::read(&path) {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned();
                out.insert((rel, bytes));
            }
        }
    }
    walk(root, root, &mut out);
    out
}

/// 把 manifest 的某个顶层字段改掉（保留其它字段）。
fn patch_manifest(manifest_path: &Path, field: &str, value: Value) -> bool {
    let Ok(raw) = fs::read_to_string(manifest_path) else {
        return false;
    };
    let Ok(mut parsed) = serde_json::from_str::<Value>(&raw) else {
        return false;
    };
    parsed[field] = value;
    fs::write(manifest_path, serde_json::to_string(&parsed).unwrap_or_default()).is_ok()
}

fuzz_target!(|data: &[u8]| {
    let root = std::env::temp_dir().join(format!(
        "apeireth-backup-fuzz-{}-{}",
        std::process::id(),
        // 用输入字节做「本次运行」的种子 ⇒ 不同输入用不同目录 ⇒ 不互相踩
        data.iter().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(*b as u64))
    ));
    let data_root = root.join("data");
    let bundle = root.join("bundle");
    let restore_root = root.join("restore");
    let _ = fs::create_dir_all(&data_root);
    let _ = fs::create_dir_all(&restore_root);

    // ── 源文件：名字从池里取（轮转），内容取自输入字节
    let count = (data.first().copied().unwrap_or(0) as usize % 6) + 1;
    let mut items = Vec::new();
    for index in 0..count {
        let name = NAME_POOL[index % NAME_POOL.len()];
        let rel = format!("src/{index}/{name}");
        let origin = data_root.join(&rel);
        let start = (index * 7 + 1) % data.len().max(1);
        let end = ((start + 13) % data.len().max(1)).max(start);
        let content = data.get(start..end).unwrap_or(&[]).to_vec();
        write_file(&origin, &content);
        items.push(BackupItem::file(
            BackupCategory::Sessions,
            origin.clone(),
            PathBuf::from(format!("items/{index}/{name}")),
        ));
    }
    let sources = BackupSources::new(data_root.clone(), items);

    // ── 导出
    let manifest = match export_backup(&sources, &bundle, &BackupHooks::none()) {
        Ok(manifest) => manifest,
        // 导出被拒（例如 staging 冲突）⇒ 本次输入无意义，直接收工（**不伪造期望**）。
        Err(_) => {
            let _ = fs::remove_dir_all(&root);
            return;
        }
    };

    // ── O1 凭据不随行：bundle 里不得出现排除表里的文件名
    let excluded_names: BTreeSet<&str> = CREDENTIAL_ARTIFACTS_EXCLUDED
        .iter()
        .filter_map(|entry| Path::new(entry).file_name())
        .filter_map(|name| name.to_str())
        .collect();
    fn walk_bundle(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_bundle(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let mut bundle_files = Vec::new();
    walk_bundle(&bundle, &mut bundle_files);
    for path in &bundle_files {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        assert!(
            !excluded_names.contains(name.as_str()),
            "凭据不随行：bundle 里出现了排除表里的文件 {name}（path={:?}）",
            path
        );
    }

    // ── O2 排除项如实列出：源里存在的凭据面文件必须出现在 excluded，且不得在 files 里
    let manifest_paths: BTreeSet<&str> = manifest
        .files
        .iter()
        .map(|entry| entry.path.as_str())
        .collect();
    let excluded_paths: BTreeSet<&str> = manifest
        .excluded
        .iter()
        .map(|entry| entry.path.as_str())
        .collect();
    // ⚠️ **以 manifest 自己的账为准**（不再用「名字在不在排除表里」自己推断）。
    // 理由（我的前三版都栽在这）：排除面 = **显式排除表 + 既有敏感路径策略** 两套
    // ⇒ `id_rsa` 就不在显式表里（它被**敏感路径策略**排掉）
    // ⇒ 我按名字猜 ⇒ 猜错 ⇒ **自造期望** ⇒ 连炸三次。
    // ⇒ 合同说的是「**排除项在 manifest `excluded` 里如实列出（不静默丢件）**」
    // ⇒ 那就**以 manifest 为真源**：每个存在的源文件，**必须**在 `files` 或 `excluded` 里。
    for name in NAME_POOL {
        if !find_named(&data_root, name) {
            continue;
        }
        let in_files = manifest_paths.iter().any(|path| path.ends_with(name));
        let in_excluded = excluded_paths.iter().any(|path| path.ends_with(name));
        assert!(
            in_files || in_excluded,
            "排除项必须如实列出：{name} 既不在 files 也不在 excluded（静默丢件）"
        );
        if in_excluded {
            assert!(
                !in_files,
                "凭据不随行：{name} 同时出现在 files 与 excluded（既进了包又被列为排除）"
            );
        }
    }

    // ── O3 导出永不覆盖：目标已存在 ⇒ 拒绝
    assert!(
        export_backup(&sources, &bundle, &BackupHooks::none()).is_err(),
        "导出到已存在的目录必须拒绝（不覆盖）"
    );

    // ── 无篡改路径：verify 必须通过（导出自洽）
    assert!(
        verify_backup(&bundle).is_ok(),
        "刚导出的 bundle 必须自洽（verify 应通过）"
    );

    // ── O7 正常恢复：落位文件与源逐字节相同
    let restore_report = restore_backup(
        &bundle,
        &RestoreOptions::to_root(&restore_root),
        &BackupHooks::none(),
    );
    assert!(
        restore_report.is_ok(),
        "无篡改时恢复必须成功: {:?}",
        restore_report.err().map(|e| e.to_string())
    );
    let restored = snapshot_dir(&restore_root);
    for (index, item) in sources.items.iter().enumerate() {
        let name = NAME_POOL[index % NAME_POOL.len()];
        let Ok(original) = fs::read(&item.origin) else {
            continue;
        };
        let rel = format!("items/{index}/{name}");
        let got = restored
            .iter()
            .find(|(path, _)| path == &rel)
            .map(|(_, bytes)| bytes.clone());
        if excluded_paths.iter().any(|path| path.ends_with(name)) {
            // ⚠️ **凭据面在恢复方向上也不得复活**（这是「凭据不随行」的另一半）：
            // 导出时被排除 ⇒ 恢复时也**不得**凭空造出它。
            // ⚠️ 我的第一版这里断言「所有源都必须被恢复」⇒ **对凭据文件是错的**
            // ⇒ 第一版崩：`恢复的定义：落位字节必须与源相同（items/4/creds.json）`。
            assert!(
                got.is_none(),
                "凭据不随行（恢复方向）：{name} 不该被恢复出来（rel={rel}）"
            );
            continue;
        }
        assert_eq!(
            got,
            Some(original),
            "恢复的定义：落位字节必须与源相同（{rel}）"
        );
    }

    // ── 篡改模式（由输入字节决定）：验证「宁拒不损」与「版本兼容 = 显式拒」
    let mode = data.get(1).copied().unwrap_or(0) % 3;
    let manifest_path = bundle.join(MANIFEST_FILE_NAME);
    let before = snapshot_dir(&restore_root);
    let escape_marker = root.join("ESCAPED.txt");

    if mode == 0 {
        // O4 版本不符 ⇒ 必须拒绝
        assert!(
            patch_manifest(&manifest_path, "format_version", Value::from(BACKUP_FORMAT_VERSION + 1)),
            "改写 manifest.format_version 失败（测试前提不成立）"
        );
        assert!(
            verify_backup(&bundle).is_err(),
            "版本不符必须显式拒绝（verify）"
        );
        assert!(
            restore_backup(&bundle, &RestoreOptions::to_root(&restore_root), &BackupHooks::none())
                .is_err(),
            "版本不符必须显式拒绝（restore）"
        );
    } else if mode == 1 {
        // O5 摘要不符 ⇒ 必须拒绝，且一个字节都不落盘
        if let Some(first) = bundle_files.iter().find(|path| {
            path.file_name()
                .map(|n| n != MANIFEST_FILE_NAME)
                .unwrap_or(false)
        }) {
            let mut bytes = fs::read(first).unwrap_or_default();
            bytes.push(b'!');
            let _ = fs::write(first, &bytes);
            assert!(
                verify_backup(&bundle).is_err(),
                "摘要不符必须拒绝（verify）"
            );
            assert!(
                restore_backup(
                    &bundle,
                    &RestoreOptions::to_root(&restore_root),
                    &BackupHooks::none()
                )
                .is_err(),
                "摘要不符必须拒绝（restore）"
            );
        }
    } else {
        // O6 路径穿越 ⇒ 必须拒绝，且不得在恢复根外落文件
        let raw = fs::read_to_string(&manifest_path).unwrap_or_default();
        if let Ok(mut parsed) = serde_json::from_str::<Value>(&raw) {
            if let Some(files) = parsed.get_mut("files").and_then(|v| v.as_array_mut()) {
                if let Some(first) = files.first_mut() {
                    if let Some(obj) = first.as_object_mut() {
                        obj.insert("path".to_string(), Value::from("../ESCAPED.txt"));
                        obj.insert("relpath".to_string(), Value::from("../ESCAPED.txt"));
                    }
                }
                let _ = fs::write(&manifest_path, serde_json::to_string(&parsed).unwrap_or_default());
                assert!(
                    restore_backup(
                        &bundle,
                        &RestoreOptions::to_root(&restore_root),
                        &BackupHooks::none()
                    )
                    .is_err(),
                    "路径穿越必须拒绝恢复"
                );
                assert!(
                    !escape_marker.exists(),
                    "路径穿越不得在恢复根外落任何文件"
                );
            }
        }
    }

    // ── 拒绝路径上的**不变式**：恢复根必须与篡改前逐字节相同（宁拒不损）
    let after = snapshot_dir(&restore_root);
    assert_eq!(
        before, after,
        "拒绝恢复时，恢复根必须一个字节都不变（宁拒不损）"
    );

    let _ = fs::remove_dir_all(&root);
});

/// 递归找名为 `name` 的文件。
fn find_named(dir: &Path, name: &str) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if find_named(&path, name) {
                return true;
            }
        } else if path.file_name().map(|n| n == name).unwrap_or(false) {
            return true;
        }
    }
    false
}
