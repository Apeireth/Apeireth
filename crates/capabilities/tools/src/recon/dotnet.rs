//! `re.dotnet`：.NET / CLR 元数据探测（零新依赖）。
//!
//! 解析 PE → CLI 头 → 元数据根 → 流表与 `#~` 表流，回答"是不是 .NET 程序集、
//! 什么运行时版本、有哪些堆"，并枚举 TypeDef / MethodDef / TypeRef /
//! AssemblyRef 四张表。方法签名、方法体及其余元数据表不在本工具范围内，
//! 显式写进 limitations。

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// .NET 探测结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DotnetReport {
    /// 是否带 CLR 头。
    pub is_dotnet: bool,
    /// 运行时版本（CLI 头 major.minor）。
    pub runtime_version: String,
    /// 元数据版本字符串（如 `v4.0.30319`）。
    pub metadata_version: String,
    /// 元数据流（`#~` / `#Strings` / `#US` / `#GUID` / `#Blob` …）。
    pub streams: Vec<String>,
    /// `TypeDef` 表条目（按表行顺序）。
    pub type_defs: Vec<TypeDefEntry>,
    /// `MethodDef` 表条目（按表行顺序）。
    pub method_defs: Vec<MethodDefEntry>,
    /// `TypeRef` 表条目（按表行顺序）。
    pub type_refs: Vec<TypeRefEntry>,
    /// `AssemblyRef` 表条目（按表行顺序）。
    pub assembly_refs: Vec<AssemblyRefEntry>,
    /// 入口 token（CLI 头里的 EntryPointToken）。
    pub entry_point_token: u32,
    /// 显式局限。
    pub limitations: Vec<String>,
}

/// `TypeDef` 表的一行。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeDefEntry {
    /// 类型所属命名空间；全局类型为空串。
    pub namespace: String,
    /// 不含命名空间的类型名。
    pub name: String,
}

/// `MethodDef` 表的一行。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodDefEntry {
    /// 方法名。
    pub name: String,
    /// 方法体 RVA；为 0（无 IL 体，如 abstract / P/Invoke）时为 `None`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rva: Option<u32>,
}

/// `TypeRef` 表的一行。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeRefEntry {
    /// 引用类型所属命名空间。
    pub namespace: String,
    /// 引用类型名。
    pub name: String,
}

/// `AssemblyRef` 表的一行。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssemblyRefEntry {
    /// 被引用程序集名。
    pub name: String,
}

/// 探测错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DotnetError {
    /// 不是 PE。
    #[error("not a PE file")]
    NotPe,
    /// PE 结构越界。
    #[error("truncated PE structure")]
    Truncated,
    /// 没有 CLI 头（不是 .NET 程序集）。
    #[error("PE has no CLI header")]
    NotDotnet,
    /// 元数据畸形。
    #[error("malformed CLR metadata: {0}")]
    BadMetadata(String),
}

const CLR_DIRECTORY_INDEX: usize = 14;

/// ECMA-335 II.22 元数据表编号（0x00..=0x2C）。
mod table {
    pub const MODULE: usize = 0x00;
    pub const TYPE_REF: usize = 0x01;
    pub const TYPE_DEF: usize = 0x02;
    pub const FIELD_PTR: usize = 0x03;
    pub const FIELD: usize = 0x04;
    pub const METHOD_PTR: usize = 0x05;
    pub const METHOD_DEF: usize = 0x06;
    pub const PARAM_PTR: usize = 0x07;
    pub const PARAM: usize = 0x08;
    pub const INTERFACE_IMPL: usize = 0x09;
    pub const MEMBER_REF: usize = 0x0A;
    pub const CONSTANT: usize = 0x0B;
    pub const CUSTOM_ATTRIBUTE: usize = 0x0C;
    pub const FIELD_MARSHAL: usize = 0x0D;
    pub const DECL_SECURITY: usize = 0x0E;
    pub const CLASS_LAYOUT: usize = 0x0F;
    pub const FIELD_LAYOUT: usize = 0x10;
    pub const STAND_ALONE_SIG: usize = 0x11;
    pub const EVENT_MAP: usize = 0x12;
    pub const EVENT_PTR: usize = 0x13;
    pub const EVENT: usize = 0x14;
    pub const PROPERTY_MAP: usize = 0x15;
    pub const PROPERTY_PTR: usize = 0x16;
    pub const PROPERTY: usize = 0x17;
    pub const METHOD_SEMANTICS: usize = 0x18;
    pub const METHOD_IMPL: usize = 0x19;
    pub const MODULE_REF: usize = 0x1A;
    pub const TYPE_SPEC: usize = 0x1B;
    pub const IMPL_MAP: usize = 0x1C;
    pub const FIELD_RVA: usize = 0x1D;
    pub const ENC_LOG: usize = 0x1E;
    pub const ENC_MAP: usize = 0x1F;
    pub const ASSEMBLY: usize = 0x20;
    pub const ASSEMBLY_PROCESSOR: usize = 0x21;
    pub const ASSEMBLY_OS: usize = 0x22;
    pub const ASSEMBLY_REF: usize = 0x23;
    pub const ASSEMBLY_REF_PROCESSOR: usize = 0x24;
    pub const ASSEMBLY_REF_OS: usize = 0x25;
    pub const FILE: usize = 0x26;
    pub const EXPORTED_TYPE: usize = 0x27;
    pub const MANIFEST_RESOURCE: usize = 0x28;
    pub const NESTED_CLASS: usize = 0x29;
    pub const GENERIC_PARAM: usize = 0x2A;
    pub const METHOD_SPEC: usize = 0x2B;
    pub const GENERIC_PARAM_CONSTRAINT: usize = 0x2C;
}

const TABLE_COUNT: usize = table::GENERIC_PARAM_CONSTRAINT + 1;
const TABLE_HEADER_SIZE: usize = 24;

/// 探测一段 PE 字节。
pub fn detect(bytes: &[u8]) -> Result<DotnetReport, DotnetError> {
    let pe = PeView::parse(bytes)?;
    let Some((cli_rva, _)) = pe.data_directory(CLR_DIRECTORY_INDEX) else {
        return Err(DotnetError::NotDotnet);
    };
    if cli_rva == 0 {
        return Err(DotnetError::NotDotnet);
    }
    let cli_offset = pe.rva_to_offset(cli_rva)?;
    let cli = read_u32(bytes, cli_offset)?;
    if cli < 0x48 {
        return Err(DotnetError::BadMetadata("CLI header too small".into()));
    }
    let runtime_major = read_u16(bytes, cli_offset + 4)?;
    let runtime_minor = read_u16(bytes, cli_offset + 6)?;
    let metadata_rva = read_u32(bytes, cli_offset + 8)?;
    let entry_point_token = read_u32(bytes, cli_offset + 20)?;
    let metadata_offset = pe.rva_to_offset(metadata_rva)?;

    let signature = read_u32(bytes, metadata_offset)?;
    if signature != 0x424A_5342 {
        return Err(DotnetError::BadMetadata("bad BSJB signature".into()));
    }
    let version_length = read_u32(bytes, metadata_offset + 12)? as usize;
    if version_length > 255 {
        return Err(DotnetError::BadMetadata("version string too long".into()));
    }
    let version_start = metadata_offset
        .checked_add(16)
        .ok_or(DotnetError::Truncated)?;
    let version_end = version_start
        .checked_add(version_length)
        .ok_or(DotnetError::Truncated)?;
    let version_bytes = bytes
        .get(version_start..version_end)
        .ok_or(DotnetError::Truncated)?;
    let metadata_version = String::from_utf8_lossy(version_bytes)
        .trim_end_matches('\0')
        .to_string();

    let after_version = version_end.checked_add(3).ok_or(DotnetError::Truncated)? & !3;
    let stream_count = read_u16(bytes, after_version + 2)? as usize;
    if stream_count > 64 {
        return Err(DotnetError::BadMetadata("too many streams".into()));
    }
    let mut streams = Vec::new();
    let mut table_stream: Option<(u32, u32)> = None;
    let mut strings_stream: Option<(u32, u32)> = None;
    let mut cursor = after_version.checked_add(4).ok_or(DotnetError::Truncated)?;
    for _ in 0..stream_count {
        let name_start = cursor.checked_add(8).ok_or(DotnetError::Truncated)?;
        let stream_offset = read_u32(bytes, cursor)?;
        let stream_size = read_u32(bytes, cursor.checked_add(4).ok_or(DotnetError::Truncated)?)?;
        let name_limit = name_start
            .checked_add(32)
            .ok_or(DotnetError::Truncated)?
            .min(bytes.len());
        let name_bytes = bytes
            .get(name_start..name_limit)
            .ok_or(DotnetError::Truncated)?;
        let name_end = name_bytes
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(DotnetError::Truncated)?;
        let name = String::from_utf8_lossy(&name_bytes[..name_end]).to_string();
        if !name.is_empty() {
            match name.as_str() {
                "#~" => table_stream = Some((stream_offset, stream_size)),
                "#-" if table_stream.is_none() => {
                    table_stream = Some((stream_offset, stream_size));
                }
                "#Strings" if strings_stream.is_none() => {
                    strings_stream = Some((stream_offset, stream_size));
                }
                _ => {}
            }
            streams.push(name);
        }
        let name_len = (name_end + 1 + 3) & !3;
        cursor = name_start
            .checked_add(name_len)
            .ok_or(DotnetError::Truncated)?;
    }

    let tables = match table_stream {
        Some((stream_offset, stream_size)) if stream_size > 0 => {
            let table_data = stream_slice(bytes, metadata_offset, stream_offset, stream_size)?;
            let strings = match strings_stream {
                Some((offset, size)) => stream_slice(bytes, metadata_offset, offset, size)?,
                None => &[],
            };
            parse_table_stream(table_data, strings)?
        }
        _ => ParsedTables::default(),
    };

    let mut limitations = vec![
        "method signatures/blobs not decoded".to_string(),
        "method bodies and IL are not decoded".to_string(),
        "only TypeDef, MethodDef, TypeRef and AssemblyRef rows are enumerated; other metadata tables are skipped".to_string(),
        "coded indexes (Extends, ResolutionScope, declarers) are not resolved".to_string(),
        "assembly references are listed by name only; no binding or resolution".to_string(),
    ];
    match table_stream {
        None => limitations
            .push("no #~ table stream present; type and assembly tables are empty".to_string()),
        Some((_, 0)) => limitations
            .push("empty #~ table stream; type and assembly tables are empty".to_string()),
        Some(_) => {}
    }

    Ok(DotnetReport {
        is_dotnet: true,
        runtime_version: format!("{runtime_major}.{runtime_minor}"),
        metadata_version,
        streams,
        type_defs: tables.type_defs,
        method_defs: tables.method_defs,
        type_refs: tables.type_refs,
        assembly_refs: tables.assembly_refs,
        entry_point_token,
        limitations,
    })
}

fn stream_slice<'a>(
    bytes: &'a [u8],
    metadata_offset: usize,
    stream_offset: u32,
    stream_size: u32,
) -> Result<&'a [u8], DotnetError> {
    let start = metadata_offset
        .checked_add(stream_offset as usize)
        .ok_or(DotnetError::Truncated)?;
    let end = start
        .checked_add(stream_size as usize)
        .ok_or(DotnetError::Truncated)?;
    bytes.get(start..end).ok_or(DotnetError::Truncated)
}

/// `#~` 表流里与目标四表相关的解析结果。
#[derive(Default)]
struct ParsedTables {
    type_defs: Vec<TypeDefEntry>,
    method_defs: Vec<MethodDefEntry>,
    type_refs: Vec<TypeRefEntry>,
    assembly_refs: Vec<AssemblyRefEntry>,
}

/// 解析 `#~`（或 `#-`）表流的头、行宽和目标四表的内容。
fn parse_table_stream(data: &[u8], strings: &[u8]) -> Result<ParsedTables, DotnetError> {
    if data.len() < TABLE_HEADER_SIZE {
        return Err(DotnetError::Truncated);
    }
    let reserved = read_u32(data, 0)?;
    if reserved != 0 {
        return Err(DotnetError::BadMetadata(format!(
            "table stream reserved field is 0x{reserved:08x}, expected 0"
        )));
    }
    let major = data[4];
    if major > 2 {
        return Err(DotnetError::BadMetadata(format!(
            "unsupported table stream major version {major}"
        )));
    }
    let heap_sizes = data[6];
    let valid = read_u64(data, 8)?;
    if valid >> TABLE_COUNT != 0 {
        let unknown = (valid >> TABLE_COUNT).trailing_zeros() as usize + TABLE_COUNT;
        return Err(DotnetError::BadMetadata(format!(
            "unknown metadata table 0x{unknown:02x}"
        )));
    }

    let mut row_counts = [0u32; TABLE_COUNT];
    let mut cursor = TABLE_HEADER_SIZE;
    for (index, rows) in row_counts.iter_mut().enumerate() {
        if valid & (1u64 << index) != 0 {
            *rows = read_u32(data, cursor)?;
            cursor = cursor.checked_add(4).ok_or(DotnetError::Truncated)?;
        }
    }

    let mut row_sizes = [0usize; TABLE_COUNT];
    for (index, size) in row_sizes.iter_mut().enumerate() {
        let columns = table_columns(index).ok_or_else(|| {
            DotnetError::BadMetadata(format!("unknown metadata table 0x{index:02x}"))
        })?;
        *size = row_width(columns, heap_sizes, &row_counts);
    }

    let mut table_offsets = [0usize; TABLE_COUNT];
    let mut offset = cursor as u64;
    for index in 0..TABLE_COUNT {
        table_offsets[index] = usize::try_from(offset).map_err(|_| DotnetError::Truncated)?;
        let body_size = u64::from(row_counts[index])
            .checked_mul(row_sizes[index] as u64)
            .ok_or(DotnetError::Truncated)?;
        offset = offset
            .checked_add(body_size)
            .ok_or(DotnetError::Truncated)?;
    }
    if offset > data.len() as u64 {
        return Err(DotnetError::Truncated);
    }

    let string_width = heap_width(heap_sizes, 0x01);

    let type_defs = {
        let name_field = field_offset(table::TYPE_DEF, 1, heap_sizes, &row_counts)?;
        let namespace_field = field_offset(table::TYPE_DEF, 2, heap_sizes, &row_counts)?;
        let count = row_counts[table::TYPE_DEF];
        let table_offset = table_offsets[table::TYPE_DEF];
        let row_size = row_sizes[table::TYPE_DEF];
        let mut entries = Vec::with_capacity(count as usize);
        for index in 0..count {
            let row = table_row(data, table_offset, row_size, index)?;
            entries.push(TypeDefEntry {
                namespace: read_heap_string(
                    strings,
                    read_heap_index(row, namespace_field, string_width)?,
                )?,
                name: read_heap_string(strings, read_heap_index(row, name_field, string_width)?)?,
            });
        }
        entries
    };

    let method_defs = {
        let name_field = field_offset(table::METHOD_DEF, 3, heap_sizes, &row_counts)?;
        let count = row_counts[table::METHOD_DEF];
        let table_offset = table_offsets[table::METHOD_DEF];
        let row_size = row_sizes[table::METHOD_DEF];
        let mut entries = Vec::with_capacity(count as usize);
        for index in 0..count {
            let row = table_row(data, table_offset, row_size, index)?;
            let rva = read_u32(row, 0)?;
            entries.push(MethodDefEntry {
                name: read_heap_string(strings, read_heap_index(row, name_field, string_width)?)?,
                rva: (rva != 0).then_some(rva),
            });
        }
        entries
    };

    let type_refs = {
        let name_field = field_offset(table::TYPE_REF, 1, heap_sizes, &row_counts)?;
        let namespace_field = field_offset(table::TYPE_REF, 2, heap_sizes, &row_counts)?;
        let count = row_counts[table::TYPE_REF];
        let table_offset = table_offsets[table::TYPE_REF];
        let row_size = row_sizes[table::TYPE_REF];
        let mut entries = Vec::with_capacity(count as usize);
        for index in 0..count {
            let row = table_row(data, table_offset, row_size, index)?;
            entries.push(TypeRefEntry {
                namespace: read_heap_string(
                    strings,
                    read_heap_index(row, namespace_field, string_width)?,
                )?,
                name: read_heap_string(strings, read_heap_index(row, name_field, string_width)?)?,
            });
        }
        entries
    };

    let assembly_refs = {
        let name_field = field_offset(table::ASSEMBLY_REF, 6, heap_sizes, &row_counts)?;
        let count = row_counts[table::ASSEMBLY_REF];
        let table_offset = table_offsets[table::ASSEMBLY_REF];
        let row_size = row_sizes[table::ASSEMBLY_REF];
        let mut entries = Vec::with_capacity(count as usize);
        for index in 0..count {
            let row = table_row(data, table_offset, row_size, index)?;
            entries.push(AssemblyRefEntry {
                name: read_heap_string(strings, read_heap_index(row, name_field, string_width)?)?,
            });
        }
        entries
    };

    Ok(ParsedTables {
        type_defs,
        method_defs,
        type_refs,
        assembly_refs,
    })
}

fn table_row<'a>(
    data: &'a [u8],
    table_offset: usize,
    row_size: usize,
    index: u32,
) -> Result<&'a [u8], DotnetError> {
    let relative = (index as usize)
        .checked_mul(row_size)
        .ok_or(DotnetError::Truncated)?;
    let start = table_offset
        .checked_add(relative)
        .ok_or(DotnetError::Truncated)?;
    let end = start.checked_add(row_size).ok_or(DotnetError::Truncated)?;
    data.get(start..end).ok_or(DotnetError::Truncated)
}

fn read_heap_index(row: &[u8], offset: usize, width: usize) -> Result<u32, DotnetError> {
    match width {
        2 => Ok(u32::from(read_u16(row, offset)?)),
        4 => read_u32(row, offset),
        _ => Err(DotnetError::BadMetadata(format!(
            "unsupported heap index width {width}"
        ))),
    }
}

fn read_heap_string(heap: &[u8], index: u32) -> Result<String, DotnetError> {
    if index == 0 {
        return Ok(String::new());
    }
    let offset = index as usize;
    let tail = heap.get(offset..).ok_or_else(|| {
        DotnetError::BadMetadata(format!("string heap index 0x{index:x} out of range"))
    })?;
    let end = tail.iter().position(|byte| *byte == 0).ok_or_else(|| {
        DotnetError::BadMetadata(format!("unterminated string at heap index 0x{index:x}"))
    })?;
    Ok(String::from_utf8_lossy(&tail[..end]).into_owned())
}

fn table_columns(index: usize) -> Option<&'static [Column]> {
    match index {
        table::MODULE => Some(&[
            Column::Fixed(2),
            Column::String,
            Column::Guid,
            Column::Guid,
            Column::Guid,
        ]),
        table::TYPE_REF => Some(&[
            Column::Coded(CodedIndex::ResolutionScope),
            Column::String,
            Column::String,
        ]),
        table::TYPE_DEF => Some(&[
            Column::Fixed(4),
            Column::String,
            Column::String,
            Column::Coded(CodedIndex::TypeDefOrRef),
            Column::Simple(table::FIELD),
            Column::Simple(table::METHOD_DEF),
        ]),
        table::FIELD_PTR => Some(&[Column::Simple(table::FIELD)]),
        table::FIELD => Some(&[Column::Fixed(2), Column::String, Column::Blob]),
        table::METHOD_PTR => Some(&[Column::Simple(table::METHOD_DEF)]),
        table::METHOD_DEF => Some(&[
            Column::Fixed(4),
            Column::Fixed(2),
            Column::Fixed(2),
            Column::String,
            Column::Blob,
            Column::Simple(table::PARAM),
        ]),
        table::PARAM_PTR => Some(&[Column::Simple(table::PARAM)]),
        table::PARAM => Some(&[Column::Fixed(2), Column::Fixed(2), Column::String]),
        table::INTERFACE_IMPL => Some(&[
            Column::Simple(table::TYPE_DEF),
            Column::Coded(CodedIndex::TypeDefOrRef),
        ]),
        table::MEMBER_REF => Some(&[
            Column::Coded(CodedIndex::MemberRefParent),
            Column::String,
            Column::Blob,
        ]),
        table::CONSTANT => Some(&[
            Column::Fixed(2),
            Column::Coded(CodedIndex::HasConstant),
            Column::Blob,
        ]),
        table::CUSTOM_ATTRIBUTE => Some(&[
            Column::Coded(CodedIndex::HasCustomAttribute),
            Column::Coded(CodedIndex::CustomAttributeType),
            Column::Blob,
        ]),
        table::FIELD_MARSHAL => Some(&[Column::Coded(CodedIndex::HasFieldMarshal), Column::Blob]),
        table::DECL_SECURITY => Some(&[
            Column::Fixed(2),
            Column::Coded(CodedIndex::HasDeclSecurity),
            Column::Blob,
        ]),
        table::CLASS_LAYOUT => Some(&[
            Column::Fixed(2),
            Column::Fixed(4),
            Column::Simple(table::TYPE_DEF),
        ]),
        table::FIELD_LAYOUT => Some(&[Column::Fixed(4), Column::Simple(table::FIELD)]),
        table::STAND_ALONE_SIG => Some(&[Column::Blob]),
        table::EVENT_MAP => Some(&[
            Column::Simple(table::TYPE_DEF),
            Column::Simple(table::EVENT),
        ]),
        table::EVENT_PTR => Some(&[Column::Simple(table::EVENT)]),
        table::EVENT => Some(&[
            Column::Fixed(2),
            Column::String,
            Column::Coded(CodedIndex::TypeDefOrRef),
        ]),
        table::PROPERTY_MAP => Some(&[
            Column::Simple(table::TYPE_DEF),
            Column::Simple(table::PROPERTY),
        ]),
        table::PROPERTY_PTR => Some(&[Column::Simple(table::PROPERTY)]),
        table::PROPERTY => Some(&[Column::Fixed(2), Column::String, Column::Blob]),
        table::METHOD_SEMANTICS => Some(&[
            Column::Fixed(2),
            Column::Simple(table::METHOD_DEF),
            Column::Coded(CodedIndex::HasSemantics),
        ]),
        table::METHOD_IMPL => Some(&[
            Column::Simple(table::TYPE_DEF),
            Column::Coded(CodedIndex::MethodDefOrRef),
            Column::Coded(CodedIndex::MethodDefOrRef),
        ]),
        table::MODULE_REF => Some(&[Column::String]),
        table::TYPE_SPEC => Some(&[Column::Blob]),
        table::IMPL_MAP => Some(&[
            Column::Fixed(2),
            Column::Coded(CodedIndex::MemberForwarded),
            Column::String,
            Column::Simple(table::MODULE_REF),
        ]),
        table::FIELD_RVA => Some(&[Column::Fixed(4), Column::Simple(table::FIELD)]),
        table::ENC_LOG => Some(&[Column::Fixed(4), Column::Fixed(4)]),
        table::ENC_MAP => Some(&[Column::Fixed(4)]),
        table::ASSEMBLY => Some(&[
            Column::Fixed(4),
            Column::Fixed(2),
            Column::Fixed(2),
            Column::Fixed(2),
            Column::Fixed(2),
            Column::Fixed(4),
            Column::Blob,
            Column::String,
            Column::String,
        ]),
        table::ASSEMBLY_PROCESSOR => Some(&[Column::Fixed(4)]),
        table::ASSEMBLY_OS => Some(&[Column::Fixed(4), Column::Fixed(4), Column::Fixed(4)]),
        table::ASSEMBLY_REF => Some(&[
            Column::Fixed(2),
            Column::Fixed(2),
            Column::Fixed(2),
            Column::Fixed(2),
            Column::Fixed(4),
            Column::Blob,
            Column::String,
            Column::String,
            Column::Blob,
        ]),
        table::ASSEMBLY_REF_PROCESSOR => {
            Some(&[Column::Fixed(4), Column::Simple(table::ASSEMBLY_REF)])
        }
        table::ASSEMBLY_REF_OS => Some(&[
            Column::Fixed(4),
            Column::Fixed(4),
            Column::Fixed(4),
            Column::Simple(table::ASSEMBLY_REF),
        ]),
        table::FILE => Some(&[Column::Fixed(4), Column::String, Column::Blob]),
        table::EXPORTED_TYPE => Some(&[
            Column::Fixed(4),
            Column::Fixed(4),
            Column::String,
            Column::String,
            Column::Coded(CodedIndex::Implementation),
        ]),
        table::MANIFEST_RESOURCE => Some(&[
            Column::Fixed(4),
            Column::Fixed(4),
            Column::String,
            Column::Coded(CodedIndex::Implementation),
        ]),
        table::NESTED_CLASS => Some(&[
            Column::Simple(table::TYPE_DEF),
            Column::Simple(table::TYPE_DEF),
        ]),
        table::GENERIC_PARAM => Some(&[
            Column::Fixed(2),
            Column::Fixed(2),
            Column::Coded(CodedIndex::TypeOrMethodDef),
            Column::String,
        ]),
        table::METHOD_SPEC => Some(&[Column::Coded(CodedIndex::MethodDefOrRef), Column::Blob]),
        table::GENERIC_PARAM_CONSTRAINT => Some(&[
            Column::Simple(table::GENERIC_PARAM),
            Column::Coded(CodedIndex::TypeDefOrRef),
        ]),
        _ => None,
    }
}

/// 表列类型；行宽完全由堆索引大小、简单索引大小和 coded index 大小决定。
#[derive(Clone, Copy)]
enum Column {
    Fixed(u8),
    String,
    Guid,
    Blob,
    Simple(usize),
    Coded(CodedIndex),
}

/// ECMA-335 II.24.2.6 的 coded index（tag 位数 + 引用表集合）。
#[derive(Clone, Copy)]
enum CodedIndex {
    TypeDefOrRef,
    HasConstant,
    HasCustomAttribute,
    HasFieldMarshal,
    HasDeclSecurity,
    MemberRefParent,
    HasSemantics,
    MethodDefOrRef,
    MemberForwarded,
    Implementation,
    CustomAttributeType,
    ResolutionScope,
    TypeOrMethodDef,
}

impl CodedIndex {
    fn tag_bits(self) -> u32 {
        match self {
            Self::TypeDefOrRef
            | Self::HasConstant
            | Self::HasDeclSecurity
            | Self::Implementation
            | Self::ResolutionScope => 2,
            Self::HasCustomAttribute => 5,
            Self::MemberRefParent | Self::CustomAttributeType => 3,
            Self::HasFieldMarshal
            | Self::HasSemantics
            | Self::MethodDefOrRef
            | Self::MemberForwarded
            | Self::TypeOrMethodDef => 1,
        }
    }

    fn tables(self) -> &'static [usize] {
        match self {
            Self::TypeDefOrRef => &[table::TYPE_DEF, table::TYPE_REF, table::TYPE_SPEC],
            Self::HasConstant => &[table::FIELD, table::PARAM, table::PROPERTY],
            Self::HasCustomAttribute => &[
                table::METHOD_DEF,
                table::FIELD,
                table::TYPE_REF,
                table::TYPE_DEF,
                table::PARAM,
                table::INTERFACE_IMPL,
                table::MEMBER_REF,
                table::MODULE,
                table::DECL_SECURITY,
                table::PROPERTY,
                table::EVENT,
                table::STAND_ALONE_SIG,
                table::MODULE_REF,
                table::TYPE_SPEC,
                table::ASSEMBLY,
                table::ASSEMBLY_REF,
                table::FILE,
                table::EXPORTED_TYPE,
                table::MANIFEST_RESOURCE,
                table::GENERIC_PARAM,
                table::GENERIC_PARAM_CONSTRAINT,
                table::METHOD_SPEC,
            ],
            Self::HasFieldMarshal => &[table::FIELD, table::PARAM],
            Self::HasDeclSecurity => &[table::TYPE_DEF, table::METHOD_DEF, table::ASSEMBLY],
            Self::MemberRefParent => &[
                table::TYPE_DEF,
                table::TYPE_REF,
                table::MODULE_REF,
                table::METHOD_DEF,
                table::TYPE_SPEC,
            ],
            Self::HasSemantics => &[table::EVENT, table::PROPERTY],
            Self::MethodDefOrRef => &[table::METHOD_DEF, table::MEMBER_REF],
            Self::MemberForwarded => &[table::FIELD, table::METHOD_DEF],
            Self::Implementation => &[table::FILE, table::ASSEMBLY_REF, table::EXPORTED_TYPE],
            // CustomAttributeType 的合法 tag 是 2 (MethodDef) / 3 (MemberRef)，
            // 但宽度仍按 3 位 tag 计算。
            Self::CustomAttributeType => &[table::METHOD_DEF, table::MEMBER_REF],
            Self::ResolutionScope => &[
                table::MODULE,
                table::MODULE_REF,
                table::ASSEMBLY_REF,
                table::TYPE_REF,
            ],
            Self::TypeOrMethodDef => &[table::TYPE_DEF, table::METHOD_DEF],
        }
    }
}

fn row_width(columns: &[Column], heap_sizes: u8, row_counts: &[u32; TABLE_COUNT]) -> usize {
    columns
        .iter()
        .map(|column| column_width(*column, heap_sizes, row_counts))
        .sum()
}

fn field_offset(
    table_index: usize,
    field: usize,
    heap_sizes: u8,
    row_counts: &[u32; TABLE_COUNT],
) -> Result<usize, DotnetError> {
    let columns = table_columns(table_index).ok_or_else(|| {
        DotnetError::BadMetadata(format!("unknown metadata table 0x{table_index:02x}"))
    })?;
    let prefix = columns.get(..field).ok_or_else(|| {
        DotnetError::BadMetadata(format!(
            "table 0x{table_index:02x} field {field} out of range"
        ))
    })?;
    Ok(prefix
        .iter()
        .map(|column| column_width(*column, heap_sizes, row_counts))
        .sum())
}

fn column_width(column: Column, heap_sizes: u8, row_counts: &[u32; TABLE_COUNT]) -> usize {
    match column {
        Column::Fixed(size) => usize::from(size),
        Column::String => heap_width(heap_sizes, 0x01),
        Column::Guid => heap_width(heap_sizes, 0x02),
        Column::Blob => heap_width(heap_sizes, 0x04),
        Column::Simple(index) => simple_width(index, row_counts),
        Column::Coded(index) => coded_width(index, row_counts),
    }
}

fn heap_width(heap_sizes: u8, mask: u8) -> usize {
    if heap_sizes & mask != 0 {
        4
    } else {
        2
    }
}

fn simple_width(index: usize, row_counts: &[u32; TABLE_COUNT]) -> usize {
    if row_counts[index] < 0x1_0000 {
        2
    } else {
        4
    }
}

fn coded_width(index: CodedIndex, row_counts: &[u32; TABLE_COUNT]) -> usize {
    let max_rows = index
        .tables()
        .iter()
        .filter_map(|table_index| row_counts.get(*table_index).copied())
        .max()
        .unwrap_or(0);
    let limit = 1u32 << (16 - index.tag_bits());
    if max_rows < limit {
        2
    } else {
        4
    }
}

struct PeView<'a> {
    bytes: &'a [u8],
    optional_offset: usize,
    bits: u8,
    sections: Vec<(u32, u32, u32)>, // (virtual_address, virtual_size, raw_offset)
}

impl<'a> PeView<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self, DotnetError> {
        if bytes.len() < 0x40 || !bytes.starts_with(b"MZ") {
            return Err(DotnetError::NotPe);
        }
        let pe_offset = read_u32(bytes, 0x3c)? as usize;
        if bytes.get(pe_offset..pe_offset + 4) != Some(b"PE\0\0") {
            return Err(DotnetError::NotPe);
        }
        let coff = pe_offset + 4;
        let section_count = read_u16(bytes, coff + 2)? as usize;
        let optional_size = read_u16(bytes, coff + 16)? as usize;
        let optional_offset = coff + 20;
        let magic = read_u16(bytes, optional_offset)?;
        let bits = match magic {
            0x10b => 32,
            0x20b => 64,
            _ => return Err(DotnetError::NotPe),
        };
        let section_table = optional_offset + optional_size;
        let mut sections = Vec::new();
        for index in 0..section_count.min(96) {
            let base = section_table + index * 40;
            let virtual_size = read_u32(bytes, base + 8)?;
            let virtual_address = read_u32(bytes, base + 12)?;
            let raw_offset = read_u32(bytes, base + 20)?;
            sections.push((virtual_address, virtual_size, raw_offset));
        }
        Ok(Self {
            bytes,
            optional_offset,
            bits,
            sections,
        })
    }

    fn data_directory(&self, index: usize) -> Option<(u32, u32)> {
        let base = self.optional_offset + if self.bits == 64 { 112 } else { 96 };
        let entry = base + index * 8;
        let rva = read_u32(self.bytes, entry).ok()?;
        let size = read_u32(self.bytes, entry + 4).ok()?;
        Some((rva, size))
    }

    fn rva_to_offset(&self, rva: u32) -> Result<usize, DotnetError> {
        for (virtual_address, virtual_size, raw_offset) in &self.sections {
            let start = *virtual_address;
            let span = (*virtual_size).max(0x1000);
            if rva >= start && rva < start.saturating_add(span) {
                let delta = rva - start;
                let offset = raw_offset.checked_add(delta).ok_or_else(|| {
                    DotnetError::BadMetadata(format!("rva 0x{rva:x} offset overflow"))
                })?;
                return Ok(offset as usize);
            }
        }
        Err(DotnetError::BadMetadata(format!("rva 0x{rva:x} unmapped")))
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, DotnetError> {
    let end = offset.checked_add(2).ok_or(DotnetError::Truncated)?;
    let raw = bytes.get(offset..end).ok_or(DotnetError::Truncated)?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, DotnetError> {
    let end = offset.checked_add(4).ok_or(DotnetError::Truncated)?;
    let raw = bytes.get(offset..end).ok_or(DotnetError::Truncated)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, DotnetError> {
    let end = offset.checked_add(8).ok_or(DotnetError::Truncated)?;
    let raw = bytes.get(offset..end).ok_or(DotnetError::Truncated)?;
    Ok(u64::from_le_bytes([
        raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_METADATA_OFFSET: usize = 0x240;
    const TEST_STREAM_BASE: usize = 0x80;

    /// 一个合法的空 `#~` 流：0 行、无表。
    fn empty_table_stream() -> Vec<u8> {
        build_table_stream(0, &[], &[])
    }

    /// 按 ECMA-335 II.24.2.6 拼一个表流头 + 行计数 + 行数据。
    fn build_table_stream(heap_sizes: u8, rows: &[(usize, u32)], body: &[u8]) -> Vec<u8> {
        debug_assert!(rows.windows(2).all(|pair| pair[0].0 < pair[1].0));
        let mut stream = Vec::new();
        stream.extend_from_slice(&0u32.to_le_bytes());
        stream.push(2);
        stream.push(0);
        stream.push(heap_sizes);
        stream.push(1);
        let mut valid = 0u64;
        for &(index, _) in rows {
            valid |= 1u64 << index;
        }
        stream.extend_from_slice(&valid.to_le_bytes());
        stream.extend_from_slice(&0u64.to_le_bytes());
        for &(_, count) in rows {
            stream.extend_from_slice(&count.to_le_bytes());
        }
        stream.extend_from_slice(body);
        stream
    }

    /// `#Strings` 堆：0 号恒为空串，返回每个名字的索引。
    fn string_heap(names: &[&str]) -> (Vec<u8>, Vec<u32>) {
        let mut heap = vec![0u8];
        let mut offsets = Vec::with_capacity(names.len());
        for name in names {
            offsets.push(heap.len() as u32);
            heap.extend_from_slice(name.as_bytes());
            heap.push(0);
        }
        (heap, offsets)
    }

    /// 构造最小 PE + CLI 头 + 元数据根，`streams` 里的数据放在固定区域。
    fn synthetic_dotnet_with_streams(streams: &[(&str, &[u8])]) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x1000];
        bytes[0] = b'M';
        bytes[1] = b'Z';
        bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&0xF0u16.to_le_bytes());
        bytes[0x98..0x9a].copy_from_slice(&0x20bu16.to_le_bytes());
        // 一个节：VA 0x2000，raw 0x200，大小 0xE00。
        let section = 0x188;
        bytes[section..section + 8].copy_from_slice(b".text\0\0\0");
        bytes[section + 8..section + 12].copy_from_slice(&0x0E00u32.to_le_bytes());
        bytes[section + 12..section + 16].copy_from_slice(&0x2000u32.to_le_bytes());
        bytes[section + 16..section + 20].copy_from_slice(&0x0E00u32.to_le_bytes());
        bytes[section + 20..section + 24].copy_from_slice(&0x200u32.to_le_bytes());
        // CLI 数据目录（索引 14）：RVA 0x2000, size 0x48。
        let data_dir = 0x98 + 112 + 14 * 8;
        bytes[data_dir..data_dir + 4].copy_from_slice(&0x2000u32.to_le_bytes());
        bytes[data_dir + 4..data_dir + 8].copy_from_slice(&0x48u32.to_le_bytes());
        // CLI 头在文件偏移 0x200。
        bytes[0x200..0x204].copy_from_slice(&0x48u32.to_le_bytes());
        bytes[0x204..0x206].copy_from_slice(&2u16.to_le_bytes());
        bytes[0x206..0x208].copy_from_slice(&5u16.to_le_bytes());
        let metadata_rva = 0x2040u32;
        bytes[0x208..0x20c].copy_from_slice(&metadata_rva.to_le_bytes());
        bytes[0x20c..0x210].copy_from_slice(&0xD00u32.to_le_bytes());
        bytes[0x214..0x218].copy_from_slice(&0x0600_0001u32.to_le_bytes());
        // 元数据根在 0x240。
        let meta = TEST_METADATA_OFFSET;
        bytes[meta..meta + 4].copy_from_slice(&0x424A_5342u32.to_le_bytes());
        bytes[meta + 4..meta + 6].copy_from_slice(&1u16.to_le_bytes());
        bytes[meta + 6..meta + 8].copy_from_slice(&1u16.to_le_bytes());
        let version = b"v4.0.30319\0";
        bytes[meta + 12..meta + 16].copy_from_slice(&(version.len() as u32).to_le_bytes());
        bytes[meta + 16..meta + 16 + version.len()].copy_from_slice(version);
        let after_version = (meta + 16 + version.len() + 3) & !3;
        bytes[after_version + 2..after_version + 4]
            .copy_from_slice(&(streams.len() as u16).to_le_bytes());
        let mut header = after_version + 4;
        let mut data = TEST_STREAM_BASE;
        for (name, payload) in streams {
            assert!(header - meta < TEST_STREAM_BASE, "stream header overlap");
            bytes[header..header + 4].copy_from_slice(&(data as u32).to_le_bytes());
            bytes[header + 4..header + 8].copy_from_slice(&(payload.len() as u32).to_le_bytes());
            let name_bytes = name.as_bytes();
            bytes[header + 8..header + 8 + name_bytes.len()].copy_from_slice(name_bytes);
            let name_len = (name_bytes.len() + 1 + 3) & !3;
            header += 8 + name_len;
            let end = meta + data + payload.len();
            assert!(end <= bytes.len(), "stream data out of file");
            bytes[meta + data..end].copy_from_slice(payload);
            data = (data + payload.len() + 3) & !3;
        }
        bytes
    }

    /// 构造一个最小 PE + CLI 头 + 元数据根（一个空的 `#~` 流）。
    fn synthetic_dotnet() -> Vec<u8> {
        let table = empty_table_stream();
        synthetic_dotnet_with_streams(&[("#~", table.as_slice())])
    }

    #[test]
    fn detects_clr_metadata() {
        let report = detect(&synthetic_dotnet()).expect("detect");
        assert!(report.is_dotnet);
        assert_eq!(report.runtime_version, "2.5");
        assert_eq!(report.metadata_version, "v4.0.30319");
        assert_eq!(report.streams, vec!["#~".to_string()]);
        assert_eq!(report.entry_point_token, 0x0600_0001);
    }

    #[test]
    fn empty_tilde_stream_yields_empty_tables() {
        let report = detect(&synthetic_dotnet()).expect("detect");
        assert!(report.type_defs.is_empty());
        assert!(report.method_defs.is_empty());
        assert!(report.type_refs.is_empty());
        assert!(report.assembly_refs.is_empty());
        assert!(report
            .limitations
            .iter()
            .any(|limitation| limitation.contains("method signatures/blobs")));

        let json = serde_json::to_value(&report).expect("serialize report");
        assert_eq!(json["type_defs"], serde_json::json!([]));
        assert_eq!(json["method_defs"], serde_json::json!([]));
        assert_eq!(json["type_refs"], serde_json::json!([]));
        assert_eq!(json["assembly_refs"], serde_json::json!([]));
    }

    #[test]
    fn enumerates_core_metadata_tables() {
        let (strings, offsets) = string_heap(&[
            "MyType",
            "My.Namespace",
            "Run",
            "Abstract",
            "Object",
            "System",
            "mscorlib",
        ]);

        let mut body = Vec::new();
        // TypeRef (0x01)：ResolutionScope=Module，TypeName=Object，TypeNamespace=System。
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&(offsets[4] as u16).to_le_bytes());
        body.extend_from_slice(&(offsets[5] as u16).to_le_bytes());
        // TypeDef (0x02)。
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&(offsets[0] as u16).to_le_bytes());
        body.extend_from_slice(&(offsets[1] as u16).to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes()); // Extends
        body.extend_from_slice(&1u16.to_le_bytes()); // FieldList
        body.extend_from_slice(&1u16.to_le_bytes()); // MethodList

        // MethodDef (0x06)：Run 有 RVA，Abstract 的 RVA 为 0。
        for (rva, name_index) in [(0x1234u32, offsets[2]), (0, offsets[3])] {
            body.extend_from_slice(&rva.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes()); // ImplFlags
            body.extend_from_slice(&0u16.to_le_bytes()); // Flags
            body.extend_from_slice(&(name_index as u16).to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes()); // Signature
            body.extend_from_slice(&1u16.to_le_bytes()); // ParamList
        }
        // AssemblyRef (0x23)。
        for value in [1u16, 2, 3, 4] {
            body.extend_from_slice(&value.to_le_bytes());
        }
        body.extend_from_slice(&0u32.to_le_bytes()); // Flags
        body.extend_from_slice(&0u16.to_le_bytes()); // PublicKeyOrToken
        body.extend_from_slice(&(offsets[6] as u16).to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes()); // Culture
        body.extend_from_slice(&0u16.to_le_bytes()); // HashValue

        let stream = build_table_stream(
            0,
            &[
                (table::TYPE_REF, 1),
                (table::TYPE_DEF, 1),
                (table::METHOD_DEF, 2),
                (table::ASSEMBLY_REF, 1),
            ],
            &body,
        );
        let pe = synthetic_dotnet_with_streams(&[
            ("#~", stream.as_slice()),
            ("#Strings", strings.as_slice()),
        ]);
        let report = detect(&pe).expect("detect");
        assert_eq!(report.streams, vec!["#~", "#Strings"]);
        assert_eq!(
            report.type_defs,
            vec![TypeDefEntry {
                namespace: "My.Namespace".to_string(),
                name: "MyType".to_string(),
            }]
        );
        assert_eq!(
            report.method_defs,
            vec![
                MethodDefEntry {
                    name: "Run".to_string(),
                    rva: Some(0x1234),
                },
                MethodDefEntry {
                    name: "Abstract".to_string(),
                    rva: None,
                },
            ]
        );
        assert_eq!(
            report.type_refs,
            vec![TypeRefEntry {
                namespace: "System".to_string(),
                name: "Object".to_string(),
            }]
        );
        assert_eq!(
            report.assembly_refs,
            vec![AssemblyRefEntry {
                name: "mscorlib".to_string(),
            }]
        );

        let json = serde_json::to_value(&report).expect("serialize report");
        assert_eq!(json["type_defs"][0]["namespace"], "My.Namespace");
        assert_eq!(json["method_defs"][0]["rva"], 0x1234);
        assert!(json["method_defs"][1].get("rva").is_none());
        assert_eq!(json["assembly_refs"][0]["name"], "mscorlib");
    }

    #[test]
    fn rejects_unknown_table_and_truncated_table_data() {
        let mut unknown = empty_table_stream();
        unknown[8..16].copy_from_slice(&(1u64 << 45).to_le_bytes());
        let pe = synthetic_dotnet_with_streams(&[("#~", unknown.as_slice())]);
        match detect(&pe) {
            Err(DotnetError::BadMetadata(message)) => {
                assert!(message.contains("unknown metadata table"), "{message}");
            }
            other => panic!("expected unknown-table error, got {other:?}"),
        }

        let truncated = build_table_stream(0, &[(table::TYPE_DEF, 1)], &[]);
        let pe = synthetic_dotnet_with_streams(&[("#~", truncated.as_slice())]);
        assert!(matches!(detect(&pe), Err(DotnetError::Truncated)));
    }

    #[test]
    fn out_of_range_string_index_is_an_error() {
        let mut body = Vec::new();
        body.extend_from_slice(&0u32.to_le_bytes()); // Flags
        body.extend_from_slice(&0xFFFFu16.to_le_bytes()); // TypeName
        body.extend_from_slice(&0u16.to_le_bytes()); // TypeNamespace
        body.extend_from_slice(&[0u8; 6]); // Extends / FieldList / MethodList
        let stream = build_table_stream(0, &[(table::TYPE_DEF, 1)], &body);
        let pe = synthetic_dotnet_with_streams(&[("#~", stream.as_slice())]);
        match detect(&pe) {
            Err(DotnetError::BadMetadata(message)) => {
                assert!(message.contains("string heap index"), "{message}");
            }
            other => panic!("expected string-heap error, got {other:?}"),
        }
    }

    #[test]
    fn computes_table_index_widths() {
        assert_eq!(heap_width(0x00, 0x01), 2);
        assert_eq!(heap_width(0x01, 0x01), 4);
        assert_eq!(heap_width(0x02, 0x02), 4);
        assert_eq!(heap_width(0x04, 0x04), 4);

        let mut row_counts = [0u32; TABLE_COUNT];
        row_counts[table::FIELD] = 0xFFFF;
        assert_eq!(simple_width(table::FIELD, &row_counts), 2);
        row_counts[table::FIELD] = 0x1_0000;
        assert_eq!(simple_width(table::FIELD, &row_counts), 4);

        let mut row_counts = [0u32; TABLE_COUNT];
        row_counts[table::TYPE_REF] = 0x3FFF;
        assert_eq!(coded_width(CodedIndex::TypeDefOrRef, &row_counts), 2);
        row_counts[table::TYPE_REF] = 0x4000;
        assert_eq!(coded_width(CodedIndex::TypeDefOrRef, &row_counts), 4);

        let mut row_counts = [0u32; TABLE_COUNT];
        row_counts[table::ASSEMBLY_REF] = 0x4000;
        assert_eq!(coded_width(CodedIndex::ResolutionScope, &row_counts), 4);

        let mut row_counts = [0u32; TABLE_COUNT];
        row_counts[table::TYPE_REF] = 1;
        let columns = table_columns(table::TYPE_DEF).expect("TypeDef columns");
        assert_eq!(row_width(columns, 0x07, &row_counts), 18);

        let mut row_counts = [0u32; TABLE_COUNT];
        row_counts[table::PARAM] = 0x1_0000;
        let columns = table_columns(table::METHOD_DEF).expect("MethodDef columns");
        assert_eq!(row_width(columns, 0x07, &row_counts), 20);
    }

    #[test]
    fn reads_four_byte_string_heap_indexes() {
        let (strings, offsets) = string_heap(&["LargeHeapType", "Large.Heap.Namespace"]);
        let mut body = Vec::new();
        body.extend_from_slice(&0u32.to_le_bytes()); // Flags
        body.extend_from_slice(&offsets[0].to_le_bytes()); // TypeName
        body.extend_from_slice(&offsets[1].to_le_bytes()); // TypeNamespace
        body.extend_from_slice(&0u16.to_le_bytes()); // Extends
        body.extend_from_slice(&1u16.to_le_bytes()); // FieldList
        body.extend_from_slice(&1u16.to_le_bytes()); // MethodList
        let stream = build_table_stream(0x07, &[(table::TYPE_DEF, 1)], &body);
        let pe = synthetic_dotnet_with_streams(&[
            ("#~", stream.as_slice()),
            ("#Strings", strings.as_slice()),
        ]);
        let report = detect(&pe).expect("detect");
        assert_eq!(
            report.type_defs,
            vec![TypeDefEntry {
                namespace: "Large.Heap.Namespace".to_string(),
                name: "LargeHeapType".to_string(),
            }]
        );
    }
}
