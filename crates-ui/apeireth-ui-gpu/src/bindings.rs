//! uniform 名 → 索引映射 + uniform 缓冲（std140 块）布局。
//!
//! ## 为什么必须自己维护这张表
//!
//! naga 的 IR 里，每个 GLSL uniform 要么变成**独立 binding**（逐个注入
//! `layout(binding=N)` 的形态），要么在 std140 块里变成**匿名成员**
//! （`layout(std140) uniform` 的形态）—— 后者再吐出时成员名被抹成
//! `_group_0_binding_N_fs`（上游实测，见 ADR-0015 §2.6.2 的第 2 轮探测
//! 记录）。⇒ 引擎侧**只能按索引绑定**，不能按名字绑定；而「哪个名字对应
//! 哪个索引」这条信息，naga 不会还给引擎，必须由引擎自己持有。
//!
//! ## 索引的含义
//!
//! 索引 = **uniform 在块里的成员序** = **源码里的声明序**。
//!
//! - 逐个注入 binding 的形态下，索引 = binding 号；
//! - 合并成 std140 块（生产路径，X3）的形态下，索引 = 块内成员序，
//!   整块占**一个** binding。
//!
//! 两种形态下「顺序 ⇒ 索引」的对应关系**完全一样** ⇒ 映射表不变。
//!
//! ## 表的权威来源
//!
//! 生产版着色器的 uniform 声明顺序（权威来源：生产前端的 uniform 名数组，
//! 顺序即索引 0..N-1）。这张表是**引擎侧常量**，与着色器源一一对应
//! ⇒ 着色器改 uniform 数量时**必须同步改表** ⇒ 由本模块的单测钉住
//! 「表长度 = 着色器 uniform 声明数」与「顺序 ⇒ 索引」。
//!
//! ## 命名中性
//!
//! 表里的标识符是**着色器层的标识符**（不是业务名词）；本模块不出现任何
//! 会话 / 审批 / 状态之类的业务词汇。

use crate::glsl::{
    float_component_count, float_std140_alignment, float_std140_size, GlslError, UniformBlockMember,
};

/// 生产版着色器的 uniform 名序 —— **索引即成员序**。
///
/// 权威来源：生产前端着色器的 uniform 声明顺序（16 条）。
/// 顺序 ⇒ 索引的对应关系由单测钉住（见本模块的
/// `binding_table_matches_shader_declaration_order`）。
pub const SHADER_UNIFORM_NAMES: [&str; 16] = [
    "uRes",
    "uCenter",
    "uScale",
    "uTime",
    "uRot",
    "uBright",
    "uTurb",
    "uPulse",
    "uIncl",
    "uAzim",
    "uDiskAzim",
    "uDiskIncl",
    "uAmbStr",
    "uEclipse",
    "uTlBright",
    "uTlGold",
];

/// uniform 名 → 索引。
///
/// 不在表里 → `None`（**不静默回落到 0**：索引 0 是别人的位置）。
pub fn binding_of(name: &str) -> Option<u32> {
    SHADER_UNIFORM_NAMES
        .iter()
        .position(|candidate| *candidate == name)
        .map(|index| index as u32)
}

/// 索引 → uniform 名。
pub fn name_of(binding: u32) -> Option<&'static str> {
    SHADER_UNIFORM_NAMES.get(binding as usize).copied()
}

/// 索引 → uniform 名的完整视图（顺序 = 索引序）。
pub fn binding_table() -> Vec<(&'static str, u32)> {
    SHADER_UNIFORM_NAMES
        .iter()
        .enumerate()
        .map(|(index, name)| (*name, index as u32))
        .collect()
}

// ---------------------------------------------------------------------------
// 缓冲布局
// ---------------------------------------------------------------------------

/// uniform 缓冲里的一个成员（= 一个 uniform）。
#[derive(Debug, Clone, PartialEq)]
pub struct UniformSlot {
    /// 成员序（= 声明序 = 映射表下标）。
    pub index: usize,
    /// uniform 名（来自着色器源）。
    pub name: String,
    /// 类型名（`float` / `vec2` / `vec3` / `vec4`）。
    pub type_name: String,
    /// 分量数。
    pub components: usize,
    /// 在 uniform 缓冲里的字节偏移（**std140**）。
    pub offset: u64,
    /// 在 uniform 缓冲里的字节大小。
    pub size: u64,
    /// 该成员是不是引擎补齐用的 **padding 成员**（不被着色器引用）。
    pub is_padding: bool,
    /// 该成员是不是**引擎自填**成员（宿主不必提供值；引擎每帧写入）。
    ///
    /// - padding 成员：只为对齐 ⇒ 宿主不提供；
    /// - 帧缓冲高度成员（[`crate::glsl::FRAMEBUFFER_HEIGHT_MEMBER`]）：
    ///   引擎每帧写入当前表面高度 ⇒ 宿主不提供。
    pub is_engine_filled: bool,
}

/// uniform 缓冲布局（一个块 + N 个成员）。
///
/// 布局规则是 **std140**（GLSL 后端与 WGSL uniform 地址空间共用的一套规则）
/// ⇒ 两个后端的字节布局一致。
#[derive(Debug, Clone, PartialEq)]
pub struct UniformBufferLayout {
    binding: u32,
    slots: Vec<UniformSlot>,
    total_size: u64,
}

impl UniformBufferLayout {
    /// 由 uniform 块的成员清单构建布局（std140）。
    ///
    /// 成员清单来自 [`crate::glsl::uniform_block_members`]（含引擎补齐的
    /// padding 成员）⇒ 布局与着色器里的块**逐字节一致**。
    ///
    /// # Errors
    ///
    /// 成员里出现不受支持的类型（整型 / 矩阵 / 数组 …）→
    /// [`GlslError::UnsupportedUniformType`]（**显式拒绝**，不静默按
    /// `float` 处理）。
    pub fn from_members(members: &[UniformBlockMember], binding: u32) -> Result<Self, GlslError> {
        let mut slots = Vec::with_capacity(members.len());
        let mut offset = 0u64;
        for (index, member) in members.iter().enumerate() {
            if !crate::glsl::is_supported_float_type(&member.type_name) {
                return Err(GlslError::UnsupportedUniformType {
                    name: member.name.clone(),
                    type_name: member.type_name.clone(),
                });
            }
            // ⚠ 类型已判定受支持 ⇒ 这两个 `Option` **必然**是 `Some`；
            //   仍显式处理 `None`（不 panic ⇒ 不把调用方的 bug 变成崩溃）。
            let alignment = float_std140_alignment(&member.type_name).ok_or_else(|| {
                GlslError::UnsupportedUniformType {
                    name: member.name.clone(),
                    type_name: member.type_name.clone(),
                }
            })?;
            let size = float_std140_size(&member.type_name).ok_or_else(|| {
                GlslError::UnsupportedUniformType {
                    name: member.name.clone(),
                    type_name: member.type_name.clone(),
                }
            })?;
            let components = float_component_count(&member.type_name).ok_or_else(|| {
                GlslError::UnsupportedUniformType {
                    name: member.name.clone(),
                    type_name: member.type_name.clone(),
                }
            })?;
            offset = align_up(offset, alignment);
            slots.push(UniformSlot {
                index,
                name: member.name.clone(),
                type_name: member.type_name.clone(),
                components,
                offset,
                size,
                is_padding: member.is_padding,
                is_engine_filled: member.is_engine_filled,
            });
            offset += size;
        }
        // std140：块大小向上对齐到 16 字节。
        let total_size = align_up(offset, 16);
        Ok(Self {
            binding,
            slots,
            total_size,
        })
    }

    /// uniform 块占用的 binding 号。
    pub fn binding(&self) -> u32 {
        self.binding
    }

    /// 全部成员（按索引序）。
    pub fn slots(&self) -> &[UniformSlot] {
        &self.slots
    }

    /// 缓冲总字节数（std140 块大小，16 字节对齐）。
    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    /// 按索引取成员。
    pub fn slot(&self, index: u32) -> Option<&UniformSlot> {
        self.slots.iter().find(|slot| slot.index == index as usize)
    }

    /// 按名字取成员。
    pub fn slot_by_name(&self, name: &str) -> Option<&UniformSlot> {
        self.slots.iter().find(|slot| slot.name == name)
    }

    /// 成员个数。
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// 是否没有成员。
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

/// 向上对齐到 `alignment` 的倍数。
fn align_up(value: u64, alignment: u64) -> u64 {
    debug_assert!(alignment > 0);
    value.div_ceil(alignment) * alignment
}

// ---------------------------------------------------------------------------
// uniform 值
// ---------------------------------------------------------------------------

/// uniform 绑定错误。
#[derive(Debug, Clone, PartialEq)]
pub enum BindingError {
    /// 名字不在映射表里。
    UnknownUniform(String),
    /// 索引没有对应成员。
    UnknownBinding(u32),
    /// 分量数与声明的类型不符。
    ComponentCountMismatch {
        /// 出问题的索引。
        binding: u32,
        /// 声明要求的分量数。
        expected: usize,
        /// 实际给了多少分量。
        got: usize,
    },
    /// 成员没有赋值。
    MissingValue {
        /// 出问题的索引。
        binding: u32,
        /// 成员名。
        name: String,
    },
    /// 目标缓冲太小。
    BufferTooSmall {
        /// 需要的字节数。
        needed: u64,
        /// 实际拿到的字节数。
        got: u64,
    },
}

impl std::fmt::Display for BindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BindingError::UnknownUniform(name) => {
                write!(f, "UnknownUniform: `{name}` 不在 uniform 映射表里")
            }
            BindingError::UnknownBinding(binding) => {
                write!(f, "UnknownBinding: 索引 {binding} 没有对应成员")
            }
            BindingError::ComponentCountMismatch {
                binding,
                expected,
                got,
            } => write!(
                f,
                "ComponentCountMismatch: 索引 {binding} 需要 {expected} 个分量，实得 {got} 个"
            ),
            BindingError::MissingValue { binding, name } => {
                write!(f, "MissingValue: 索引 {binding}（`{name}`）没有赋值")
            }
            BindingError::BufferTooSmall { needed, got } => {
                write!(f, "BufferTooSmall: 需要 {needed} 字节，实得 {got} 字节")
            }
        }
    }
}

impl std::error::Error for BindingError {}

/// 一组 uniform 值 —— **按索引寻址**。
///
/// naga 会抹掉 uniform 名，所以这层的**主键是索引**；
/// [`UniformValues::set_by_name`] 只是引擎自维护映射表的便利入口，
/// 内部同样落到索引上。
///
/// ## ⭐ 为什么是「扁平缓冲」而不是 `Vec<(u32, Vec<f32>)>`
///
/// 早期形态每个取值各带一个 `Vec<f32>` ⇒ **每帧 16 次堆分配**（外加外层
/// `Vec`）⇒ 60fps 下就是每秒近千次分配 ⇒ GC 压力与卡顿来源。
///
/// ⇒ 现在是**两块扁平缓冲**：
/// - `entries: Vec<(索引, 起点, 分量数)>` —— 索引表；
/// - `values: Vec<f32>` —— 所有取值**首尾相接**。
///
/// 同一个索引重复赋值 ⇒ **原地覆盖**（不重新分配）⇒ 稳态下**每帧零堆分配**
/// （容量在第一次赋值后就不再增长）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UniformValues {
    /// 索引表：`(索引, 在 `values` 里的起点, 分量数)`。
    entries: Vec<(u32, usize, usize)>,
    /// 扁平取值缓冲。
    values: Vec<f32>,
}

impl UniformValues {
    /// 空的 uniform 值集合。
    pub fn new() -> Self {
        Self::default()
    }

    /// 按索引赋值（覆盖同索引的旧值）。
    ///
    /// ⚠ **零分配**：同索引覆盖是**原地**改写；新索引只在**首次**赋值时增长
    /// 缓冲（之后容量复用 ⇒ 稳态零分配）。
    pub fn set(&mut self, binding: u32, components: &[f32]) -> &mut Self {
        if let Some(position) = self
            .entries
            .iter()
            .position(|(existing, _, _)| *existing == binding)
        {
            let (_, start, len) = self.entries[position];
            if len == components.len() {
                // 同分量数 ⇒ **原地覆盖**（零分配）。
                self.values[start..start + len].copy_from_slice(components);
                return self;
            }
            // 分量数变了 ⇒ 该取值在扁平缓冲里的长度变了 ⇒ **重排整块缓冲**
            // （罕见路径：宿主改了某个 uniform 的分量数）。
            let mut values = Vec::with_capacity(self.values.len() + components.len());
            let mut entries = Vec::with_capacity(self.entries.len());
            for (index, (existing, at, count)) in self.entries.iter().enumerate() {
                if index == position {
                    let new_at = values.len();
                    values.extend_from_slice(components);
                    entries.push((*existing, new_at, components.len()));
                } else {
                    let source = &self.values[*at..*at + *count];
                    let new_at = values.len();
                    values.extend_from_slice(source);
                    entries.push((*existing, new_at, *count));
                }
            }
            self.values = values;
            self.entries = entries;
            return self;
        }
        let start = self.values.len();
        self.values.extend_from_slice(components);
        self.entries.push((binding, start, components.len()));
        self
    }

    /// 按 uniform 名赋值（名字 → 索引走引擎自维护的映射表）。
    ///
    /// # Errors
    ///
    /// 名字不在映射表里 → [`BindingError::UnknownUniform`]。
    pub fn set_by_name(
        &mut self,
        name: &str,
        components: &[f32],
    ) -> Result<&mut Self, BindingError> {
        let binding =
            binding_of(name).ok_or_else(|| BindingError::UnknownUniform(name.to_string()))?;
        Ok(self.set(binding, components))
    }

    /// 取某个索引的值。
    pub fn get(&self, binding: u32) -> Option<&[f32]> {
        self.entries
            .iter()
            .find(|(existing, _, _)| *existing == binding)
            .map(|(_, start, len)| &self.values[*start..*start + *len])
    }

    /// 已赋值的索引列表（升序）。
    pub fn assigned_bindings(&self) -> Vec<u32> {
        let mut bindings: Vec<u32> = self
            .entries
            .iter()
            .map(|(binding, _, _)| *binding)
            .collect();
        bindings.sort_unstable();
        bindings
    }

    /// 按布局把值写进缓冲字节（小端 f32）。
    ///
    /// **引擎自填成员**（padding / 帧缓冲高度）**不由宿主提供** ⇒ 跳过
    /// （引擎自己在别处写入）。
    ///
    /// # Errors
    ///
    /// - 分量数与声明不符 → [`BindingError::ComponentCountMismatch`]；
    /// - 成员缺值 → [`BindingError::MissingValue`]；
    /// - 缓冲太小 → [`BindingError::BufferTooSmall`]。
    pub fn write_into(
        &self,
        layout: &UniformBufferLayout,
        bytes: &mut [u8],
    ) -> Result<(), BindingError> {
        if (bytes.len() as u64) < layout.total_size() {
            return Err(BindingError::BufferTooSmall {
                needed: layout.total_size(),
                got: bytes.len() as u64,
            });
        }
        for slot in layout.slots() {
            if slot.is_padding || slot.is_engine_filled {
                // 引擎自填成员：宿主不必提供（padding 留 0；帧缓冲高度由引擎写）。
                continue;
            }
            let values = self
                .get(slot.index as u32)
                .ok_or_else(|| BindingError::MissingValue {
                    binding: slot.index as u32,
                    name: slot.name.clone(),
                })?;
            if values.len() != slot.components {
                return Err(BindingError::ComponentCountMismatch {
                    binding: slot.index as u32,
                    expected: slot.components,
                    got: values.len(),
                });
            }
            let start = slot.offset as usize;
            for (index, value) in values.iter().enumerate() {
                let at = start + index * 4;
                bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glsl::uniform_declarations;

    // -----------------------------------------------------------------------
    // 映射表
    // -----------------------------------------------------------------------

    #[test]
    fn binding_table_is_dense_and_ordered() {
        let table = binding_table();
        assert_eq!(table.len(), SHADER_UNIFORM_NAMES.len());
        for (index, (name, binding)) in table.iter().enumerate() {
            assert_eq!(*binding, index as u32, "索引必须等于表下标：{name}");
        }
    }

    #[test]
    fn binding_lookup_is_exact_and_never_falls_back_to_zero() {
        assert_eq!(binding_of("uRes"), Some(0));
        assert_eq!(binding_of("uCenter"), Some(1));
        assert_eq!(binding_of("uScale"), Some(2));
        assert_eq!(binding_of("uTime"), Some(3));
        assert_eq!(binding_of("uGold"), None, "不在表里的名字必须返回 None");
        assert_eq!(binding_of("ures"), None, "名字匹配必须大小写敏感");
        assert_eq!(binding_of(""), None);
    }

    #[test]
    fn name_lookup_is_the_inverse_of_binding_lookup() {
        for (name, binding) in binding_table() {
            assert_eq!(name_of(binding), Some(name));
            assert_eq!(binding_of(name), Some(binding));
        }
        assert_eq!(name_of(SHADER_UNIFORM_NAMES.len() as u32), None);
    }

    #[test]
    fn mapping_table_has_no_duplicate_names() {
        let mut names = SHADER_UNIFORM_NAMES.to_vec();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "映射表里不得有重名（否则索引冲突）");
    }

    #[test]
    fn binding_table_matches_shader_declaration_order() {
        // 权威来源：随 crate 入库的着色器资产（生产版 GLSL 的去注释副本）。
        let decls = uniform_declarations(crate::shaders::SCENE_FRAGMENT_GLSL);
        assert_eq!(
            decls.len(),
            SHADER_UNIFORM_NAMES.len(),
            "映射表长度必须等于着色器的 uniform 声明数"
        );
        for (index, decl) in decls.iter().enumerate() {
            assert_eq!(
                decl.name, SHADER_UNIFORM_NAMES[index],
                "第 {index} 个 uniform 的名字与映射表不符"
            );
            assert_eq!(decl.order, index, "声明序必须连续");
            assert_eq!(
                binding_of(&decl.name),
                Some(index as u32),
                "顺序 ⇒ 索引必须成立"
            );
        }
    }

    // -----------------------------------------------------------------------
    // 布局
    // -----------------------------------------------------------------------

    fn sample_members() -> Vec<UniformBlockMember> {
        crate::glsl::uniform_block_members(&uniform_declarations(
            "uniform vec2 uRes;\nuniform float uScale;\n",
        ))
    }

    #[test]
    fn layout_places_members_at_std140_offsets() {
        let layout = UniformBufferLayout::from_members(&sample_members(), 0).expect("布局可建");
        assert_eq!(layout.len(), 3, "2 个 uniform + 1 个 padding 成员");
        assert_eq!(layout.binding(), 0);
        let res = layout.slot(0).expect("成员 0");
        let scale = layout.slot(1).expect("成员 1");
        assert_eq!(res.offset, 0);
        assert_eq!(res.size, 8);
        assert_eq!(res.components, 2);
        assert_eq!(scale.offset, 8, "float 紧跟 vec2 ⇒ 偏移 8（std140）");
        assert_eq!(scale.size, 4);
        assert_eq!(layout.total_size(), 32, "块大小向上对齐到 16");
        assert!(
            layout.slot(2).expect("padding 成员").is_padding,
            "第 3 个成员必须是引擎补的 padding 成员"
        );
    }

    #[test]
    fn layout_matches_the_production_uniform_set() {
        let members = crate::glsl::uniform_block_members(&uniform_declarations(
            crate::shaders::SCENE_FRAGMENT_GLSL,
        ));
        let layout = UniformBufferLayout::from_members(&members, 0).expect("布局可建");
        assert_eq!(layout.len(), 17, "16 个 uniform + 1 个 padding 成员");
        assert_eq!(layout.slot(0).expect("uRes").offset, 0);
        assert_eq!(layout.slot(1).expect("uCenter").offset, 8);
        assert_eq!(layout.slot(2).expect("uScale").offset, 16);
        assert_eq!(layout.slot(15).expect("uTlGold").offset, 68);
        assert_eq!(layout.total_size(), 96, "std140 块大小 = 16 的倍数");
    }

    #[test]
    fn layout_rejects_unsupported_uniform_types() {
        let members =
            crate::glsl::uniform_block_members(&uniform_declarations("uniform int uCount;\n"));
        let err = UniformBufferLayout::from_members(&members, 0).expect_err("整型必须显式拒绝");
        assert!(matches!(err, GlslError::UnsupportedUniformType { .. }));
    }

    #[test]
    fn layout_lookup_by_name_and_index_agree() {
        let layout = UniformBufferLayout::from_members(&sample_members(), 0).expect("布局可建");
        assert_eq!(layout.slot_by_name("uScale").map(|s| s.index), Some(1));
        assert_eq!(layout.slot_by_name("uGhost"), None);
        assert_eq!(layout.slot(9), None);
    }

    #[test]
    fn empty_layout_is_valid() {
        let layout = UniformBufferLayout::from_members(&[], 0).expect("空布局合法");
        assert!(layout.is_empty());
        assert_eq!(layout.total_size(), 0);
    }

    // -----------------------------------------------------------------------
    // 值
    // -----------------------------------------------------------------------

    #[test]
    fn values_are_addressed_by_index() {
        let mut values = UniformValues::new();
        values.set(0, &[320.0, 240.0]);
        values.set(2, &[1.5]);
        assert_eq!(values.get(0), Some(&[320.0, 240.0][..]));
        assert_eq!(values.get(2), Some(&[1.5][..]));
        assert_eq!(values.get(1), None);
        assert_eq!(values.assigned_bindings(), [0, 2]);
    }

    #[test]
    fn values_by_name_resolve_through_the_engine_table() {
        let mut values = UniformValues::new();
        values.set_by_name("uTlGold", &[0.25]).expect("名字在表里");
        assert_eq!(values.get(15), Some(&[0.25][..]));
        let err = values
            .set_by_name("uNope", &[0.0])
            .expect_err("不在表里必须报错");
        assert_eq!(err, BindingError::UnknownUniform("uNope".to_string()));
    }

    #[test]
    fn setting_the_same_index_twice_overwrites() {
        let mut values = UniformValues::new();
        values.set(0, &[1.0]);
        values.set(0, &[2.0]);
        assert_eq!(values.get(0), Some(&[2.0][..]));
        assert_eq!(values.assigned_bindings(), [0]);
    }

    #[test]
    fn write_into_places_components_at_declared_offsets() {
        let layout = UniformBufferLayout::from_members(&sample_members(), 0).expect("布局可建");
        let mut values = UniformValues::new();
        values.set(0, &[320.0, 240.0]);
        values.set(1, &[2.5]);
        let mut bytes = vec![0u8; layout.total_size() as usize];
        values.write_into(&layout, &mut bytes).expect("写入成功");

        let res = layout.slot(0).expect("成员 0");
        let scale = layout.slot(1).expect("成员 1");
        let read = |offset: u64| {
            f32::from_le_bytes(
                bytes[offset as usize..offset as usize + 4]
                    .try_into()
                    .expect("4 字节"),
            )
        };
        assert_eq!(read(res.offset), 320.0);
        assert_eq!(read(res.offset + 4), 240.0);
        assert_eq!(read(scale.offset), 2.5);
    }

    #[test]
    fn write_into_rejects_component_count_mismatch() {
        let layout = UniformBufferLayout::from_members(&sample_members(), 0).expect("布局可建");
        let mut values = UniformValues::new();
        values.set(0, &[320.0]);
        values.set(1, &[1.0]);
        let mut bytes = vec![0u8; layout.total_size() as usize];
        let err = values
            .write_into(&layout, &mut bytes)
            .expect_err("分量数不符必须报错");
        assert_eq!(
            err,
            BindingError::ComponentCountMismatch {
                binding: 0,
                expected: 2,
                got: 1
            }
        );
    }

    #[test]
    fn write_into_rejects_missing_values() {
        let layout = UniformBufferLayout::from_members(&sample_members(), 0).expect("布局可建");
        let values = UniformValues::new();
        let mut bytes = vec![0u8; layout.total_size() as usize];
        let err = values
            .write_into(&layout, &mut bytes)
            .expect_err("缺值必须报错");
        assert!(matches!(err, BindingError::MissingValue { binding: 0, .. }));
    }

    #[test]
    fn write_into_rejects_small_buffers() {
        let layout = UniformBufferLayout::from_members(&sample_members(), 0).expect("布局可建");
        let mut values = UniformValues::new();
        values.set(0, &[1.0, 2.0]);
        values.set(1, &[3.0]);
        let mut bytes = vec![0u8; 4];
        let err = values
            .write_into(&layout, &mut bytes)
            .expect_err("缓冲太小必须报错");
        assert!(matches!(err, BindingError::BufferTooSmall { .. }));
    }

    // -----------------------------------------------------------------------
    // 引擎自填成员（宿主不必提供值）
    // -----------------------------------------------------------------------

    #[test]
    fn engine_filled_members_do_not_require_a_host_value() {
        // ⭐ 引擎注入的「帧缓冲高度」成员由**引擎**每帧写入 ⇒ 宿主只提供
        // 生产 uniform ⇒ `write_into` **不得**报「缺值」。
        let members =
            crate::glsl::fragment_uniform_block_members(crate::shaders::SCENE_FRAGMENT_GLSL);
        let layout = UniformBufferLayout::from_members(&members, 0).expect("布局可建");
        assert!(
            layout
                .slot_by_name(crate::glsl::FRAMEBUFFER_HEIGHT_MEMBER)
                .is_some(),
            "片元布局里必须有帧缓冲高度成员"
        );

        let mut values = UniformValues::new();
        for (index, name) in SHADER_UNIFORM_NAMES.iter().enumerate() {
            let components: Vec<f32> = if index < 2 { vec![1.0, 1.0] } else { vec![0.5] };
            values.set_by_name(name, &components).expect("写入成功");
        }
        let mut bytes = vec![0u8; layout.total_size() as usize];
        values
            .write_into(&layout, &mut bytes)
            .expect("引擎自填成员不得要求宿主提供值");
    }

    #[test]
    fn overwriting_a_value_does_not_disturb_the_others() {
        // ⭐ 扁平缓冲的**核心**性质：同索引覆盖是**原地**改写 ⇒ 稳态零分配
        //   ⇒ 其它索引的取值**不得**被挪动。
        let mut values = UniformValues::new();
        values.set(0, &[1.0, 2.0]);
        values.set(1, &[3.0]);
        values.set(0, &[4.0, 5.0]);
        assert_eq!(values.get(0), Some(&[4.0f32, 5.0][..]), "覆盖必须生效");
        assert_eq!(values.get(1), Some(&[3.0f32][..]), "其它索引不得被挪动");
        assert_eq!(values.assigned_bindings(), [0, 1], "索引集合不得变");
    }
}
