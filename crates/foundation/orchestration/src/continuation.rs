//! `apeireth-orchestration::continuation` — 续行快照与段编辑原语 (R12-CoordinationContext-3 实施).
//!
//! **设计哲学 (长程任务断点续传与上下文精确编辑)**:
//! - **① 续行快照 (`ContinuationSnapshot`)**:
//!   在多轮工具调用与长时间异步等待时持久化当前上下文与未决调用 (`PendingToolCall`)，
//!   支持进程重启、崩溃与多轮断点续跑；
//! - **② 一次性消费 (`consume`)**:
//!   快照采用读取并销毁语义，防止陈旧状态回放与重复执行；
//! - **③ 段编辑原语 (`SegmentEditor` + `EditAction`)**:
//!   提供 `Retain` / `Remove` / `Replace` 声明式动作，供上下文衰减调度器精细化编辑长上下文；
//! - **④ 0 假装 (O-5)**:
//!   纯确定性快照存储与上下文编辑流水线，显式时间戳注入。
//!
//! **O-6 三阶审查**:
//! 1. 总体: 解决长程 Agent 执行中的单点中断、断点续传与长上下文精细编辑问题
//! 2. 系统: 放置在 `foundation/orchestration`, 与 `context_rot` 形成协同闭环
//! 3. 架构: 强类型数据结构与存储 Trait 契约，原子文件与内存存储实现

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::context_rot::Segment;

/// 挂起的工具调用 (异步等待结果或断点续传时使用).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingToolCall {
    /// 工具名称
    pub tool_name: String,
    /// 调用参数
    pub args: Value,
    /// 唯一调用标识
    pub call_id: String,
}

/// 续行快照: 可恢复的 Agent 运行时上下文.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuationSnapshot {
    /// 快照唯一 ID
    pub id: String,
    /// 关联会话 ID
    pub session_id: String,
    /// 上下文消息列表
    pub messages: Vec<Value>,
    /// 挂起的工具调用 (None 表示常规轮次断点)
    pub pending_tool_call: Option<PendingToolCall>,
    /// 保存时间戳 (毫秒)
    pub saved_at_epoch_ms: i64,
    /// 轮次编号
    pub turn: u64,
}

impl ContinuationSnapshot {
    /// 构造新的续行快照.
    pub fn new(
        session_id: impl Into<String>,
        messages: Vec<Value>,
        pending_tool_call: Option<PendingToolCall>,
        saved_at_epoch_ms: i64,
        turn: u64,
    ) -> Self {
        Self {
            id: format!("snap-{}", Uuid::new_v4()),
            session_id: session_id.into(),
            messages,
            pending_tool_call,
            saved_at_epoch_ms,
            turn,
        }
    }
}

/// 续行快照存储与检索 Trait.
pub trait ContinuationStore: Send + Sync {
    /// 保存快照.
    fn save(&self, snapshot: &ContinuationSnapshot) -> Result<(), String>;

    /// 读取快照.
    fn load(&self, id: &str) -> Result<ContinuationSnapshot, String>;

    /// 消费快照 (读取并删除，保证一次性有效).
    fn consume(&self, id: &str) -> Result<ContinuationSnapshot, String> {
        let snap = self.load(id)?;
        self.delete(id)?;
        Ok(snap)
    }

    /// 删除快照.
    fn delete(&self, id: &str) -> Result<(), String>;

    /// 检查快照是否存在.
    fn exists(&self, id: &str) -> bool;

    /// 列出所有快照 ID.
    fn list(&self) -> Vec<String>;
}

/// 内存版续行快照存储 (供测试与无盘嵌入使用).
#[derive(Debug, Default)]
pub struct InMemoryContinuationStore {
    snapshots: Mutex<BTreeMap<String, ContinuationSnapshot>>,
}

impl InMemoryContinuationStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ContinuationStore for InMemoryContinuationStore {
    fn save(&self, snapshot: &ContinuationSnapshot) -> Result<(), String> {
        let mut guard = self.snapshots.lock().map_err(|e| e.to_string())?;
        guard.insert(snapshot.id.clone(), snapshot.clone());
        Ok(())
    }

    fn load(&self, id: &str) -> Result<ContinuationSnapshot, String> {
        let guard = self.snapshots.lock().map_err(|e| e.to_string())?;
        guard
            .get(id)
            .cloned()
            .ok_or_else(|| format!("快照不存在: {id}"))
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        let mut guard = self.snapshots.lock().map_err(|e| e.to_string())?;
        guard.remove(id);
        Ok(())
    }

    fn exists(&self, id: &str) -> bool {
        let guard = self.snapshots.lock().expect("mutex lock");
        guard.contains_key(id)
    }

    fn list(&self) -> Vec<String> {
        let guard = self.snapshots.lock().expect("mutex lock");
        guard.keys().cloned().collect()
    }
}

/// 文件系统原子续行快照存储 (原子 tmp+rename 写入).
pub struct FileContinuationStore {
    dir: PathBuf,
}

/// 快照文件名安全段: 只保留 ASCII 字母数字与 `-`/`_`, 其余字符一律剔除,
/// 并限制最大长度. 空 id 回退为 `"snapshot"`.
///
/// **P1 硬化**: 此函数同时用于最终文件名与 tmp 文件名 — 此前 tmp 文件名
/// 直接拼接原始 `snapshot.id`, 恶意 id (如 `../../evil`) 可使 tmp 写入
/// 逃逸出 store root. 本地实现而非复用 tools 层 `safe_segment`, 避免
/// foundation → capabilities 依赖倒置.
fn sanitize_snapshot_id(id: &str) -> String {
    let cleaned: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(120)
        .collect();
    if cleaned.is_empty() {
        "snapshot".to_string()
    } else {
        cleaned
    }
}

impl FileContinuationStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{}.json", sanitize_snapshot_id(id)))
    }
}

impl ContinuationStore for FileContinuationStore {
    fn save(&self, snapshot: &ContinuationSnapshot) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("创建快照目录失败: {e}"))?;
        // P1 硬化: tmp 文件名使用净化后的 id, 与最终文件名同一安全段规则,
        // 保证 tmp 写入不会逃逸出 store root (无 `..`/分隔符/绝对路径).
        let tmp = self.dir.join(format!(
            "{}.tmp-{}",
            sanitize_snapshot_id(&snapshot.id),
            Uuid::new_v4()
        ));
        let bytes =
            serde_json::to_vec_pretty(snapshot).map_err(|e| format!("序列化快照失败: {e}"))?;
        std::fs::write(&tmp, bytes).map_err(|e| format!("写入临时快照失败: {e}"))?;
        std::fs::rename(&tmp, self.path_for(&snapshot.id))
            .map_err(|e| format!("原子提交快照失败: {e}"))?;
        Ok(())
    }

    fn load(&self, id: &str) -> Result<ContinuationSnapshot, String> {
        let path = self.path_for(id);
        let bytes =
            std::fs::read(&path).map_err(|e| format!("读取快照文件失败 ({path:?}): {e}"))?;
        serde_json::from_slice(&bytes).map_err(|e| format!("解析快照失败: {e}"))
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        let path = self.path_for(id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| format!("删除快照失败: {e}"))?;
        }
        Ok(())
    }

    fn exists(&self, id: &str) -> bool {
        self.path_for(id).exists()
    }

    fn list(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
            .filter_map(|e| {
                e.path()
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
            })
            .collect()
    }
}

/// 上下文段编辑动作.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum EditAction {
    /// 保留原段
    Retain { block_id: String },
    /// 移除指定段
    Remove { block_id: String },
    /// 替换指定段内容
    Replace {
        block_id: String,
        new_content: String,
    },
}

/// 段编辑拒绝错误 (O-1 核心段保护, P1 硬化).
///
/// 核心保护段 (`Segment::core = true`) 不得通过正常编辑 API 被
/// 移除/替换/清空/间接改写; 违反时返回显式类型化错误, 不静默忽略,
/// 不提供 override 逃生口.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentEditError {
    /// 试图移除核心保护段.
    CoreSegmentRemoveDenied { block_id: String },
    /// 试图替换核心保护段 (含替换为空串).
    CoreSegmentReplaceDenied { block_id: String },
}

impl fmt::Display for SegmentEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoreSegmentRemoveDenied { block_id } => {
                write!(f, "O-1 拒绝: 核心保护段 '{block_id}' 不可被 Remove")
            }
            Self::CoreSegmentReplaceDenied { block_id } => {
                write!(f, "O-1 拒绝: 核心保护段 '{block_id}' 不可被 Replace (含替换为空)")
            }
        }
    }
}

impl std::error::Error for SegmentEditError {}

/// 上下文段编辑执行器.
#[derive(Debug, Default)]
pub struct SegmentEditor;

impl SegmentEditor {
    /// 将一系列编辑动作应用到一组段落上.
    ///
    /// **O-1 核心段保护 (P1 硬化)**:
    /// - `Remove` / `Replace` 作用于 `core = true` 段 → 返回 [`SegmentEditError`];
    /// - 预检先于任何编辑产出: 只要存在任一核心段违规, 整体拒绝并返回 `Err`,
    ///   不做部分应用;
    /// - 入参为借用, 失败时原始段落集合必然原样保留.
    ///
    /// 未被任何动作指明的段默认保留; 输出保持输入的相对顺序不变.
    pub fn apply(
        segments: &[Segment],
        actions: &[EditAction],
    ) -> Result<Vec<Segment>, SegmentEditError> {
        // 预检: 收集核心段名, 任何针对核心段的 Remove/Replace 都整体拒绝.
        let core_ids: HashSet<&str> = segments
            .iter()
            .filter(|s| s.core)
            .map(|s| s.name.as_str())
            .collect();
        for act in actions {
            match act {
                EditAction::Remove { block_id } if core_ids.contains(block_id.as_str()) => {
                    return Err(SegmentEditError::CoreSegmentRemoveDenied {
                        block_id: block_id.clone(),
                    });
                }
                EditAction::Replace { block_id, .. } if core_ids.contains(block_id.as_str()) => {
                    return Err(SegmentEditError::CoreSegmentReplaceDenied {
                        block_id: block_id.clone(),
                    });
                }
                _ => {}
            }
        }

        let mut action_map: HashMap<&str, &EditAction> = HashMap::new();
        for act in actions {
            match act {
                EditAction::Retain { block_id } => {
                    action_map.insert(block_id.as_str(), act);
                }
                EditAction::Remove { block_id } => {
                    action_map.insert(block_id.as_str(), act);
                }
                EditAction::Replace { block_id, .. } => {
                    action_map.insert(block_id.as_str(), act);
                }
            }
        }

        let mut output = Vec::new();
        for seg in segments {
            if let Some(act) = action_map.get(seg.name.as_str()) {
                match act {
                    EditAction::Retain { .. } => {
                        output.push(seg.clone());
                    }
                    EditAction::Remove { .. } => {
                        // 非核心段: 正常移除 (核心段已在预检中拒绝)
                    }
                    EditAction::Replace { new_content, .. } => {
                        let mut edited = seg.clone();
                        edited.content = new_content.clone();
                        output.push(edited);
                    }
                }
            } else {
                // 默认保留未指明的段
                output.push(seg.clone());
            }
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn in_memory_store_save_load_consume() {
        let store = InMemoryContinuationStore::new();
        let snap = ContinuationSnapshot::new(
            "session-1",
            vec![json!({"role": "user", "content": "你好"})],
            Some(PendingToolCall {
                tool_name: "WebSearch".into(),
                args: json!({"query": "Apeireth"}),
                call_id: "call-1".into(),
            }),
            1000,
            1,
        );

        store.save(&snap).unwrap();
        assert!(store.exists(&snap.id));

        let loaded = store.load(&snap.id).unwrap();
        assert_eq!(loaded, snap);

        let consumed = store.consume(&snap.id).unwrap();
        assert_eq!(consumed, snap);
        assert!(!store.exists(&snap.id));
    }

    #[test]
    fn file_store_atomic_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileContinuationStore::new(tmp.path());

        let snap = ContinuationSnapshot::new(
            "session-2",
            vec![json!({"role": "assistant", "content": "确认"})],
            None,
            2000,
            2,
        );

        store.save(&snap).unwrap();
        assert!(store.exists(&snap.id));

        let list = store.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], snap.id);

        let consumed = store.consume(&snap.id).unwrap();
        assert_eq!(consumed, snap);
        assert!(!store.exists(&snap.id));
    }

    #[test]
    fn segment_editor_edits_non_core_and_preserves_order() {
        let seg1 = Segment::new("b1", "普通历史消息", 1);
        let seg2 = Segment::new("b2", "核心人设设定", 1).with_core(true);
        let seg3 = Segment::new("b3", "需替换的旧消息", 1);

        let segments = vec![seg1, seg2, seg3];
        let actions = vec![
            EditAction::Remove {
                block_id: "b1".into(),
            },
            EditAction::Retain {
                block_id: "b2".into(),
            },
            EditAction::Replace {
                block_id: "b3".into(),
                new_content: "已压缩的精炼消息".into(),
            },
        ];

        let edited = SegmentEditor::apply(&segments, &actions).unwrap();
        assert_eq!(edited.len(), 2);
        assert_eq!(edited[0].name, "b2"); // 核心段 Retain 保留
        assert_eq!(edited[0].content, "核心人设设定");
        assert_eq!(edited[1].name, "b3");
        assert_eq!(edited[1].content, "已压缩的精炼消息");
    }

    /// P1 硬化: 核心段 Remove → 显式类型化拒绝.
    #[test]
    fn core_segment_remove_is_denied() {
        let segments = vec![
            Segment::new("b1", "普通历史消息", 1),
            Segment::new("b2", "核心人设设定", 1).with_core(true),
        ];
        let actions = vec![EditAction::Remove {
            block_id: "b2".into(),
        }];
        let err = SegmentEditor::apply(&segments, &actions).unwrap_err();
        assert_eq!(
            err,
            SegmentEditError::CoreSegmentRemoveDenied {
                block_id: "b2".into()
            }
        );
    }

    /// P1 硬化: 核心段 Replace → 显式类型化拒绝.
    #[test]
    fn core_segment_replace_is_denied() {
        let segments = vec![Segment::new("b2", "核心人设设定", 1).with_core(true)];
        let actions = vec![EditAction::Replace {
            block_id: "b2".into(),
            new_content: "偷换后的人设".into(),
        }];
        let err = SegmentEditor::apply(&segments, &actions).unwrap_err();
        assert_eq!(
            err,
            SegmentEditError::CoreSegmentReplaceDenied {
                block_id: "b2".into()
            }
        );
    }

    /// P1 硬化: 核心段 Replace 为空串 (清空攻击) → 同样拒绝.
    #[test]
    fn core_segment_replace_empty_is_denied() {
        let segments = vec![Segment::new("b2", "核心人设设定", 1).with_core(true)];
        let actions = vec![EditAction::Replace {
            block_id: "b2".into(),
            new_content: String::new(),
        }];
        let err = SegmentEditor::apply(&segments, &actions).unwrap_err();
        assert!(matches!(
            err,
            SegmentEditError::CoreSegmentReplaceDenied { .. }
        ));
    }

    /// P1 硬化: 失败编辑不做部分应用, 原始段落集合原样保留.
    #[test]
    fn failed_edit_preserves_original_segments() {
        let segments = vec![
            Segment::new("b1", "普通历史消息", 1),
            Segment::new("b2", "核心人设设定", 1).with_core(true),
            Segment::new("b3", "需替换的旧消息", 1),
        ];
        let snapshot = segments.clone();
        // b3 合法, 但 b2 核心违规 → 整体拒绝 (即使合法动作在前/在后都一样)
        let actions = vec![
            EditAction::Replace {
                block_id: "b3".into(),
                new_content: "已压缩".into(),
            },
            EditAction::Replace {
                block_id: "b2".into(),
                new_content: "偷换".into(),
            },
        ];
        assert!(SegmentEditor::apply(&segments, &actions).is_err());
        assert_eq!(segments, snapshot, "失败编辑不得改动原始段落");
        // 同样拒绝 Remove 场景
        let remove_actions = vec![EditAction::Remove {
            block_id: "b2".into(),
        }];
        assert!(SegmentEditor::apply(&segments, &remove_actions).is_err());
        assert_eq!(segments, snapshot);
    }

    /// P1 硬化: 恶意 snapshot.id 不得使 tmp/最终文件逃逸出 store root.
    #[test]
    fn malicious_snapshot_id_cannot_escape_store_root() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileContinuationStore::new(tmp.path());

        for hostile in [
            "../../../etc/evil",
            "..\\..\\..\\windows\\evil",
            "/absolute/escape",
            "C:\\evil",
            "..",
            "",
        ] {
            let snap = ContinuationSnapshot {
                id: hostile.to_string(),
                session_id: "session-x".into(),
                messages: vec![json!({"role": "user", "content": "攻击样本"})],
                pending_tool_call: None,
                saved_at_epoch_ms: 1000,
                turn: 1,
            };
            store.save(&snap).unwrap();

            // 最终文件必须落在 store root 内
            let root_canonical = std::fs::canonicalize(tmp.path()).unwrap();
            let saved_path = std::fs::canonicalize(store.path_for(&snap.id)).unwrap();
            assert!(
                saved_path.starts_with(&root_canonical),
                "id={hostile:?} 的落盘路径 {saved_path:?} 逃逸出 root {root_canonical:?}"
            );
            assert!(saved_path.extension().map_or(false, |e| e == "json"));
            // tmp 文件已被 rename 消费, root 内不应残留任何 .tmp- 散射文件
            let leftovers: Vec<_> = std::fs::read_dir(tmp.path())
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
                .collect();
            assert!(leftovers.is_empty(), "id={hostile:?} 残留 tmp 文件: {leftovers:?}");
        }
    }

    /// P1 硬化: 默认 UUID id 的行为不变 (字符集已在安全段白名单内).
    #[test]
    fn default_uuid_ids_behave_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileContinuationStore::new(tmp.path());
        let snap = ContinuationSnapshot::new("session-2", vec![json!("ok")], None, 2000, 2);
        store.save(&snap).unwrap();
        assert!(store.exists(&snap.id));
        assert_eq!(store.load(&snap.id).unwrap(), snap);
        let list = store.list();
        assert_eq!(list, vec![snap.id.clone()]);
    }
}
