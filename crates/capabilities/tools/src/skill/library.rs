//! 技能库 (数据目录 `skills/`) —— 技能卡的落盘面与版本演进审计。
//!
//! 存储形态: **每卡一 StoredDoc 文件** (`<id>.json`, 信封
//! `{ name, version, compatible_versions, body }`), 拒开语义原样沿用
//! [`apeireth_core::stored_doc`] —— 坏 JSON / 串档 / 版本不符一律**拒绝打开**
//! 并报结构化错误, 不静默迁移、不回退默认。
//!
//! 版本演进 (同名更新) 的旧版留档在 `audit.jsonl` (逐记录 jsonl, 坏行读作
//! 不存在): 每次演进先把被取代的整卡追加进审计, 再原子写新版 —— 审计是
//! 「留旧版」的唯一事实源, 当前版本序号 = 已存档条数 + 1。

use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::{
    self, scan_records, DocCompat, RecordScan, StoredDoc, StoredDocError, DEFAULT_DOC_MODE,
};
use serde::{Deserialize, Serialize};

use super::card::{name_key, structure_card, SkillCard, SkillCardDraft};

/// 技能库子目录名 (挂在数据目录下: `<data_dir>/skills/`)。
pub const SKILLS_SUBDIR: &str = "skills";

/// 技能卡 StoredDoc 的格式版本戳。
pub const SKILL_CARD_DOC_VERSION: u32 = 1;

/// 版本演进审计 StoredDoc 的身份与格式版本戳。
pub const SKILL_AUDIT_DOC_NAME: &str = "skill-card-audit";

/// 版本演进审计 StoredDoc 的格式版本戳。
pub const SKILL_AUDIT_DOC_VERSION: u32 = 1;

/// 版本演进审计文件 (逐记录 jsonl)。
pub const SKILL_AUDIT_FILE: &str = "audit.jsonl";

/// 技能卡 StoredDoc 身份: 每卡带自己的 id (串档打开即拒)。
fn card_doc_name(id: &str) -> String {
    format!("skill-card:{id}")
}

/// 技能卡读端契约 (仅当前格式版本可读, 其余拒开)。
fn card_compat(id: &str) -> DocCompat {
    DocCompat::exact(card_doc_name(id), SKILL_CARD_DOC_VERSION)
}

/// 审计读端契约。
fn audit_compat() -> DocCompat {
    DocCompat::exact(SKILL_AUDIT_DOC_NAME, SKILL_AUDIT_DOC_VERSION)
}

/// 版本演进审计条目: 被取代的**整张旧卡**留档。
///
/// 条目陈述恒真: 「这张卡的第 `revision` 版于 `archived_at` 被存档」——即使
/// 后续新版落盘失败, 存档本身仍是该版本的真实副本。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillRevisionAudit {
    /// 被存档卡的稳定身份。
    pub id: String,
    /// 被存档卡的卡名。
    pub name: String,
    /// 被存档卡当时的版本序号 (1 起)。
    pub revision: u32,
    /// 存档时刻 (epoch 毫秒)。
    pub archived_at: u64,
    /// 被存档的整张卡。
    pub archived: SkillCard,
}

/// 一次入库结果 (创建或版本演进)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillIntake {
    /// 落盘后的当前卡 (StoredDoc 信封)。
    pub card: StoredDoc<SkillCard>,
    /// 当前卡的版本序号 (创建 = 1; 每次演进 +1)。
    pub revision: u32,
    /// 版本演进时被存档的旧版本 (创建 = 无)。
    pub archived: Option<SkillRevisionAudit>,
}

/// 库扫描结果: 好卡照读, 打不开的卡如实列为拒绝 (不砖化整个技能库)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillLibraryScan {
    /// 打开成功的卡 (按卡名键 + id 排序, 确定性)。
    pub cards: Vec<StoredDoc<SkillCard>>,
    /// 被拒开的卡文件 (路径 + 结构化原因)。
    pub refused: Vec<SkillCardRefusal>,
}

/// 一张被拒开的卡 (拒开语义的如实回报)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillCardRefusal {
    /// 被拒开的文件路径。
    pub path: PathBuf,
    /// 拒开原因 (结构化错误的显示文本)。
    pub reason: String,
}

/// 技能库错误。
#[derive(Debug)]
pub enum SkillLibraryError {
    /// 磁盘 IO 失败。
    Io {
        /// 相关路径。
        path: PathBuf,
        /// 底层错误。
        source: std::io::Error,
    },
    /// StoredDoc 拒开 (坏 JSON / 串档 / 版本不符)。
    Stored(StoredDocError),
    /// 创建撞同名 (重复名拒绝)。
    DuplicateName {
        /// 撞上的卡名。
        name: String,
        /// 既有卡的稳定身份。
        existing_id: String,
    },
    /// 版本演进找不到同名卡 (fail-closed: 演进必须有旧版)。
    MissingName {
        /// 目标卡名。
        name: String,
    },
    /// 草稿结构校验不过 (坏卡拒)。
    BadCard(String),
}

impl std::fmt::Display for SkillLibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "技能库 IO 失败 {}: {source}", path.display())
            }
            Self::Stored(error) => write!(f, "技能卡拒开: {error}"),
            Self::DuplicateName { name, existing_id } => {
                write!(f, "拒绝入库 {name}: 技能库已有同名卡 ({existing_id}), 重复名拒绝创建; 版本演进请走 update")
            }
            Self::MissingName { name } => {
                write!(f, "拒绝演进 {name}: 技能库没有同名卡 (版本演进必须有旧版)")
            }
            Self::BadCard(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for SkillLibraryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Stored(error) => Some(error),
            _ => None,
        }
    }
}

impl From<StoredDocError> for SkillLibraryError {
    fn from(error: StoredDocError) -> Self {
        Self::Stored(error)
    }
}

/// 技能库: `<root>/` 下每卡一 StoredDoc 文件 + `audit.jsonl` 版本演进审计。
#[derive(Debug, Clone)]
pub struct SkillLibrary {
    root: PathBuf,
}

impl SkillLibrary {
    /// 绑定技能库根目录 (即 `<data_dir>/skills/`)。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 挂在数据目录下的技能库 (`<data_dir>/skills/`)。
    pub fn under_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self::new(data_dir.as_ref().join(SKILLS_SUBDIR))
    }

    /// 技能库根。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 卡文件路径 (`<root>/<id>.json`)。
    pub fn card_path(&self, id: &str) -> PathBuf {
        self.root.join(format!("{id}.json"))
    }

    /// 审计文件路径 (`<root>/audit.jsonl`)。
    pub fn audit_path(&self) -> PathBuf {
        self.root.join(SKILL_AUDIT_FILE)
    }

    /// 严格打开一张卡 (拒开语义): 坏 JSON / 串档 / 版本不符一律报错。
    pub fn open_card(&self, id: &str) -> Result<StoredDoc<SkillCard>, StoredDocError> {
        stored_doc::open_single(&self.card_path(id), &card_compat(id))
    }

    /// 扫描技能库: 好卡照读 (确定性排序), 打不开的卡如实列拒 (跳过并回报,
    /// 不砖化整个库、也绝不「读坏用默认」)。
    pub fn scan(&self) -> SkillLibraryScan {
        let mut scan = SkillLibraryScan::default();
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return scan;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let Some(id) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            match self.open_card(id) {
                Ok(doc) => scan.cards.push(doc),
                Err(error) => scan.refused.push(SkillCardRefusal {
                    path,
                    reason: error.to_string(),
                }),
            }
        }
        scan.cards.sort_by(|a, b| {
            name_key(&a.body.name)
                .cmp(&name_key(&b.body.name))
                .then_with(|| a.body.id.cmp(&b.body.id))
        });
        scan.refused.sort_by(|a, b| a.path.cmp(&b.path));
        scan
    }

    /// 按卡名找当前卡 (同名键 = 去空白 + 大小写折叠)。
    pub fn find_by_name(&self, name: &str) -> Option<StoredDoc<SkillCard>> {
        let key = name_key(name);
        self.scan()
            .cards
            .into_iter()
            .find(|doc| name_key(&doc.body.name) == key)
    }

    /// 某张卡的版本演进审计 (按存档顺序; 坏审计行读作不存在)。
    pub fn revision_archive(
        &self,
        name: &str,
    ) -> Result<Vec<SkillRevisionAudit>, SkillLibraryError> {
        let key = name_key(name);
        let RecordScan { records, .. } =
            scan_records::<SkillRevisionAudit>(&self.audit_path(), &audit_compat())?;
        Ok(records
            .into_iter()
            .map(|doc| doc.body)
            .filter(|entry| name_key(&entry.name) == key)
            .collect())
    }

    /// 创建入库: 坏卡拒 + **重复名拒绝** (同名卡已存在即拒, 版本演进走
    /// [`Self::update`])。
    pub fn create(
        &self,
        draft: &SkillCardDraft,
        now_ms: u64,
    ) -> Result<SkillIntake, SkillLibraryError> {
        let card = structure_card(draft, now_ms).map_err(SkillLibraryError::BadCard)?;
        if let Some(existing) = self.find_by_name(&card.name) {
            return Err(SkillLibraryError::DuplicateName {
                name: card.name.clone(),
                existing_id: existing.body.id,
            });
        }
        let doc = stored_doc::save_single(
            &self.card_path(&card.id),
            &card_compat(&card.id),
            card,
            DEFAULT_DOC_MODE,
        )?;
        Ok(SkillIntake {
            card: doc,
            revision: 1,
            archived: None,
        })
    }

    /// 版本演进 (同名更新): 旧版先留审计 (`audit.jsonl`), 新版原子落盘 ——
    /// `id` 与 `created_at` 保持不变。找不到同名卡即拒 (fail-closed)。
    pub fn update(
        &self,
        draft: &SkillCardDraft,
        now_ms: u64,
    ) -> Result<SkillIntake, SkillLibraryError> {
        let card = structure_card(draft, now_ms).map_err(SkillLibraryError::BadCard)?;
        let Some(old) = self.find_by_name(&card.name) else {
            return Err(SkillLibraryError::MissingName { name: card.name });
        };
        let archived_count = self.revision_archive(&card.name)?.len() as u32;
        let entry = SkillRevisionAudit {
            id: old.body.id.clone(),
            name: old.body.name.clone(),
            revision: archived_count + 1,
            archived_at: now_ms,
            archived: old.body.clone(),
        };
        // 审计先行: 「留旧版」是版本演进的硬语义, 存档失败即不演进。
        self.append_audit_line(&entry)?;
        let evolved = SkillCard {
            id: old.body.id,
            created_at: old.body.created_at,
            ..card
        };
        let doc = stored_doc::save_single(
            &self.card_path(&evolved.id),
            &card_compat(&evolved.id),
            evolved,
            DEFAULT_DOC_MODE,
        )?;
        Ok(SkillIntake {
            card: doc,
            revision: archived_count + 2,
            archived: Some(entry),
        })
    }

    /// 追加一行版本演进审计 (JSONL, 追加写 + 落盘)。
    fn append_audit_line(&self, entry: &SkillRevisionAudit) -> Result<(), SkillLibraryError> {
        use std::io::Write;
        std::fs::create_dir_all(&self.root).map_err(|source| SkillLibraryError::Io {
            path: self.root.clone(),
            source,
        })?;
        let doc = StoredDoc::new(
            SKILL_AUDIT_DOC_NAME,
            SKILL_AUDIT_DOC_VERSION,
            vec![SKILL_AUDIT_DOC_VERSION],
            entry,
        );
        // ⚠️ **2026-10-26 修同族缺陷**：原写法是**两次写**（行体一次、换行一次）
        // ⇒ **并发写者**可以在两次写之间插进来 ⇒ ⇒ **两行被挤成一行** ⇒ 记录丢失。
        // ⇒ 改用**共用原语** `crate::proposal::append_jsonl_row`
        // （四个逐记录追加器**全部**用它 ⇒ 一个实现、不分叉）。
        crate::proposal::append_jsonl_row(&self.audit_path(), "技能卡审计账", &doc).map_err(
            |error| SkillLibraryError::Io {
                path: self.audit_path(),
                source: std::io::Error::other(error),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::card::{SkillSource, SkillSourceKind};

    fn draft(name: &str, body: &str) -> SkillCardDraft {
        SkillCardDraft {
            name: name.into(),
            summary: "换元后微分元同步替换".into(),
            triggers: vec!["换元".into(), "dx".into()],
            body: body.into(),
            source: SkillSource {
                kind: SkillSourceKind::Book,
                title: "数学分析讲义".into(),
            },
        }
    }

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-skill-library-{tag}-{}-{}",
            std::process::id(),
            crate::worktree_plan::fnv1a64(tag.as_bytes())
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("创建测试目录");
        dir
    }

    /// 回归（2026-10-26）：**撕裂写后追加不许把旧行吞掉**（技能卡版本审计账）。
    ///
    /// 旧实现是**两次写**（行体一次、换行一次）⇒ 末行缺换行时，下一次追加会把
    /// **旧行与新行粘成一行** ⇒ 旧行读不出来（看着像「坏行」，实则是**丢记录**）。
    #[test]
    fn appending_audit_after_a_torn_line_keeps_every_row_readable() {
        let root = temp_root("torn-audit");
        let library = SkillLibrary::new(&root);

        // 第一次入库（**不写审计行** —— 审计行是「版本演进时归档旧版」才写的）。
        let _ = library
            .create(&draft("换元法", "步骤一"), 1_000)
            .expect("create");
        // 演一版 ⇒ 归档旧版 ⇒ 写入第一条审计行。
        let _ = library
            .update(&draft("换元法", "步骤二"), 2_000)
            .expect("update v2");

        // 制造撕裂写：把审计账末行的换行去掉。
        let audit_path = library.audit_path();
        let mut bytes = std::fs::read(&audit_path).expect("read audit");
        assert!(bytes.ends_with(b"\n"), "正常追加的末行应有换行");
        bytes.pop();
        std::fs::write(&audit_path, &bytes).expect("write torn audit");

        // 再演一版（追加第二条审计行）。
        let _ = library
            .update(&draft("换元法", "步骤三"), 3_000)
            .expect("update v3");

        // 审计账读回：两行都在，且没有坏行。
        let scan = scan_records::<SkillRevisionAudit>(&library.audit_path(), &audit_compat())
            .expect("审计账可读");
        assert_eq!(
            scan.skipped, 0,
            "撕裂写后追加不许产生坏行（丢记录）: scan={scan:?}"
        );
        assert_eq!(
            scan.records.len(),
            2,
            "撕裂写后追加不许把旧行吞掉（两行都应可读）: scan={scan:?}"
        );
    }

    #[test]
    fn create_lands_a_stored_doc_that_reopens_exactly() {
        let root = temp_root("create");
        let library = SkillLibrary::new(&root);
        let intake = library.create(&draft("换元法", "步骤一"), 1_000).unwrap();
        assert_eq!(intake.revision, 1);
        assert!(intake.archived.is_none());
        let reopened = library.open_card(&intake.card.body.id).unwrap();
        assert_eq!(reopened, intake.card, "落盘往返必须逐字段等价");
        assert_eq!(reopened.body.created_at, 1_000);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn duplicate_names_are_refused_and_the_existing_card_stays_untouched() {
        let root = temp_root("dup");
        let library = SkillLibrary::new(&root);
        let first = library.create(&draft("换元法", "旧正文"), 1_000).unwrap();
        let err = library
            .create(&draft("  换元法 ", "新正文"), 2_000)
            .unwrap_err();
        assert!(
            matches!(err, SkillLibraryError::DuplicateName { .. }),
            "{err}"
        );
        let still = library.open_card(&first.card.body.id).unwrap();
        assert_eq!(still.body.body, "旧正文", "重复名拒绝不得改写既有卡");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn update_evolves_the_version_and_archives_the_old_one() {
        let root = temp_root("evolve");
        let library = SkillLibrary::new(&root);
        let first = library
            .create(&draft("换元法", "第一版正文"), 1_000)
            .unwrap();
        let evolved = library
            .update(&draft("换元法", "第二版正文"), 2_000)
            .unwrap();
        assert_eq!(evolved.revision, 2);
        assert_eq!(evolved.card.body.id, first.card.body.id, "id 版本演进不变");
        assert_eq!(
            evolved.card.body.created_at, 1_000,
            "created_at 版本演进不变"
        );
        assert_eq!(evolved.card.body.body, "第二版正文");
        let archived = evolved.archived.expect("旧版必须留审计");
        assert_eq!(archived.revision, 1);
        assert_eq!(archived.archived.body, "第一版正文");
        assert_eq!(archived.archived_at, 2_000);
        // 审计落盘可读: 一条记录, 内容 = 被取代的整张旧卡。
        let archive = library.revision_archive("换元法").unwrap();
        assert_eq!(archive.len(), 1);
        assert_eq!(archive[0].archived.body, "第一版正文");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn update_without_an_existing_card_is_refused() {
        let root = temp_root("update-missing");
        let library = SkillLibrary::new(&root);
        let err = library
            .update(&draft("不存在的卡", "正文"), 1_000)
            .unwrap_err();
        assert!(
            matches!(err, SkillLibraryError::MissingName { .. }),
            "{err}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
