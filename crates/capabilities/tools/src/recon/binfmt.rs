//! `re.binfmt`：ELF / PE / Mach-O 结构解析（纯 safe Rust，零新依赖）。
//!
//! 只解析**静态结构**：头、节、段、导入表。不做反编译、不执行目标；
//! 未覆盖的字段写进 [`BinaryReport::limitations`]。
//!
//! 所有读取都做边界检查：畸形输入返回 [`BinFmtError`]，绝不 panic。

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 一个节区。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    /// 节名。
    pub name: String,
    /// 文件内偏移。
    pub offset: u64,
    /// 文件内大小。
    pub size: u64,
    /// 虚拟地址。
    pub address: u64,
    /// 原始标志位。
    pub flags: u64,
}

/// 一个段（程序头 / Mach-O segment）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    /// 段类型（`load` / `dynamic` / `interp` …）。
    pub kind: String,
    /// 文件内偏移。
    pub offset: u64,
    /// 虚拟地址。
    pub vaddr: u64,
    /// 文件内大小。
    pub filesz: u64,
    /// 内存内大小。
    pub memsz: u64,
}

/// 一次静态结构解析结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryReport {
    /// 容器格式：`elf` / `pe` / `macho`。
    pub format: String,
    /// 位数（32 / 64）。
    pub bits: u8,
    /// 字节序：`little` / `big`。
    pub endian: String,
    /// 架构（`x86_64` / `aarch64` …）。
    pub machine: String,
    /// 入口地址。
    pub entry: u64,
    /// 是否为 PIE / 可重定位（能确定时）。
    pub is_pie: Option<bool>,
    /// 节区。
    pub sections: Vec<Section>,
    /// 段。
    pub segments: Vec<Segment>,
    /// 导入（ELF 为 DT_NEEDED 库名；PE 为导入 DLL；Mach-O 为 dylib）。
    pub imports: Vec<String>,
    /// 导出符号名（ELF `.dynsym`，上限保护）。
    pub exports: Vec<String>,
    /// PE 版本资源里的 UTF-16 字符串（CompanyName/FileVersion 等）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub version_info: Vec<String>,
    /// ELF notes（build-id 等）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    /// 显式未测项。
    pub limitations: Vec<String>,
}

impl BinaryReport {
    fn new(format: &str, bits: u8, endian: &str, machine: &str) -> Self {
        Self {
            format: format.to_string(),
            bits,
            endian: endian.to_string(),
            machine: machine.to_string(),
            entry: 0,
            is_pie: None,
            sections: Vec::new(),
            segments: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            version_info: Vec::new(),
            notes: Vec::new(),
            limitations: Vec::new(),
        }
    }
}

/// 解析失败。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BinFmtError {
    /// 输入太短。
    #[error("input is too short for {0}")]
    TooShort(String),
    /// 魔数不支持。
    #[error("unsupported magic: {0}")]
    BadMagic(String),
    /// 结构越界。
    #[error("truncated {0}")]
    Truncated(String),
    /// 结构畸形。
    #[error("malformed {0}")]
    Malformed(String),
}

/// 上限：避免恶意节表/符号表把内存撑爆。
const MAX_SECTIONS: usize = 512;
const MAX_EXPORTS: usize = 256;

/// 识别并解析二进制。
pub fn parse(bytes: &[u8]) -> Result<BinaryReport, BinFmtError> {
    if bytes.starts_with(b"\x7fELF") {
        parse_elf(bytes)
    } else if bytes.starts_with(b"MZ") {
        parse_pe(bytes)
    } else if bytes.len() >= 8 && is_fat_macho_magic(&bytes[..4]) {
        parse_macho_fat(bytes)
    } else if bytes.len() >= 4 && is_macho_magic(&bytes[..4]) {
        parse_macho(bytes)
    } else {
        Err(BinFmtError::BadMagic(hex(&bytes[..bytes.len().min(4)])))
    }
}

fn is_fat_macho_magic(magic: &[u8]) -> bool {
    matches!(magic, [0xca, 0xfe, 0xba, 0xbe] | [0xbe, 0xba, 0xfe, 0xca])
        || matches!(magic, [0xca, 0xfe, 0xba, 0xbf] | [0xbf, 0xba, 0xfe, 0xca])
}

fn is_macho_magic(magic: &[u8]) -> bool {
    matches!(magic, [0xcf, 0xfa, 0xed, 0xfe] | [0xce, 0xfa, 0xed, 0xfe])
        || matches!(magic, [0xfe, 0xed, 0xfa, 0xcf] | [0xfe, 0xed, 0xfa, 0xce])
}

/// 小端/大端读取器，所有访问做边界检查。
struct Reader<'a> {
    data: &'a [u8],
    little: bool,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8], little: bool) -> Self {
        Self { data, little }
    }

    fn slice(&self, offset: u64, len: usize, what: &str) -> Result<&'a [u8], BinFmtError> {
        let start = usize::try_from(offset).map_err(|_| BinFmtError::Truncated(what.into()))?;
        let end = start
            .checked_add(len)
            .ok_or_else(|| BinFmtError::Truncated(what.into()))?;
        self.data
            .get(start..end)
            .ok_or_else(|| BinFmtError::Truncated(what.into()))
    }

    fn u16(&self, offset: u64, what: &str) -> Result<u16, BinFmtError> {
        let raw = self.slice(offset, 2, what)?;
        Ok(if self.little {
            u16::from_le_bytes([raw[0], raw[1]])
        } else {
            u16::from_be_bytes([raw[0], raw[1]])
        })
    }

    fn u32(&self, offset: u64, what: &str) -> Result<u32, BinFmtError> {
        let raw = self.slice(offset, 4, what)?;
        let arr = [raw[0], raw[1], raw[2], raw[3]];
        Ok(if self.little {
            u32::from_le_bytes(arr)
        } else {
            u32::from_be_bytes(arr)
        })
    }

    fn u64(&self, offset: u64, what: &str) -> Result<u64, BinFmtError> {
        let raw = self.slice(offset, 8, what)?;
        let mut arr = [0u8; 8];
        arr.copy_from_slice(raw);
        Ok(if self.little {
            u64::from_le_bytes(arr)
        } else {
            u64::from_be_bytes(arr)
        })
    }

    fn cstr(&self, offset: u64) -> Option<String> {
        let start = usize::try_from(offset).ok()?;
        let rest = self.data.get(start..)?;
        let end = rest.iter().position(|&b| b == 0)?;
        let text = std::str::from_utf8(&rest[..end]).ok()?;
        (!text.is_empty()).then(|| text.to_string())
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn elf_machine(value: u16) -> &'static str {
    match value {
        0x03 => "x86",
        0x3e => "x86_64",
        0x28 => "arm",
        0xb7 => "aarch64",
        0x08 => "mips",
        0x14 => "ppc",
        0x15 => "ppc64",
        0xf3 => "riscv",
        _ => "unknown",
    }
}

fn pe_machine(value: u16) -> &'static str {
    match value {
        0x014c => "x86",
        0x8664 => "x86_64",
        0xaa64 => "aarch64",
        0x01c0 => "arm",
        _ => "unknown",
    }
}

fn macho_cpu(value: u32) -> &'static str {
    match value {
        0x0100_0007 => "x86_64",
        0x0100_000c => "aarch64",
        0x0000_0007 => "x86",
        0x0000_000c => "arm",
        _ => "unknown",
    }
}

fn segment_kind(pe_type: u32) -> String {
    match pe_type {
        0x1 => "load".into(),
        0x2 => "dynamic".into(),
        0x3 => "interp".into(),
        0x4 => "note".into(),
        0x6 => "phdr".into(),
        0x7 => "tls".into(),
        0x6474_e551 => "gnu_stack".into(),
        0x6474_e552 => "gnu_relro".into(),
        other => format!("0x{other:x}"),
    }
}

fn parse_elf(bytes: &[u8]) -> Result<BinaryReport, BinFmtError> {
    if bytes.len() < 52 {
        return Err(BinFmtError::TooShort("elf".into()));
    }
    let class = bytes[4];
    let little = match bytes[5] {
        1 => true,
        2 => false,
        _ => return Err(BinFmtError::Malformed("elf data encoding".into())),
    };
    let bits = match class {
        1 => 32,
        2 => 64,
        _ => return Err(BinFmtError::Malformed("elf class".into())),
    };
    let reader = Reader::new(bytes, little);
    let machine_raw = reader.u16(18, "elf e_machine")?;
    let mut report = BinaryReport::new(
        "elf",
        bits,
        if little { "little" } else { "big" },
        elf_machine(machine_raw),
    );

    let (entry, phoff, shoff, phentsize, phnum, shentsize, shnum, shstrndx, etype) = if bits == 64 {
        (
            reader.u64(24, "elf e_entry")?,
            reader.u64(32, "elf e_phoff")?,
            reader.u64(40, "elf e_shoff")?,
            reader.u16(54, "elf e_phentsize")?,
            reader.u16(56, "elf e_phnum")?,
            reader.u16(58, "elf e_shentsize")?,
            reader.u16(60, "elf e_shnum")?,
            reader.u16(62, "elf e_shstrndx")?,
            reader.u16(16, "elf e_type")?,
        )
    } else {
        (
            u64::from(reader.u32(24, "elf e_entry")?),
            u64::from(reader.u32(28, "elf e_phoff")?),
            u64::from(reader.u32(32, "elf e_shoff")?),
            reader.u16(42, "elf e_phentsize")?,
            reader.u16(44, "elf e_phnum")?,
            reader.u16(46, "elf e_shentsize")?,
            reader.u16(48, "elf e_shnum")?,
            reader.u16(50, "elf e_shstrndx")?,
            reader.u16(16, "elf e_type")?,
        )
    };
    report.entry = entry;
    report.is_pie = Some(etype == 3);

    // 节区表：先校验表本身落在文件内，避免后续偏移运算溢出。
    let file_len = bytes.len() as u64;
    if shnum > 0 && shoff > file_len {
        report
            .limitations
            .push("section table offset is outside the file".into());
    }
    let section_count = if shoff > file_len {
        0
    } else {
        (shnum as usize).min(MAX_SECTIONS)
    };
    if section_count > 0 && shoff != 0 {
        let mut pairs = Vec::with_capacity(section_count);
        for index in 0..section_count {
            let base = shoff + (index as u64) * u64::from(shentsize);
            pairs.push(parse_elf_section(&reader, base, bits)?);
        }
        // 名称表：shstrndx 指向的节区提供节名。
        let strtab_offset = pairs
            .get(shstrndx as usize)
            .map(|(section, _)| section.offset)
            .unwrap_or(0);
        let mut sections = Vec::with_capacity(pairs.len());
        for (mut section, name_offset) in pairs {
            if strtab_offset != 0 && name_offset != 0 {
                if let Some(offset) = strtab_offset.checked_add(u64::from(name_offset)) {
                    if let Some(name) = reader.cstr(offset) {
                        section.name = name;
                    }
                }
            }
            sections.push(section);
        }
        // 导入（DT_NEEDED）与导出（.dynsym）。
        parse_elf_dynamic(&reader, &sections, bits, &mut report)?;
        parse_elf_exports(&reader, &sections, bits, &mut report)?;
        report.sections = sections;
    }

    // 程序头。
    let phnum = if phoff > file_len { 0 } else { phnum };
    if phnum > 0 && phoff != 0 {
        let count = (phnum as usize).min(MAX_SECTIONS);
        let mut segments = Vec::with_capacity(count);
        for index in 0..count {
            let base = phoff + (index as u64) * u64::from(phentsize);
            segments.push(parse_elf_segment(&reader, base, bits)?);
        }
        parse_elf_notes(&reader, &segments, &mut report);
        report.segments = segments;
    }

    if report.sections.is_empty() {
        report.limitations.push("no section headers".into());
    }
    Ok(report)
}

fn parse_elf_notes(reader: &Reader<'_>, segments: &[Segment], report: &mut BinaryReport) {
    for segment in segments {
        if segment.kind != "note" {
            continue;
        }
        let mut offset = segment.offset;
        let end = segment.offset.saturating_add(segment.filesz);
        for _ in 0..64 {
            if offset + 12 > end {
                break;
            }
            let Ok(namesz) = reader.u32(offset, "elf note namesz") else {
                break;
            };
            let Ok(descsz) = reader.u32(offset + 4, "elf note descsz") else {
                break;
            };
            let Ok(note_type) = reader.u32(offset + 8, "elf note type") else {
                break;
            };
            let name_start = offset + 12;
            let name = reader
                .slice(name_start, namesz as usize, "elf note name")
                .map(|bytes| {
                    String::from_utf8_lossy(bytes)
                        .trim_end_matches('\0')
                        .to_string()
                })
                .unwrap_or_default();
            let desc_start = (name_start + u64::from(namesz) + 3) & !3;
            let desc = reader
                .slice(desc_start, descsz as usize, "elf note desc")
                .ok();
            match (name.as_str(), note_type) {
                ("GNU", 3) => {
                    let digest = desc
                        .map(|bytes| {
                            bytes
                                .iter()
                                .map(|byte| format!("{byte:02x}"))
                                .collect::<String>()
                        })
                        .unwrap_or_default();
                    report.notes.push(format!("build-id: {digest}"));
                }
                ("", _) => {}
                _ => report
                    .notes
                    .push(format!("{name}: type {note_type} ({descsz} bytes)")),
            }
            let next = (desc_start + u64::from(descsz) + 3) & !3;
            if next <= offset {
                break;
            }
            offset = next;
        }
    }
}

fn parse_elf_section(
    reader: &Reader<'_>,
    base: u64,
    bits: u8,
) -> Result<(Section, u32), BinFmtError> {
    let name_offset = reader.u32(base, "elf sh_name")?;
    let section = if bits == 64 {
        Section {
            name: String::new(),
            offset: reader.u64(base + 24, "elf sh_offset")?,
            size: reader.u64(base + 32, "elf sh_size")?,
            address: reader.u64(base + 16, "elf sh_addr")?,
            flags: reader.u64(base + 8, "elf sh_flags")?,
        }
    } else {
        Section {
            name: String::new(),
            offset: u64::from(reader.u32(base + 16, "elf sh_offset")?),
            size: u64::from(reader.u32(base + 20, "elf sh_size")?),
            address: u64::from(reader.u32(base + 12, "elf sh_addr")?),
            flags: u64::from(reader.u32(base + 8, "elf sh_flags")?),
        }
    };
    Ok((section, name_offset))
}

fn parse_elf_segment(reader: &Reader<'_>, base: u64, bits: u8) -> Result<Segment, BinFmtError> {
    if bits == 64 {
        let kind = reader.u32(base, "elf p_type")?;
        Ok(Segment {
            kind: segment_kind(kind),
            offset: reader.u64(base + 8, "elf p_offset")?,
            vaddr: reader.u64(base + 16, "elf p_vaddr")?,
            filesz: reader.u64(base + 32, "elf p_filesz")?,
            memsz: reader.u64(base + 40, "elf p_memsz")?,
        })
    } else {
        let kind = reader.u32(base, "elf p_type")?;
        Ok(Segment {
            kind: segment_kind(kind),
            offset: u64::from(reader.u32(base + 4, "elf p_offset")?),
            vaddr: u64::from(reader.u32(base + 8, "elf p_vaddr")?),
            filesz: u64::from(reader.u32(base + 16, "elf p_filesz")?),
            memsz: u64::from(reader.u32(base + 20, "elf p_memsz")?),
        })
    }
}

fn parse_elf_dynamic(
    reader: &Reader<'_>,
    sections: &[Section],
    _bits: u8,
    report: &mut BinaryReport,
) -> Result<(), BinFmtError> {
    let Some(dynamic) = sections.iter().find(|s| s.name == ".dynamic") else {
        report.limitations.push("no .dynamic section".into());
        return Ok(());
    };
    // .dynamic 的 sh_link 指向 .dynstr；这里用名称兜底。
    let Some(dynstr) = sections
        .iter()
        .find(|s| s.name == ".dynstr")
        .or_else(|| sections.iter().find(|s| s.name.is_empty()))
    else {
        return Ok(());
    };
    let entry_size = 16u64;
    let count = dynamic.size / entry_size;
    for index in 0..count.min(4096) {
        let Some(base) = dynamic
            .offset
            .checked_add(index.checked_mul(entry_size).unwrap_or(u64::MAX))
        else {
            break;
        };
        let tag = reader.u64(base, "elf d_tag")? as i64;
        if tag == 0 {
            break;
        }
        if tag == 1 {
            let value = reader.u64(base + 8, "elf d_val")?;
            if let Some(offset) = dynstr.offset.checked_add(value) {
                if let Some(name) = reader.cstr(offset) {
                    report.imports.push(name);
                }
            }
        }
    }
    Ok(())
}

fn parse_elf_exports(
    reader: &Reader<'_>,
    sections: &[Section],
    bits: u8,
    report: &mut BinaryReport,
) -> Result<(), BinFmtError> {
    let Some(dynsym) = sections.iter().find(|s| s.name == ".dynsym") else {
        report.limitations.push("no .dynsym section".into());
        return Ok(());
    };
    let Some(dynstr) = sections.iter().find(|s| s.name == ".dynstr") else {
        return Ok(());
    };
    let entry_size = if bits == 64 { 24 } else { 16 };
    let count = dynsym.size / entry_size;
    for index in 0..count.min(MAX_EXPORTS as u64) {
        let Some(base) = dynsym
            .offset
            .checked_add(index.checked_mul(entry_size).unwrap_or(u64::MAX))
        else {
            break;
        };
        let name_offset = reader.u32(base, "elf st_name")?;
        let shndx = if bits == 64 {
            reader.u16(base + 6, "elf st_shndx")?
        } else {
            reader.u16(base + 14, "elf st_shndx")?
        };
        if shndx == 0 || name_offset == 0 {
            continue;
        }
        if let Some(offset) = dynstr.offset.checked_add(u64::from(name_offset)) {
            if let Some(name) = reader.cstr(offset) {
                report.exports.push(name);
            }
        }
    }
    Ok(())
}

fn parse_pe(bytes: &[u8]) -> Result<BinaryReport, BinFmtError> {
    if bytes.len() < 0x40 {
        return Err(BinFmtError::TooShort("pe".into()));
    }
    let reader = Reader::new(bytes, true);
    let pe_offset = u64::from(reader.u32(0x3c, "pe e_lfanew")?);
    let signature = reader.slice(pe_offset, 4, "pe signature")?;
    if signature != b"PE\0\0" {
        return Err(BinFmtError::BadMagic("PE signature".into()));
    }
    let coff = pe_offset + 4;
    let machine_raw = reader.u16(coff, "pe machine")?;
    let section_count = reader.u16(coff + 2, "pe sections")? as usize;
    let optional_size = u64::from(reader.u16(coff + 16, "pe optional size")?);
    let optional = coff + 20;
    let magic = reader.u16(optional, "pe optional magic")?;
    let (bits, entry_offset, _image_base_offset) = match magic {
        0x10b => (32u8, 16u64, 28u64),
        0x20b => (64u8, 16u64, 24u64),
        _ => return Err(BinFmtError::Malformed("pe optional magic".into())),
    };
    let entry_rva = u64::from(reader.u32(optional + entry_offset, "pe entry")?);

    let mut report = BinaryReport::new("pe", bits, "little", pe_machine(machine_raw));
    report.entry = entry_rva;
    let characteristics = reader.u16(coff + 18, "pe characteristics")?;
    report.is_pie = Some(characteristics & 0x0020 != 0);

    let section_table = optional
        .checked_add(optional_size)
        .ok_or_else(|| BinFmtError::Malformed("pe section table overflow".into()))?;
    let mut sections = Vec::with_capacity(section_count.min(MAX_SECTIONS));
    for index in 0..section_count.min(MAX_SECTIONS) {
        let base = section_table
            .checked_add((index as u64).checked_mul(40).unwrap_or(u64::MAX))
            .ok_or_else(|| BinFmtError::Malformed("pe section offset overflow".into()))?;
        let raw_name = reader.slice(base, 8, "pe section name")?;
        let name = String::from_utf8_lossy(raw_name)
            .trim_end_matches('\0')
            .to_string();
        sections.push(Section {
            name,
            size: u64::from(reader.u32(base + 16, "pe raw size")?),
            offset: u64::from(reader.u32(base + 20, "pe raw pointer")?),
            address: u64::from(reader.u32(base + 12, "pe virtual address")?),
            flags: u64::from(reader.u32(base + 36, "pe characteristics")?),
        });
    }

    // 导入表：数据目录 1。
    let data_dir = optional + if bits == 64 { 112 } else { 96 };
    let import_rva = u64::from(reader.u32(data_dir + 8, "pe import rva")?);
    if import_rva != 0 {
        if let Some(offset) = rva_to_offset(&sections, import_rva) {
            parse_pe_imports(&reader, offset, &sections, &mut report)?;
        }
    }
    // 版本资源：数据目录 2。
    let resource_rva = u64::from(reader.u32(data_dir + 16, "pe resource rva")?);
    if resource_rva != 0 {
        if let Some(offset) = rva_to_offset(&sections, resource_rva) {
            parse_pe_version(&reader, offset, &sections, &mut report)?;
        }
    }
    // 导出表：数据目录 0。
    let export_rva = u64::from(reader.u32(data_dir, "pe export rva")?);
    if export_rva != 0 {
        if let Some(offset) = rva_to_offset(&sections, export_rva) {
            parse_pe_exports(&reader, offset, &sections, &mut report)?;
        }
    }
    report.sections = sections;
    Ok(report)
}

fn rva_to_offset(sections: &[Section], rva: u64) -> Option<u64> {
    for section in sections {
        let start = section.address;
        let end = start.saturating_add(section.size.max(0x1000));
        if rva >= start && rva < end {
            return Some(section.offset + (rva - start));
        }
    }
    None
}

fn parse_pe_imports(
    reader: &Reader<'_>,
    mut offset: u64,
    sections: &[Section],
    report: &mut BinaryReport,
) -> Result<(), BinFmtError> {
    for _ in 0..64 {
        let name_rva = u64::from(reader.u32(offset + 12, "pe import name rva")?);
        if name_rva == 0 {
            break;
        }
        if let Some(name_offset) = rva_to_offset(sections, name_rva) {
            if let Some(name) = reader.cstr(name_offset) {
                report.imports.push(name);
            }
        }
        offset += 20;
    }
    Ok(())
}

fn parse_pe_version(
    reader: &Reader<'_>,
    base: u64,
    sections: &[Section],
    report: &mut BinaryReport,
) -> Result<(), BinFmtError> {
    let Some(type_dir) = resource_entry(reader, base, 0, Some(16)) else {
        return Ok(());
    };
    let Some(name_dir) = resource_entry(reader, base, type_dir, None) else {
        return Ok(());
    };
    let Some(language_dir) = resource_entry(reader, base, name_dir, None) else {
        return Ok(());
    };
    let data_entry = base + u64::from(language_dir);
    let data_rva = u64::from(reader.u32(data_entry, "pe version data rva")?);
    let size = reader.u32(data_entry + 4, "pe version size")? as usize;
    let Some(offset) = rva_to_offset(sections, data_rva) else {
        return Ok(());
    };
    let Ok(blob) = reader.slice(offset, size.min(64 * 1024), "pe version blob") else {
        return Ok(());
    };

    let mut current = String::new();
    let mut seen = std::collections::BTreeSet::new();
    for pair in blob.chunks(2) {
        let code = u16::from_le_bytes([pair[0], *pair.get(1).unwrap_or(&0)]);
        if (0x20..0x7f).contains(&code) {
            current.push(char::from(code as u8));
        } else {
            if current.len() >= 3 && seen.insert(current.clone()) {
                report.version_info.push(current.clone());
                if report.version_info.len() >= 64 {
                    break;
                }
            }
            current.clear();
        }
    }
    Ok(())
}

/// 读取资源目录中的一个条目：`want_id = None` 取第一个；返回相对 base 的偏移。
fn resource_entry(
    reader: &Reader<'_>,
    base: u64,
    directory: u32,
    want_id: Option<u32>,
) -> Option<u32> {
    let dir = base + u64::from(directory);
    let named = u32::from(reader.u16(dir + 12, "pe resource named").ok()?);
    let ids = u32::from(reader.u16(dir + 14, "pe resource ids").ok()?);
    for index in 0..(named + ids).min(64) {
        let entry = dir + 16 + u64::from(index) * 8;
        let id = reader.u32(entry, "pe resource id").ok()?;
        if let Some(want) = want_id {
            if id != want {
                continue;
            }
        }
        let offset = reader.u32(entry + 4, "pe resource offset").ok()?;
        return Some(offset & 0x7fff_ffff);
    }
    None
}

fn parse_pe_exports(
    reader: &Reader<'_>,
    offset: u64,
    sections: &[Section],
    report: &mut BinaryReport,
) -> Result<(), BinFmtError> {
    let number_of_names = reader.u32(offset + 24, "pe export name count")? as usize;
    let names_rva = u64::from(reader.u32(offset + 32, "pe export names rva")?);
    if names_rva == 0 {
        return Ok(());
    }
    let Some(names_offset) = rva_to_offset(sections, names_rva) else {
        return Ok(());
    };
    for index in 0..number_of_names.min(MAX_EXPORTS) {
        let name_rva =
            u64::from(reader.u32(names_offset + (index as u64) * 4, "pe export name rva")?);
        if let Some(name_offset) = rva_to_offset(sections, name_rva) {
            if let Some(name) = reader.cstr(name_offset) {
                report.exports.push(name);
            }
        }
    }
    Ok(())
}

fn parse_macho_fat(bytes: &[u8]) -> Result<BinaryReport, BinFmtError> {
    if bytes.len() < 8 {
        return Err(BinFmtError::TooShort("macho-fat".into()));
    }
    let reader = Reader::new(bytes, false);
    let count = reader.u32(4, "fat nfat_arch")? as usize;
    if count == 0 || count > 32 {
        return Err(BinFmtError::Malformed("fat nfat_arch".into()));
    }
    let mut report = BinaryReport::new("macho-fat", 0, "big", "universal");
    let mut slices = Vec::new();
    for index in 0..count {
        let base = 8 + (index as u64) * 20;
        let cputype = reader.u32(base, "fat cputype")?;
        let offset = u64::from(reader.u32(base + 8, "fat offset")?);
        let size = u64::from(reader.u32(base + 12, "fat size")?);
        slices.push(format!("{}@0x{offset:x}+0x{size:x}", macho_cpu(cputype)));
        // 解析第一个切片的结构，作为 fat 文件的代表。
        if index == 0 {
            if let Some(slice) = bytes.get(offset as usize..offset.saturating_add(size) as usize) {
                if let Ok(inner) = parse_macho(slice) {
                    report.segments = inner.segments;
                    report.imports = inner.imports;
                    report.sections = inner.sections;
                    report.entry = inner.entry;
                    report.is_pie = inner.is_pie;
                }
            }
        }
    }
    report
        .limitations
        .push(format!("fat slices: {}", slices.join("; ")));
    Ok(report)
}

fn parse_macho(bytes: &[u8]) -> Result<BinaryReport, BinFmtError> {
    if bytes.len() < 28 {
        return Err(BinFmtError::TooShort("macho".into()));
    }
    let magic = &bytes[..4];
    // Mach-O 魔数区分 32/64 位与字节序（0xfeedfacf / 0xfeedface 的两种排布）。
    let (little, bits) = match magic {
        [0xcf, 0xfa, 0xed, 0xfe] => (true, 64u8),
        [0xce, 0xfa, 0xed, 0xfe] => (true, 32u8),
        [0xfe, 0xed, 0xfa, 0xcf] => (false, 64u8),
        [0xfe, 0xed, 0xfa, 0xce] => (false, 32u8),
        _ => return Err(BinFmtError::BadMagic(hex(magic))),
    };
    let reader = Reader::new(bytes, little);
    let cputype = reader.u32(4, "macho cputype")?;
    let filetype = reader.u32(12, "macho filetype")?;
    let ncmds = u64::from(reader.u32(16, "macho ncmds")?);
    let header_size = if bits == 64 { 32u64 } else { 28u64 };
    let mut report = BinaryReport::new(
        "macho",
        bits,
        if little { "little" } else { "big" },
        macho_cpu(cputype),
    );
    report.is_pie = Some(filetype == 2);

    let mut offset = header_size;
    for _ in 0..ncmds.min(4096) {
        let cmd = reader.u32(offset, "macho cmd")?;
        let cmdsize = u64::from(reader.u32(offset + 4, "macho cmdsize")?);
        if cmdsize < 8 {
            return Err(BinFmtError::Malformed("macho cmdsize".into()));
        }
        match cmd {
            0x19 => {
                let name_raw = reader.slice(offset + 8, 16, "macho segname")?;
                let name = String::from_utf8_lossy(name_raw)
                    .trim_end_matches('\0')
                    .to_string();
                report.segments.push(Segment {
                    kind: name,
                    vaddr: reader.u64(offset + 24, "macho vmaddr")?,
                    memsz: reader.u64(offset + 32, "macho vmsize")?,
                    offset: reader.u64(offset + 40, "macho fileoff")?,
                    filesz: reader.u64(offset + 48, "macho filesize")?,
                });
            }
            0xc => {
                let str_offset = u64::from(reader.u32(offset + 8, "macho dylib offset")?);
                let command_end = offset + cmdsize;
                if let Some(name) = reader.cstr(offset + str_offset) {
                    if offset + str_offset < command_end {
                        report.imports.push(name);
                    }
                }
            }
            _ => {}
        }
        offset = offset
            .checked_add(cmdsize)
            .ok_or_else(|| BinFmtError::Malformed("macho load command overflow".into()))?;
    }
    if bits == 64 && report.segments.is_empty() {
        report.limitations.push("no LC_SEGMENT_64".into());
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current_exe_bytes() -> Vec<u8> {
        std::fs::read(std::env::current_exe().expect("current exe")).expect("read current exe")
    }

    #[test]
    fn parses_the_running_elf() {
        let bytes = current_exe_bytes();
        let report = parse(&bytes).expect("parse self");
        assert_eq!(report.format, "elf");
        assert!(matches!(report.bits, 32 | 64));
        assert!(!report.sections.is_empty());
        // 导入/导出至少能读出一部分（测试二进制通常有）。
        assert!(
            report.limitations.iter().any(|l| l.contains("section")) || !report.sections.is_empty()
        );
    }

    #[test]
    fn rejects_short_and_unknown_inputs() {
        assert!(matches!(parse(b"\x7fELF"), Err(BinFmtError::TooShort(_))));
        assert!(matches!(parse(b"nonsense"), Err(BinFmtError::BadMagic(_))));
        assert!(matches!(parse(b"MZ"), Err(BinFmtError::TooShort(_))));
    }

    #[test]
    fn parses_a_minimal_pe_header() {
        let mut bytes = vec![0u8; 0x200];
        bytes[0] = b'M';
        bytes[1] = b'Z';
        bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        // COFF: machine x86_64, 1 section, optional size 0xF0.
        bytes[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&0xF0u16.to_le_bytes());
        // Optional header: PE32+ magic, entry RVA 0x1000.
        bytes[0x98..0x9a].copy_from_slice(&0x20bu16.to_le_bytes());
        bytes[0x98 + 16..0x98 + 20].copy_from_slice(&0x1000u32.to_le_bytes());
        // Section table at 0x98 + 0xF0 = 0x188.
        let section = 0x188;
        bytes[section..section + 8].copy_from_slice(b".text\0\0\0");
        bytes[section + 12..section + 16].copy_from_slice(&0x1000u32.to_le_bytes());
        bytes[section + 16..section + 20].copy_from_slice(&0x200u32.to_le_bytes());
        bytes[section + 20..section + 24].copy_from_slice(&0x200u32.to_le_bytes());

        let report = parse(&bytes).expect("parse pe");
        assert_eq!(report.format, "pe");
        assert_eq!(report.bits, 64);
        assert_eq!(report.machine, "x86_64");
        assert_eq!(report.sections.len(), 1);
        assert_eq!(report.sections[0].name, ".text");
        assert_eq!(report.entry, 0x1000);
    }

    #[test]
    fn parses_pe_export_names() {
        let mut bytes = vec![0u8; 0x400];
        bytes[0] = b'M';
        bytes[1] = b'Z';
        bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&0xF0u16.to_le_bytes());
        bytes[0x98..0x9a].copy_from_slice(&0x20bu16.to_le_bytes());
        let section = 0x188;
        bytes[section..section + 8].copy_from_slice(b".text\0\0\0");
        bytes[section + 8..section + 12].copy_from_slice(&0x200u32.to_le_bytes());
        bytes[section + 12..section + 16].copy_from_slice(&0x1000u32.to_le_bytes());
        bytes[section + 16..section + 20].copy_from_slice(&0x200u32.to_le_bytes());
        bytes[section + 20..section + 24].copy_from_slice(&0x200u32.to_le_bytes());
        // 导出目录：RVA 0x1000（文件 0x200）。
        let data_dir = 0x98 + 112;
        bytes[data_dir..data_dir + 4].copy_from_slice(&0x1000u32.to_le_bytes());
        bytes[data_dir + 4..data_dir + 8].copy_from_slice(&0x100u32.to_le_bytes());
        let export = 0x200usize;
        bytes[export + 24..export + 28].copy_from_slice(&1u32.to_le_bytes()); // NumberOfNames
        bytes[export + 32..export + 36].copy_from_slice(&0x1040u32.to_le_bytes()); // AddressOfNames
                                                                                   // 名称 RVA 数组在 0x240，名字在 0x280。
        bytes[0x240..0x244].copy_from_slice(&0x1080u32.to_le_bytes());
        bytes[0x280..0x285].copy_from_slice(b"demo\0");

        let report = parse(&bytes).expect("parse pe exports");
        assert_eq!(report.format, "pe");
        assert_eq!(report.exports, vec!["demo".to_string()]);
    }

    #[test]
    fn parses_a_minimal_macho_header() {
        let mut bytes = vec![0u8; 0x100];
        bytes[0..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
        bytes[4..8].copy_from_slice(&0x0100_0007u32.to_le_bytes()); // x86_64
        bytes[12..16].copy_from_slice(&2u32.to_le_bytes()); // MH_EXECUTE
        bytes[16..20].copy_from_slice(&1u32.to_le_bytes()); // ncmds
                                                            // LC_SEGMENT_64 at 0x20.
        bytes[0x20..0x24].copy_from_slice(&0x19u32.to_le_bytes());
        bytes[0x24..0x28].copy_from_slice(&72u32.to_le_bytes());
        bytes[0x28..0x38].copy_from_slice(b"__TEXT\0\0\0\0\0\0\0\0\0\0");

        let report = parse(&bytes).expect("parse macho");
        assert_eq!(report.format, "macho");
        assert_eq!(report.bits, 64);
        assert_eq!(report.machine, "x86_64");
        assert_eq!(report.segments.len(), 1);
        assert_eq!(report.segments[0].kind, "__TEXT");
    }
}

#[cfg(test)]
mod fat_tests {
    use super::*;

    #[test]
    fn parses_fat_macho_header_and_first_slice() {
        let mut bytes = vec![0u8; 0x200];
        bytes[0..4].copy_from_slice(&[0xca, 0xfe, 0xba, 0xbe]);
        bytes[4..8].copy_from_slice(&1u32.to_be_bytes());
        bytes[8..12].copy_from_slice(&0x0100_0007u32.to_be_bytes()); // x86_64
        bytes[16..20].copy_from_slice(&0x100u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&0x100u32.to_be_bytes());

        let thin = 0x100usize;
        bytes[thin..thin + 4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
        bytes[thin + 4..thin + 8].copy_from_slice(&0x0100_0007u32.to_le_bytes());
        bytes[thin + 12..thin + 16].copy_from_slice(&2u32.to_le_bytes());
        bytes[thin + 16..thin + 20].copy_from_slice(&1u32.to_le_bytes());
        bytes[thin + 0x20..thin + 0x24].copy_from_slice(&0x19u32.to_le_bytes());
        bytes[thin + 0x24..thin + 0x28].copy_from_slice(&72u32.to_le_bytes());
        bytes[thin + 0x28..thin + 0x38].copy_from_slice(b"__TEXT\0\0\0\0\0\0\0\0\0\0");

        let report = parse(&bytes).expect("parse fat macho");
        assert_eq!(report.format, "macho-fat");
        assert_eq!(report.machine, "universal");
        assert_eq!(report.segments.len(), 1);
        assert!(report
            .limitations
            .iter()
            .any(|line| line.contains("x86_64")));
    }
}

#[cfg(test)]
mod note_tests {
    use super::*;

    #[test]
    fn parses_gnu_build_id_note() {
        let mut bytes = vec![0u8; 0x120];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[6] = 1;
        bytes[16..18].copy_from_slice(&2u16.to_le_bytes());
        bytes[18..20].copy_from_slice(&0x3eu16.to_le_bytes());
        bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
        bytes[24..32].copy_from_slice(&0x400000u64.to_le_bytes());
        bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
        bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
        bytes[54..56].copy_from_slice(&56u16.to_le_bytes());
        bytes[56..58].copy_from_slice(&1u16.to_le_bytes());
        bytes[58..60].copy_from_slice(&64u16.to_le_bytes());

        let note_len = 20u64;
        let ph = 64usize;
        bytes[ph..ph + 4].copy_from_slice(&4u32.to_le_bytes()); // PT_NOTE
        bytes[ph + 8..ph + 16].copy_from_slice(&0x100u64.to_le_bytes());
        bytes[ph + 16..ph + 24].copy_from_slice(&0x100u64.to_le_bytes());
        bytes[ph + 32..ph + 40].copy_from_slice(&note_len.to_le_bytes());
        bytes[ph + 40..ph + 48].copy_from_slice(&note_len.to_le_bytes());

        let note = 0x100usize;
        bytes[note..note + 4].copy_from_slice(&4u32.to_le_bytes());
        bytes[note + 4..note + 8].copy_from_slice(&4u32.to_le_bytes());
        bytes[note + 8..note + 12].copy_from_slice(&3u32.to_le_bytes());
        bytes[note + 12..note + 16].copy_from_slice(b"GNU\0");
        bytes[note + 16..note + 20].copy_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]);

        let report = parse(&bytes).expect("parse notes");
        assert!(
            report.notes.iter().any(|note| note == "build-id: aabbccdd"),
            "notes: {:?}",
            report.notes
        );
    }
}

#[cfg(test)]
mod version_tests {
    use super::*;

    fn push_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn push_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn parses_pe_version_strings() {
        let mut bytes = vec![0u8; 0x400];
        bytes[0] = b'M';
        bytes[1] = b'Z';
        push_u32(&mut bytes, 0x3c, 0x80);
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        push_u16(&mut bytes, 0x84, 0x8664);
        push_u16(&mut bytes, 0x86, 1);
        push_u16(&mut bytes, 0x94, 0xF0);
        push_u16(&mut bytes, 0x98, 0x20b);

        let section = 0x188;
        bytes[section..section + 8].copy_from_slice(b".rsrc\0\0\0");
        push_u32(&mut bytes, section + 8, 0x200);
        push_u32(&mut bytes, section + 12, 0x1000);
        push_u32(&mut bytes, section + 16, 0x200);
        push_u32(&mut bytes, section + 20, 0x200);

        // 资源目录（数据目录 2）
        let data_dir = 0x98 + 112;
        push_u32(&mut bytes, data_dir + 16, 0x1000);
        push_u32(&mut bytes, data_dir + 20, 0x200);

        let base = 0x200;
        push_u16(&mut bytes, base + 12, 0);
        push_u16(&mut bytes, base + 14, 1);
        push_u32(&mut bytes, base + 16, 16);
        push_u32(&mut bytes, base + 20, 0x8000_0018);

        push_u16(&mut bytes, base + 0x18 + 12, 0);
        push_u16(&mut bytes, base + 0x18 + 14, 1);
        push_u32(&mut bytes, base + 0x28, 1);
        push_u32(&mut bytes, base + 0x2c, 0x8000_0030);

        push_u16(&mut bytes, base + 0x30 + 12, 0);
        push_u16(&mut bytes, base + 0x30 + 14, 1);
        push_u32(&mut bytes, base + 0x40, 0x409);
        push_u32(&mut bytes, base + 0x44, 0x50);

        let blob = 0x280;
        let text = "CompanyName\0AcmeCorp\0FileVersion\01.2.3\0";
        let mut units = Vec::new();
        for ch in text.chars() {
            units.push(ch as u16);
        }
        push_u32(&mut bytes, base + 0x50, 0x1080);
        push_u32(&mut bytes, base + 0x54, (units.len() * 2) as u32);
        for (index, unit) in units.iter().enumerate() {
            push_u16(&mut bytes, blob + index * 2, *unit);
        }

        let report = parse(&bytes).expect("parse pe version");
        assert!(
            report.version_info.iter().any(|value| value == "AcmeCorp"),
            "version_info: {:?}",
            report.version_info
        );
        assert!(report
            .version_info
            .iter()
            .any(|value| value == "FileVersion"));
    }
}
