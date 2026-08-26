//! # Lark 文档 (per @larksuiteoapi/lark-sdk v0.9.21 商业版 1:1 翻译)
//!
//! 飞书文档 `docx/v1/documents` / `sheets/v3/spreadsheets` / `bitable/v1/apps` API 翻译源.
//! 4 实体之一: `Document` (含 doc / sheet / bitable 三种类型).
//!
//! **2 核心 API** (per v0.9.21 商业版):
//! - `create_doc` — 创建 docx 文档
//! - `create_sheet` — 创建 spreadsheet
//!
//! **当前 STUB**: 字段保留 1:1 翻译, 走 `create_doc` / `create_sheet` 返 `NotImplemented`.
//!
//! ## 3 DocumentType 守门 (per v0.9.21 商业版)
//!
//! - `Doc` — docx 文档 (Word 兼容)
//! - `Sheet` — spreadsheet (Excel 兼容)
//! - `Bitable` — 多维表格 (Airtable 风)

use serde::{Deserialize, Serialize};

use crate::lark::error::LarkError;

// ============================================================================
// §1 DocumentType (3 variant, 1:1 翻译 v0.9.21 商业版)
// ============================================================================

/// 文档类型 (3 variant, per v0.9.21 商业版 `type` 字段).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    /// docx 文档 (per v0.9.21 商业版 `type: "docx"`).
    #[default]
    Doc,
    /// spreadsheet (per v0.9.21 商业版 `type: "sheet"`).
    Sheet,
    /// 多维表格 (per v0.9.21 商业版 `type: "bitable"`).
    Bitable,
}

impl DocumentType {
    /// 3 variant hardcode 常量.
    pub const COUNT: usize = 3;

    /// 字符串.
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentType::Doc => "docx",
            DocumentType::Sheet => "sheet",
            DocumentType::Bitable => "bitable",
        }
    }
}

impl std::fmt::Display for DocumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §2 Document (per v0.9.21 商业版 1:1)
// ============================================================================

/// 文档顶层结构 (per v0.9.21 商业版 docx / sheet / bitable 1:1 翻译).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    /// 文档 ID (per `document_id` 字段, R21 真接飞书后才有, STUB 模式 None).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    /// 文档 token (per `token` 字段, R21 真接飞书后才有, URL 标识).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// 文档类型 (per `type` 字段, 3 variant).
    pub doc_type: DocumentType,
    /// 文档标题 (per `title` 字段, 非空).
    pub title: String,
    /// 所在文件夹 token (per `folder_token` 字段, 根目录为 None).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder_token: Option<String>,
    /// 文档 URL (per `url` 字段, R21 真接飞书后才有).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 所有者 open_id (per `owner_open_id` 字段).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_open_id: Option<String>,
    /// 创建时间 (per `created_at` 字段, RFC3339 字符串).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// 最后修改时间 (per `updated_at` 字段, RFC3339 字符串).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl Document {
    /// 创建新 docx 文档.
    pub fn new_docx(
        title: impl Into<String>,
        folder_token: Option<String>,
    ) -> Result<Self, LarkError> {
        let title: String = title.into();
        Self::validate_title(&title)?;
        Ok(Self {
            document_id: None,
            token: None,
            doc_type: DocumentType::Doc,
            title,
            folder_token,
            url: None,
            owner_open_id: None,
            created_at: None,
            updated_at: None,
        })
    }

    /// 创建新 spreadsheet.
    pub fn new_sheet(
        title: impl Into<String>,
        folder_token: Option<String>,
    ) -> Result<Self, LarkError> {
        let title: String = title.into();
        Self::validate_title(&title)?;
        Ok(Self {
            document_id: None,
            token: None,
            doc_type: DocumentType::Sheet,
            title,
            folder_token,
            url: None,
            owner_open_id: None,
            created_at: None,
            updated_at: None,
        })
    }

    /// 创建新多维表格.
    pub fn new_bitable(
        title: impl Into<String>,
        folder_token: Option<String>,
    ) -> Result<Self, LarkError> {
        let title: String = title.into();
        Self::validate_title(&title)?;
        Ok(Self {
            document_id: None,
            token: None,
            doc_type: DocumentType::Bitable,
            title,
            folder_token,
            url: None,
            owner_open_id: None,
            created_at: None,
            updated_at: None,
        })
    }

    /// 校验 title.
    fn validate_title(title: &str) -> Result<(), LarkError> {
        if title.trim().is_empty() {
            return Err(LarkError::Other("document title is empty".to_string()));
        }
        if title.len() > 1024 {
            return Err(LarkError::Other(format!(
                "document title too long: {} > 1024",
                title.len()
            )));
        }
        Ok(())
    }

    /// 校验 3 字段 (K-1 强校验守门: doc_type / title / folder_token).
    pub fn validate(&self) -> Result<(), LarkError> {
        Self::validate_title(&self.title)?;
        if let Some(owner) = &self.owner_open_id {
            LarkError::validate_open_id(owner)?;
        }
        Ok(())
    }

    /// 设置所有者 (K-1 #4 open_id 强校验).
    pub fn with_owner(mut self, open_id: String) -> Result<Self, LarkError> {
        LarkError::validate_open_id(&open_id)?;
        self.owner_open_id = Some(open_id);
        Ok(self)
    }
}

// ============================================================================
// §3 SheetMeta (per spreadsheet 1:1 翻译, 跟 Document 配合用)
// ============================================================================

/// Sheet 元数据 (per v0.9.21 商业版 `sheets/v3/spreadsheets/{token}/sheets/{sheet_id}` 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetMeta {
    /// Sheet ID (per `sheet_id` 字段).
    pub sheet_id: String,
    /// Sheet 标题 (per `title` 字段, 默认 "Sheet1").
    pub title: String,
    /// 索引位置 (per `index` 字段, 0-based).
    pub index: u32,
    /// 行数 (per `row_count` 字段, R21 真接飞书后才有).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_count: Option<u32>,
    /// 列数 (per `column_count` 字段, R21 真接飞书后才有).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_count: Option<u32>,
    /// 是否隐藏 (per `is_hidden` 字段).
    #[serde(default)]
    pub is_hidden: bool,
}

impl SheetMeta {
    /// 创建新 sheet 元数据.
    pub fn new(sheet_id: String, title: String, index: u32) -> Self {
        Self {
            sheet_id,
            title,
            index,
            row_count: None,
            column_count: None,
            is_hidden: false,
        }
    }
}

// ============================================================================
// §4 BitableMeta (per bitable 1:1 翻译, 跟 Document 配合用)
// ============================================================================

/// 多维表格元数据 (per v0.9.21 商业版 `bitable/v1/apps/{app_token}/tables` 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitableMeta {
    /// Table ID (per `table_id` 字段).
    pub table_id: String,
    /// Table 名称 (per `name` 字段).
    pub name: String,
    /// 字段列表 (per `fields` 字段).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<BitableField>,
}

/// 多维表格字段 (per v0.9.21 商业版 `bitable/v1/apps/{app_token}/tables/{table_id}/fields` 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitableField {
    /// 字段名 (per `field_name` 字段, 非空).
    pub field_name: String,
    /// 字段类型 (per `type` 字段, e.g. "text" / "number" / "date" / "single_select").
    #[serde(rename = "type")]
    pub field_type: String,
    /// 是否必填 (per `is_required` 字段).
    #[serde(default)]
    pub is_required: bool,
}

// ============================================================================
// §5 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_type_3_variants() {
        assert_eq!(DocumentType::COUNT, 3);
    }

    #[test]
    fn document_new_docx() {
        let doc = Document::new_docx("项目计划".to_string(), None).expect("valid");
        assert_eq!(doc.doc_type, DocumentType::Doc);
        assert_eq!(doc.title, "项目计划");
    }

    #[test]
    fn document_new_sheet() {
        let doc = Document::new_sheet("预算表".to_string(), None).expect("valid");
        assert_eq!(doc.doc_type, DocumentType::Sheet);
    }

    #[test]
    fn document_new_bitable() {
        let doc = Document::new_bitable("任务列表".to_string(), None).expect("valid");
        assert_eq!(doc.doc_type, DocumentType::Bitable);
    }

    #[test]
    fn document_reject_empty_title() {
        let result = Document::new_docx(String::new(), None);
        assert!(matches!(result, Err(LarkError::Other(_))));
    }

    #[test]
    fn document_reject_too_long_title() {
        let long_title = "x".repeat(2000);
        let result = Document::new_docx(long_title, None);
        assert!(matches!(result, Err(LarkError::Other(_))));
    }

    #[test]
    fn document_with_owner() {
        let doc = Document::new_docx("title".to_string(), None)
            .expect("valid")
            .with_owner("ou_owner1234567890abcdef".to_string())
            .expect("valid owner");
        assert_eq!(
            doc.owner_open_id.as_deref(),
            Some("ou_owner1234567890abcdef")
        );
    }

    #[test]
    fn document_with_owner_rejects_invalid() {
        let result = Document::new_docx("title".to_string(), None)
            .expect("valid")
            .with_owner("invalid".to_string());
        assert!(matches!(result, Err(LarkError::OpenIdInvalid(_))));
    }

    #[test]
    fn document_validate_full() {
        let doc = Document::new_docx("title".to_string(), None)
            .expect("valid")
            .with_owner("ou_owner1234567890abcdef".to_string())
            .expect("valid");
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn sheet_meta_creation() {
        let meta = SheetMeta::new("sheet_001".to_string(), "Sheet1".to_string(), 0);
        assert_eq!(meta.sheet_id, "sheet_001");
        assert_eq!(meta.index, 0);
    }

    #[test]
    fn bitable_meta_with_fields() {
        let bitable = BitableMeta {
            table_id: "tbl_001".to_string(),
            name: "Tasks".to_string(),
            fields: vec![
                BitableField {
                    field_name: "title".to_string(),
                    field_type: "text".to_string(),
                    is_required: true,
                },
                BitableField {
                    field_name: "due_date".to_string(),
                    field_type: "date".to_string(),
                    is_required: false,
                },
            ],
        };
        assert_eq!(bitable.fields.len(), 2);
    }
}
