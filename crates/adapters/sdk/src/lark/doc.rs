//! # lark 文档面 (docx / spreadsheet 创建 wire 契约)
//!
//! 平台文档端点:
//! - `POST /docx/v1/documents` (创建 docx 文档)
//! - `POST /sheets/v3/spreadsheets` (创建 spreadsheet)
//!
//! ## wire 契约 (严格形状 + 未知字段容错)
//!
//! - 请求体: `{"title": <非空 ≤1024 字节>, "folder_token": <可选>}`;
//! - 响应 `data`:
//!   - docx: `{"document": {"document_id": <非空 必填>, "title": <可选>, ...}}`
//!   - sheet: `{"spreadsheet": {"spreadsheet_token": <非空 必填>, "url": <可选>, "title": <可选>}}`
//! - 未知字段忽略; 缺关键 ID = 永久错误。
//!
//! ## 字段校验 (发送前置 + 响应映射)
//!
//! - `title` 非空且 ≤ 1024 字节; `owner_open_id` (若有) K-1 #4;
//! - `doc_type` 必须与调用的端点匹配 (`create_doc` 只接受 `Doc`,
//!   `create_sheet` 只接受 `Sheet`) —— 不匹配 = 永久错误, 防止把 bitable
//!   误发到 docx/sheet 端点。

use serde::{Deserialize, Serialize};

use crate::lark::error::LarkError;
use crate::lark::http::ApiRequest;

/// 文档标题字节上限。
pub const MAX_TITLE_BYTES: usize = 1024;

// ============================================================================
// §1 DocumentType (3 variant 闭合枚举)
// ============================================================================

/// 文档类型 (3 variant 闭合枚举)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    /// docx 文档 (`docx`)。
    #[default]
    Doc,
    /// spreadsheet (`sheet`)。
    Sheet,
    /// 多维表格 (`bitable`)。
    Bitable,
}

impl DocumentType {
    /// 3 variant hardcode 常量。
    pub const COUNT: usize = 3;

    /// wire 字符串。
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
// §2 Document (领域实体)
// ============================================================================

/// 文档 (领域实体, 覆盖 docx / spreadsheet / bitable 三种类型)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    /// 文档 ID (平台颁发后才有)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    /// 文档 token (URL 标识, 平台颁发后才有)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// 文档类型 (闭合枚举)。
    pub doc_type: DocumentType,
    /// 文档标题 (非空, ≤ 1024 字节)。
    pub title: String,
    /// 所在文件夹 token (根目录为 None)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder_token: Option<String>,
    /// 文档 URL (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 所有者 open_id (可选, K-1 #4)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_open_id: Option<String>,
    /// 创建时间 (RFC3339, 可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// 最后修改时间 (RFC3339, 可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl Document {
    /// 创建新 docx 文档 (title 校验)。
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

    /// 创建新 spreadsheet (title 校验)。
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

    /// 创建新多维表格 (title 校验)。
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

    /// title 校验 (非空 + ≤ [`MAX_TITLE_BYTES`])。
    fn validate_title(title: &str) -> Result<(), LarkError> {
        if title.trim().is_empty() {
            return Err(LarkError::Other("document title is empty".to_string()));
        }
        if title.len() > MAX_TITLE_BYTES {
            return Err(LarkError::Other(format!(
                "document title too long: {} > {MAX_TITLE_BYTES}",
                title.len()
            )));
        }
        Ok(())
    }

    /// 字段校验 (title + owner K-1 #4)。
    pub fn validate(&self) -> Result<(), LarkError> {
        Self::validate_title(&self.title)?;
        if let Some(owner) = &self.owner_open_id {
            LarkError::validate_open_id(owner)?;
        }
        Ok(())
    }

    /// 设置所有者 (K-1 #4)。
    pub fn with_owner(mut self, open_id: String) -> Result<Self, LarkError> {
        LarkError::validate_open_id(&open_id)?;
        self.owner_open_id = Some(open_id);
        Ok(self)
    }
}

// ============================================================================
// §3 SheetMeta / BitableMeta (跟 Document 配合的元数据)
// ============================================================================

/// Sheet 元数据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetMeta {
    /// Sheet ID。
    pub sheet_id: String,
    /// Sheet 标题 (默认 "Sheet1")。
    pub title: String,
    /// 索引位置 (0-based)。
    pub index: u32,
    /// 行数 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_count: Option<u32>,
    /// 列数 (可选)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_count: Option<u32>,
    /// 是否隐藏。
    #[serde(default)]
    pub is_hidden: bool,
}

impl SheetMeta {
    /// 创建新 sheet 元数据。
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

/// 多维表格元数据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitableMeta {
    /// Table ID。
    pub table_id: String,
    /// Table 名称。
    pub name: String,
    /// 字段列表。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<BitableField>,
}

/// 多维表格字段。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitableField {
    /// 字段名 (非空)。
    pub field_name: String,
    /// 字段类型 (e.g. "text" / "number" / "date" / "single_select")。
    #[serde(rename = "type")]
    pub field_type: String,
    /// 是否必填。
    #[serde(default)]
    pub is_required: bool,
}

// ============================================================================
// §4 wire 契约 (请求构造 + 响应映射)
// ============================================================================

fn create_body(doc: &Document) -> Result<serde_json::Value, LarkError> {
    doc.validate()?;
    let mut body = serde_json::Map::new();
    body.insert(
        "title".to_string(),
        serde_json::Value::String(doc.title.clone()),
    );
    if let Some(folder) = &doc.folder_token {
        if !folder.is_empty() {
            body.insert(
                "folder_token".to_string(),
                serde_json::Value::String(folder.clone()),
            );
        }
    }
    Ok(serde_json::Value::Object(body))
}

/// 构造 docx 创建请求 (`POST /docx/v1/documents`)。
///
/// `doc.doc_type != Doc` = 永久错误 (端点与类型必须匹配)。
pub fn build_create_doc_request(doc: &Document) -> Result<ApiRequest, LarkError> {
    if doc.doc_type != DocumentType::Doc {
        return Err(LarkError::Other(format!(
            "doc_type mismatch for create_doc: {} (expected docx)",
            doc.doc_type
        )));
    }
    Ok(ApiRequest::post("/docx/v1/documents", create_body(doc)?))
}

/// 构造 spreadsheet 创建请求 (`POST /sheets/v3/spreadsheets`)。
///
/// `doc.doc_type != Sheet` = 永久错误 (端点与类型必须匹配)。
pub fn build_create_sheet_request(doc: &Document) -> Result<ApiRequest, LarkError> {
    if doc.doc_type != DocumentType::Sheet {
        return Err(LarkError::Other(format!(
            "doc_type mismatch for create_sheet: {} (expected sheet)",
            doc.doc_type
        )));
    }
    Ok(ApiRequest::post(
        "/sheets/v3/spreadsheets",
        create_body(doc)?,
    ))
}

/// docx 创建响应 `data` 载荷 (未知字段容错; `document_id` 必填非空)。
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDocData {
    /// 文档对象。
    pub document: DocxDocumentWire,
}

/// docx 文档 wire 形状。
#[derive(Debug, Clone, Deserialize)]
pub struct DocxDocumentWire {
    /// 文档 ID (必填)。
    pub document_id: String,
    /// 标题 (可选)。
    #[serde(default)]
    pub title: Option<String>,
    /// 所有者 open_id (可选)。
    #[serde(default)]
    pub owner_open_id: Option<String>,
    /// 创建时间 (可选, RFC3339)。
    #[serde(default)]
    pub created_at: Option<String>,
}

/// spreadsheet 创建响应 `data` 载荷 (未知字段容错; `spreadsheet_token` 必填非空)。
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSheetData {
    /// spreadsheet 对象。
    pub spreadsheet: SpreadsheetWire,
}

/// spreadsheet wire 形状。
#[derive(Debug, Clone, Deserialize)]
pub struct SpreadsheetWire {
    /// spreadsheet token (必填)。
    pub spreadsheet_token: String,
    /// URL (可选)。
    #[serde(default)]
    pub url: Option<String>,
    /// 标题 (可选)。
    #[serde(default)]
    pub title: Option<String>,
}

/// docx wire → 领域 (document_id 非空校验)。
pub fn map_created_doc(
    template: &Document,
    wire: &DocxDocumentWire,
) -> Result<Document, LarkError> {
    if wire.document_id.trim().is_empty() {
        return Err(LarkError::Other(
            "malformed response: empty document_id".to_string(),
        ));
    }
    let mut doc = template.clone();
    doc.document_id = Some(wire.document_id.clone());
    doc.token = Some(wire.document_id.clone());
    doc.title = wire.title.clone().unwrap_or_else(|| template.title.clone());
    doc.owner_open_id = wire
        .owner_open_id
        .clone()
        .or_else(|| template.owner_open_id.clone());
    doc.created_at = wire.created_at.clone();
    doc.validate()?;
    Ok(doc)
}

/// spreadsheet wire → 领域 (spreadsheet_token 非空校验)。
pub fn map_created_sheet(
    template: &Document,
    wire: &SpreadsheetWire,
) -> Result<Document, LarkError> {
    if wire.spreadsheet_token.trim().is_empty() {
        return Err(LarkError::Other(
            "malformed response: empty spreadsheet_token".to_string(),
        ));
    }
    let mut doc = template.clone();
    doc.document_id = Some(wire.spreadsheet_token.clone());
    doc.token = Some(wire.spreadsheet_token.clone());
    doc.url = wire.url.clone();
    doc.title = wire.title.clone().unwrap_or_else(|| template.title.clone());
    doc.validate()?;
    Ok(doc)
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
    fn document_constructors() {
        let doc = Document::new_docx("项目计划".to_string(), None).expect("valid");
        assert_eq!(doc.doc_type, DocumentType::Doc);
        assert_eq!(doc.title, "项目计划");
        assert_eq!(
            Document::new_sheet("预算表".to_string(), None)
                .expect("valid")
                .doc_type,
            DocumentType::Sheet
        );
        assert_eq!(
            Document::new_bitable("任务列表".to_string(), None)
                .expect("valid")
                .doc_type,
            DocumentType::Bitable
        );
    }

    #[test]
    fn document_title_boundaries() {
        assert!(Document::new_docx(String::new(), None).is_err());
        assert!(Document::new_docx("   ".to_string(), None).is_err());
        let long_title = "x".repeat(2000);
        assert!(Document::new_docx(long_title, None).is_err());
        let max_title = "x".repeat(MAX_TITLE_BYTES);
        assert!(Document::new_docx(max_title, None).is_ok());
    }

    #[test]
    fn document_with_owner_validation() {
        let doc = Document::new_docx("title".to_string(), None)
            .expect("valid")
            .with_owner("ou_owner1234567890abcdef".to_string())
            .expect("valid owner");
        assert_eq!(
            doc.owner_open_id.as_deref(),
            Some("ou_owner1234567890abcdef")
        );
        assert!(doc.validate().is_ok());
        assert!(matches!(
            Document::new_docx("title".to_string(), None)
                .expect("valid")
                .with_owner("invalid".to_string()),
            Err(LarkError::OpenIdInvalid(_))
        ));
    }

    #[test]
    fn sheet_and_bitable_meta() {
        let meta = SheetMeta::new("sheet_001".to_string(), "Sheet1".to_string(), 0);
        assert_eq!(meta.sheet_id, "sheet_001");
        assert_eq!(meta.index, 0);
        let bitable = BitableMeta {
            table_id: "tbl_001".to_string(),
            name: "Tasks".to_string(),
            fields: vec![BitableField {
                field_name: "title".to_string(),
                field_type: "text".to_string(),
                is_required: true,
            }],
        };
        assert_eq!(bitable.fields.len(), 1);
    }

    // ---- wire 契约 ----

    #[test]
    fn build_create_requests_enforce_endpoint_type_match() {
        let doc =
            Document::new_docx("title".to_string(), Some("fld_1".to_string())).expect("valid");
        let req = build_create_doc_request(&doc).expect("request");
        assert_eq!(req.path, "/docx/v1/documents");
        let body = req.body.expect("body");
        assert_eq!(body["title"], "title");
        assert_eq!(body["folder_token"], "fld_1");
        // docx 文档不能发到 sheet 端点
        assert!(build_create_sheet_request(&doc).is_err());

        let sheet = Document::new_sheet("budget".to_string(), None).expect("valid");
        let req = build_create_sheet_request(&sheet).expect("request");
        assert_eq!(req.path, "/sheets/v3/spreadsheets");
        assert!(build_create_doc_request(&sheet).is_err());
    }

    #[test]
    fn create_doc_response_maps_and_tolerates_unknown_fields() {
        let template = Document::new_docx("title".to_string(), None).expect("valid");
        let data: CreateDocData = serde_json::from_value(serde_json::json!({
            "document": {
                "document_id": "doxcnabc123",
                "title": "title",
                "future_field": [1, 2]
            }
        }))
        .expect("parse");
        let doc = map_created_doc(&template, &data.document).expect("map");
        assert_eq!(doc.document_id.as_deref(), Some("doxcnabc123"));
        assert_eq!(doc.doc_type, DocumentType::Doc);
        assert!(doc.validate().is_ok());

        // 空 document_id → 永久错误
        let data: CreateDocData = serde_json::from_value(serde_json::json!({
            "document": {"document_id": "  "}
        }))
        .expect("parse");
        assert!(matches!(
            map_created_doc(&template, &data.document),
            Err(LarkError::Other(_))
        ));
    }

    #[test]
    fn create_sheet_response_maps_token_and_url() {
        let template = Document::new_sheet("budget".to_string(), None).expect("valid");
        let data: CreateSheetData = serde_json::from_value(serde_json::json!({
            "spreadsheet": {
                "spreadsheet_token": "shtcnabc123",
                "url": "https://docs.example.test/sheets/shtcnabc123"
            }
        }))
        .expect("parse");
        let doc = map_created_sheet(&template, &data.spreadsheet).expect("map");
        assert_eq!(doc.document_id.as_deref(), Some("shtcnabc123"));
        assert_eq!(
            doc.url.as_deref(),
            Some("https://docs.example.test/sheets/shtcnabc123")
        );

        // 缺 spreadsheet_token → serde 拒 (严格必填)
        let result = serde_json::from_value::<CreateSheetData>(serde_json::json!({
            "spreadsheet": {"url": "https://x"}
        }));
        assert!(result.is_err());
    }
}
