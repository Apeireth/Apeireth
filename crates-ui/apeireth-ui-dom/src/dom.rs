//! DOM 后端抽象：真实 DOM 与内存 DOM 共用同一套渲染逻辑。
//!
//! 设计文档 §5.1.2：**这是本设计能消灭源码镜像测试的结构性前提** —— 差分 /
//! 委托 / 绑定逻辑全部写成 `D: Dom` 泛型代码，因而 100% 可 native 单测。
//!
//! ## 诚实边界：失败必须显式，不许静默降级
//!
//! `Dom` 的**每一个**操作都返回 [`Result`]：
//! - 节点不存在 ⇒ [`DomError::NodeNotFound`]；
//! - 节点已被摘除 ⇒ [`DomError::NodeDetached`]（设计文档 §5.5.2 规则 T6：
//!   被移除的节点不参与本批次的属性写入 ⇒ **必须可判定**，不能静默吞掉）；
//! - 操作不适用 ⇒ [`DomError::Unsupported`]；
//! - 后端未接线 ⇒ [`DomError::NotImplemented`]。
//!
//! 先例：`ui-gpu` 的 `GpuContext::submit_frame -> Result<(), GpuError>`。
//!
//! ## 后端清单（P2 实况）
//!
//! | 后端 | 目标 | 状态 |
//! |---|---|---|
//! | [`InMemoryDom`] | native（永远可用） | ✅ 已实现，且**记录操作序列**（T1–T6 的机械判据） |
//! | `WebDom`（web-sys） | wasm32 only | ✅ 已实现；**未经任何运行时验证**（见 crate 文档头） |
//! | [`UnavailableDom`] | —— | 🔵 未接线桩：每个操作都报 [`DomError::NotImplemented`] |

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};

use crate::listeners::ListenerTable;

/// 元素盒（CSS 像素，语义与浏览器 `getBoundingClientRect()` 一致）。
///
/// ⚠️ **引擎不做布局**（ADR-0015）：`bounding_box` 是向 DOM/宿主**查询**，
/// 不是引擎自己算出来的。内存后端只提供**脚本化**测量（测试用）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ElementBox {
    /// 左边（相对视口，CSS 像素）。
    pub x: f64,
    /// 上边（相对视口，CSS 像素）。
    pub y: f64,
    /// 宽（CSS 像素）。
    pub width: f64,
    /// 高（CSS 像素）。
    pub height: f64,
}

impl ElementBox {
    /// 构造元素盒（CSS 像素）。
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    /// 右边界（`x + width`）。
    pub fn right(&self) -> f64 {
        self.x + self.width
    }
    /// 下边界（`y + height`）。
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }
    /// 点是否落在盒内（命中测试的最基本形式）。
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }
}

/// ⭐ **测量与命中测试的乘数**：设备像素比（DPR）× 界面密度（L1-G6）。
///
/// ## 为什么必须由引擎拥有
///
/// 系统缩放 125%、紧凑密度、Retina 屏下，**同一份 CSS 像素**对应不同设备像素
/// ⇒ ⇒ 命中测试、纹理采样、拖拽阈值都必须按这个乘数换算
/// ⇒ ⇒ 若让每个上层自己拿 `devicePixelRatio` 相乘，必然出现「差 1px」「拖不动」
/// 这类只在特定缩放下复现的缺陷。
///
/// ## 单位约定
///
/// - **CSS 像素**：布局与事件坐标（`ElementBox`、`client_position`）；
/// - **设备像素**：渲染目标与采样。
/// - `device = css × dpr × density`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeasurementScale {
    device_pixel_ratio: f64,
    density: f64,
}

impl Default for MeasurementScale {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl MeasurementScale {
    /// 恒等乘数（dpr = 1、密度 = 1）。
    pub const IDENTITY: Self = Self {
        device_pixel_ratio: 1.0,
        density: 1.0,
    };

    /// 构造乘数。非法值（`<= 0` / 非有限）按 **1.0** 处理并**不**静默产生 0 尺寸。
    pub fn new(device_pixel_ratio: f64, density: f64) -> Self {
        let sanitize = |value: f64| {
            if value.is_finite() && value > 0.0 {
                value
            } else {
                1.0
            }
        };
        Self {
            device_pixel_ratio: sanitize(device_pixel_ratio),
            density: sanitize(density),
        }
    }

    /// 设备像素比。
    pub fn device_pixel_ratio(&self) -> f64 {
        self.device_pixel_ratio
    }

    /// 界面密度标量。
    pub fn density(&self) -> f64 {
        self.density
    }

    /// CSS 像素 ⇒ 设备像素。
    pub fn css_to_device(&self, css: f64) -> f64 {
        css * self.device_pixel_ratio * self.density
    }

    /// 设备像素 ⇒ CSS 像素。
    pub fn device_to_css(&self, device: f64) -> f64 {
        device / (self.device_pixel_ratio * self.density)
    }

    /// 把元素盒整体换算到设备像素。
    pub fn box_to_device(&self, rect: ElementBox) -> ElementBox {
        ElementBox::new(
            self.css_to_device(rect.x),
            self.css_to_device(rect.y),
            self.css_to_device(rect.width),
            self.css_to_device(rect.height),
        )
    }
}

/// 滚动度量（CSS 像素）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScrollMetrics {
    /// 当前滚动位置（CSS 像素，0 = 顶部）。
    pub scroll_top: f64,
    /// 内容总高度（CSS 像素）。
    pub scroll_height: f64,
    /// 可视区高度（CSS 像素）。
    pub client_height: f64,
}

impl ScrollMetrics {
    /// 可滚动的最大 `scroll_top`（内容不足一屏时为 0）。
    pub fn max_scroll_top(&self) -> f64 {
        (self.scroll_height - self.client_height).max(0.0)
    }
    /// 把任意 `scroll_top` **夹到**合法区间。
    pub fn clamp_scroll_top(&self, top: f64) -> f64 {
        top.clamp(0.0, self.max_scroll_top())
    }
}

/// 脚本化测量（**测试专用**）：一个节点的元素盒与滚动度量。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScriptedMeasurement {
    /// 该节点的元素盒。
    pub bounding_box: ElementBox,
    /// 该节点的滚动度量。
    pub scroll_metrics: ScrollMetrics,
}

impl ScriptedMeasurement {
    /// 构造脚本化测量。
    pub fn new(bounding_box: ElementBox, scroll_metrics: ScrollMetrics) -> Self {
        Self {
            bounding_box,
            scroll_metrics,
        }
    }
}

/// DOM 事件类型（按仓库实测使用面裁剪，不照抄浏览器 API 全集）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventKind {
    /// 点击（冒泡，委托根监听）。
    Click,
    /// 双击（冒泡）。
    DoubleClick,
    /// 键盘按下（冒泡）。
    KeyDown,
    /// 键盘抬起（冒泡）—— 组合键判定要用它（只按 `KeyDown` 会漏「松开」）。
    KeyUp,
    /// 输入（冒泡；双向绑定的取值事件；IME 组合期间**不**触发，见 G3）。
    Input,
    /// 变更（冒泡）—— 复选框/单选的 `checked` 走这条。
    Change,
    /// 获得焦点（`focusin` **冒泡**版本；`focus` 本身不冒泡，不用）。
    FocusIn,
    /// 失焦（冒泡）。
    FocusOut,
    /// 表单提交（冒泡）。
    Submit,
    /// 指针按下（冒泡）。
    PointerDown,
    /// 指针抬起（冒泡）。
    PointerUp,
    /// 指针移动（冒泡；**默认会高频** ⇒ 上层需自行节流/合并）。
    PointerMove,
    /// 指针进入（冒泡）。
    PointerEnter,
    /// 指针离开（冒泡）。
    PointerLeave,
    /// 滚动（**不冒泡** ⇒ 必须直接监听滚动容器本身，不能靠委托根）。
    Scroll,
    /// 滚轮（冒泡）。
    Wheel,
    /// 右键菜单（冒泡）。
    ContextMenu,
    /// ⭐ **IME 组合开始**（冒泡）⇒ 必须**关掉**受控写回（见 `focus::CompositionGate`）。
    CompositionStart,
    /// ⭐ **IME 组合结束**（冒泡）⇒ 打开写回并采纳组合结果（`data` 载荷）。
    CompositionEnd,
}

impl EventKind {
    /// 事件名（与仓内事件属性同名，降低迁移摩擦）。
    pub fn as_str(&self) -> &'static str {
        match self {
            EventKind::Click => "click",
            EventKind::DoubleClick => "dblclick",
            EventKind::KeyDown => "keydown",
            EventKind::KeyUp => "keyup",
            EventKind::Input => "input",
            EventKind::Change => "change",
            EventKind::FocusIn => "focusin",
            EventKind::FocusOut => "focusout",
            EventKind::Submit => "submit",
            EventKind::PointerDown => "pointerdown",
            EventKind::PointerUp => "pointerup",
            EventKind::PointerMove => "pointermove",
            EventKind::PointerEnter => "pointerenter",
            EventKind::PointerLeave => "pointerleave",
            EventKind::Scroll => "scroll",
            EventKind::Wheel => "wheel",
            EventKind::ContextMenu => "contextmenu",
            EventKind::CompositionStart => "compositionstart",
            EventKind::CompositionEnd => "compositionend",
        }
    }

    /// 该事件**能否**挂在委托根上统一监听。
    ///
    /// ⚠️ `scroll` **不冒泡**（浏览器规范）⇒ ⇒ 它**必须**直接监听到滚动容器本身
    /// ⇒ ⇒ 若把它挂到委托根，会**永远收不到事件**（静默失效，最难查的一类缺陷）。
    pub fn bubbles_to_delegation_root(&self) -> bool {
        !matches!(self, EventKind::Scroll)
    }

    /// 该事件在本枚举中的**稳定序号**。
    ///
    /// ⚠️ **穷尽 `match`**：新增变体却忘了在这里登记 ⇒ ⇒ **编译失败**
    /// ⇒ ⇒ 这是「新增事件类型」的**编译期护栏**（比测试更早、更硬）。
    pub fn index(self) -> usize {
        match self {
            EventKind::Click => 0,
            EventKind::DoubleClick => 1,
            EventKind::KeyDown => 2,
            EventKind::KeyUp => 3,
            EventKind::Input => 4,
            EventKind::Change => 5,
            EventKind::FocusIn => 6,
            EventKind::FocusOut => 7,
            EventKind::Submit => 8,
            EventKind::PointerDown => 9,
            EventKind::PointerUp => 10,
            EventKind::PointerMove => 11,
            EventKind::PointerEnter => 12,
            EventKind::PointerLeave => 13,
            EventKind::Scroll => 14,
            EventKind::Wheel => 15,
            EventKind::ContextMenu => 16,
            EventKind::CompositionStart => 17,
            EventKind::CompositionEnd => 18,
        }
    }

    /// 由序号还原事件类型（与 [`Self::index`] 互逆；越界 ⇒ `None`）。
    pub fn from_index(index: usize) -> Option<Self> {
        Some(match index {
            0 => EventKind::Click,
            1 => EventKind::DoubleClick,
            2 => EventKind::KeyDown,
            3 => EventKind::KeyUp,
            4 => EventKind::Input,
            5 => EventKind::Change,
            6 => EventKind::FocusIn,
            7 => EventKind::FocusOut,
            8 => EventKind::Submit,
            9 => EventKind::PointerDown,
            10 => EventKind::PointerUp,
            11 => EventKind::PointerMove,
            12 => EventKind::PointerEnter,
            13 => EventKind::PointerLeave,
            14 => EventKind::Scroll,
            15 => EventKind::Wheel,
            16 => EventKind::ContextMenu,
            17 => EventKind::CompositionStart,
            18 => EventKind::CompositionEnd,
            _ => return None,
        })
    }

    /// 全部事件类型（**由 `from_index` 枚举得出** ⇒ 不会与变体集合漂移）。
    pub fn all() -> Vec<EventKind> {
        let mut kinds = Vec::new();
        let mut index = 0;
        while let Some(kind) = EventKind::from_index(index) {
            kinds.push(kind);
            index += 1;
        }
        kinds
    }
}

/// DOM 后端错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomError {
    /// 节点不存在。
    NodeNotFound,
    /// 节点已从树上摘除（禁止再写入，见设计 §5.5.2 规则 T6）。
    NodeDetached,
    /// **未接线**：后端能力尚未接入。
    NotImplemented,
    /// 不支持的操作。
    Unsupported(String),
    /// **不可判定**：差分无法得出唯一结果（例：同一列表出现重复 key ⇒ 身份不唯一）。
    ///
    /// 与 [`DomError::Unsupported`] 分开：前者是「这个后端做不了」，
    /// 后者是「这个输入没有唯一答案」——两者的处置方式不同。
    Undecidable(String),
}

impl std::fmt::Display for DomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomError::NodeNotFound => write!(f, "NodeNotFound: 节点不存在"),
            DomError::NodeDetached => write!(f, "NodeDetached: 节点已摘除"),
            DomError::NotImplemented => {
                write!(f, "NotImplemented: 该 DOM 后端能力未接线")
            }
            DomError::Unsupported(detail) => write!(f, "Unsupported: {detail}"),
            DomError::Undecidable(detail) => write!(f, "Undecidable: {detail}"),
        }
    }
}

impl std::error::Error for DomError {}

/// DOM 后端抽象：渲染层唯一允许触碰「真实节点」的地方。
///
/// ## 节点句柄的约束
///
/// `Node` 要求 `'static`：节点句柄是**拥有所有权**的句柄（内存后端 = 编号，
/// 真实后端 = JS 句柄），不借用外部数据。`'static` 也是 `bind:this` 槽位
/// 能把句柄**类型擦除**后仍可还原的前提（见 [`crate::binding::ThisRef`]）。
///
/// 后端本身同样要求 `'static`（`Dom: 'static`）：后端由渲染器**拥有**，
/// 且 `bind:this` 的擦除视图要求后端类型可被放进 `'static` 的 trait object。
pub trait Dom: 'static {
    /// 后端节点类型（真实后端是 web-sys 的节点句柄，内存后端是编号）。
    type Node: Clone + PartialEq + 'static;

    /// 建一个元素节点。
    fn create_element(&self, tag: &str) -> Result<Self::Node, DomError>;

    /// 建一个**命名空间**内的元素节点（SVG 等）。
    ///
    /// 默认实现 = [`Dom::create_element`]：内存后端不渲染，命名空间对它没有
    /// 意义（它只记录标签名）⇒ 默认实现不是「假装支持」，而是「本后端确实
    /// 不需要」。
    ///
    /// 真实 DOM 后端**必须**覆写：HTML 命名空间里的 `<svg>` **不是** SVG 元素
    /// （浏览器不会把它当矢量图渲染，`viewBox` 这类驼峰属性名还会被小写化）
    /// ⇒ SVG 元素必须走 `createElementNS`。
    fn create_element_ns(&self, namespace: &str, tag: &str) -> Result<Self::Node, DomError> {
        let _ = namespace;
        self.create_element(tag)
    }

    /// 建一个文本节点。
    fn create_text(&self, data: &str) -> Result<Self::Node, DomError>;

    /// ⭐ **元素盒（CSS 像素）** —— 与浏览器 `getBoundingClientRect()` **同语义**
    /// （相对**视口**左上角、含边框、不含外边距）。
    ///
    /// ## 为什么是 CSS 像素而不是设备像素
    ///
    /// 上层（弹层定位、滚动、拖拽命中）用的是 CSS 像素；设备像素只在**上传/采样**时才需要
    /// ⇒ ⇒ 换算放在一处，不要散落在各处
    /// ⇒ ⇒ 否则 125% 缩放/紧凑密度下必然出现「差 1px」类缺陷。
    ///
    /// 默认实现返回 [`DomError::NotImplemented`]（**显式**不支持，不是静默零值）。
    fn bounding_box(&self, node: &Self::Node) -> Result<ElementBox, DomError> {
        let _ = node;
        Err(DomError::NotImplemented)
    }

    /// ⭐ **滚动度量**（CSS 像素）：当前滚动位置、内容高度、可视高度。
    fn scroll_metrics(&self, node: &Self::Node) -> Result<ScrollMetrics, DomError> {
        let _ = node;
        Err(DomError::NotImplemented)
    }

    /// ⭐ **把焦点移到该节点**（L1-G3 接线点）。
    ///
    /// ⚠️ 焦点**策略**（栈 / 陷阱 / 恢复）在 [`crate::focus::FocusTracker`] 与渲染器里；
    /// 后端只负责「把浏览器的焦点真的挪过去」这一件事。
    fn focus(&self, node: &Self::Node) -> Result<(), DomError> {
        let _ = node;
        Err(DomError::NotImplemented)
    }

    /// ⭐ **把焦点从该节点移走**。
    fn blur(&self, node: &Self::Node) -> Result<(), DomError> {
        let _ = node;
        Err(DomError::NotImplemented)
    }

    /// 当前聚焦的节点（后端不知道 ⇒ `None`；测试后端会记录）。
    fn focused(&self) -> Option<Self::Node> {
        None
    }

    /// ⭐ **是否处于 IME 组合中**（L1-G3 接线点）。
    ///
    /// ## 为什么放在后端 trait 上
    ///
    /// 组合事件的**到达**发生在后端（它把浏览器事件折成 [`crate::events::DomEvent`]）
    /// ⇒ ⇒ 把「组合中」这个状态记在后端，渲染器就不必依赖宿主转告
    /// ⇒ ⇒ 否则每个宿主都要自己记得调 `note_event()`，忘一处就回到「中文输入被打散」。
    ///
    /// 默认 `false`（不认识组合事件的后端 = 永远不在组合中）。
    fn is_composing(&self) -> bool {
        false
    }

    /// ⭐ **设置滚动位置**（CSS 像素）⇒ 实现应把值**夹到** `[0, max_scroll_top]`。
    fn set_scroll_top(&self, node: &Self::Node, top: f64) -> Result<(), DomError> {
        let _ = (node, top);
        Err(DomError::NotImplemented)
    }

    /// 写属性。
    fn set_attribute(&self, node: &Self::Node, name: &str, value: &str) -> Result<(), DomError>;

    /// 删属性。
    fn remove_attribute(&self, node: &Self::Node, name: &str) -> Result<(), DomError>;

    /// 增删 class token（**token 级**差分；设计文档 §5.2.1）。
    ///
    /// 为什么不是整体覆写 `class` 字符串：整体覆写会连带抹掉别的 class，
    /// 且在有 CSS `transition` 时会产生额外的中间态。
    ///
    /// 语义：先删 `removed` 里的 token，再加 `added` 里的 token（去重保序）。
    fn set_class_tokens(
        &self,
        node: &Self::Node,
        added: &[String],
        removed: &[String],
    ) -> Result<(), DomError>;

    /// 写 property（`value` / `checked` 等）。
    ///
    /// **不是属性**：`bind:value` 走 property（设计文档 §5.4.1）——写 attribute
    /// 对 `input.value` 无效，且会破坏浏览器的受控输入语义。
    fn set_property(&self, node: &Self::Node, name: &str, value: &str) -> Result<(), DomError>;

    /// 写**布尔** property（`checked` / `selected` / …）。
    ///
    /// ⚠️ **不能**用 [`Self::set_property`] 传 `"false"` 写布尔 property ——
    /// `Reflect::set(el, "checked", "false")` 写进去是**非空字符串** ⇒ 真值判断为**真**
    /// ⇒ 语义**反了**。
    ///
    /// 默认实现 ⇒ **显式失败**（`Unsupported`）而不是静默写错：
    /// 没实现该通道的后端**不该假装支持**（0 装）。
    fn set_property_bool(
        &self,
        node: &Self::Node,
        name: &str,
        value: bool,
    ) -> Result<(), DomError> {
        let _ = (node, name, value);
        Err(DomError::Unsupported(format!(
            "set_property_bool({name}) 该后端未实现"
        )))
    }

    /// 读 property。
    fn get_property(&self, node: &Self::Node, name: &str) -> Result<String, DomError>;

    /// **移除** property（`value` / `checked` 等）。
    ///
    /// ## 为什么必须有这一条（2026-10-25 修）
    ///
    /// 修前渲染器遇到「property 从虚拟树上消失」只能报 [`DomError::Unsupported`]
    /// ⇒ ⇒ **整帧中止** ⇒ ⇒ DOM 已经改了一半 ⇒ 界面冻结。
    ///
    /// ⇒ 「property 消失」是**正常**的差分结果（条件渲染会让 property 时有时无）
    /// ⇒ ⇒ 必须有一条**能真正移除**的通道。
    ///
    /// ## 语义（后端必须遵守）
    ///
    /// 移除之后 [`Dom::get_property`] 读到的必须是该 property 的**默认值**
    /// （字符串 property ⇒ 空串；布尔 property ⇒ `false`）⇒
    /// ⇒ 与浏览器的「回落到默认值」一致。
    ///
    /// ## 默认实现 = **显式失败**（0 装）
    ///
    /// 没实现该通道的后端**不该假装支持** ⇒ 报 [`DomError::Unsupported`] ⇒
    /// 渲染器据此**降级**（写回默认值 + 记警告），**不中止整帧**。
    fn remove_property(&self, node: &Self::Node, name: &str) -> Result<(), DomError> {
        let _ = (node, name);
        Err(DomError::Unsupported(String::from(
            "remove_property 该后端未实现（property 无通用删除操作）",
        )))
    }

    /// 写文本内容（就地改，不重建文本节点，见设计 §5.2.4）。
    fn set_text(&self, node: &Self::Node, data: &str) -> Result<(), DomError>;

    /// 追加子节点。
    fn append_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError>;

    /// 在锚点之前插入子节点（`anchor = None` ⇒ 追加到末尾）。
    ///
    /// keyed 列表的**移动**必须走本操作（设计文档 §5.2.3）：移动 = 换位置，
    /// **不是**「先摘除再追加」——后者会让节点短暂离开文档树，CSS 过渡 /
    /// 动画状态会被打断。
    fn insert_before(
        &self,
        parent: &Self::Node,
        child: &Self::Node,
        anchor: Option<&Self::Node>,
    ) -> Result<(), DomError>;

    /// 移除子节点（节点被摘除后再写入 ⇒ [`DomError::NodeDetached`]）。
    fn remove_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError>;

    /// 读子节点列表（按文档顺序）。
    fn children(&self, parent: &Self::Node) -> Result<Vec<Self::Node>, DomError>;

    /// 挂事件处理器（**元素直挂**：浏览器负责冒泡；`currentTarget` 语义 =
    /// 「处理器注册在哪个元素上」，与设计文档 §5.3.3 的确定解一致）。
    ///
    /// 同一节点同一事件类型挂两次 ⇒ 后者覆盖前者（与虚拟树
    /// [`crate::vnode::ElementVNode::handler`] 的覆盖语义一致）。
    fn add_listener(
        &self,
        node: &Self::Node,
        kind: EventKind,
        handler: crate::events::ElementHandler,
    ) -> Result<(), DomError>;

    /// 摘掉某节点上的**全部**事件处理器（卸载时必做；差分时重挂前也做）。
    fn remove_listeners(&self, node: &Self::Node) -> Result<(), DomError>;

    /// 摘掉某节点上**指定事件类型**的处理器（其余类型的**保留**）。
    ///
    /// ## 为什么必须有这一条（2026-10-25 修）
    ///
    /// 差分时「换了处理器 ⇒ 重挂」如果走 [`Dom::remove_listeners`]（摘**全部**）
    /// ⇒ ⇒ 一个元素上挂多个事件类型时（例：标题同时挂 `click` 与 `keydown`）
    /// ⇒ ⇒ 摘 `click` 会把 `keydown` 一起带走 ⇒ ⇒ **先挂的处理器静默失效**。
    ///
    /// ⇒ ⇒ 必须有**按 kind** 的增量摘挂。
    ///
    /// ## 默认实现 = 摘**全部**（⚠️ 会误伤其它 kind）
    ///
    /// 没实现按 kind 摘挂的后端**会**误伤同元素的其它事件类型 ⇒
    /// ⇒ **后端应当覆写**本方法（内存后端与真实后端都已覆写）。
    fn remove_listener(&self, node: &Self::Node, kind: EventKind) -> Result<(), DomError> {
        let _ = kind;
        self.remove_listeners(node)
    }

    /// 目标节点在树里的**子节点下标路径**（从根到目标；空 = 目标就是根）。
    ///
    /// ## 口径（两个后端必须一致 ⇒ 事件负载才可比）
    ///
    /// - 只数**元素与文本**子节点（注释节点**跳过**）⇒ 与 [`Dom::children`]
    ///   的口径一致 ⇒ 下标空间两侧相同；
    /// - 从**最顶层的祖先**算起（浏览器侧 = 文档根；内存侧 = 树的根）。
    ///
    /// ## 为什么要有这一条
    ///
    /// 事件负载里的 `target_path` 是**委托路由**的依据 ⇒ 两个后端算出来的
    /// 路径必须**逐项可比** ⇒ ⇒ 口径必须写在 trait 上（而不是各后端各写一套）。
    ///
    /// 默认实现 = **显式失败**（0 装）：后端算不出来就报不支持，不许瞎编。
    fn target_path(&self, node: &Self::Node) -> Result<Vec<usize>, DomError> {
        let _ = node;
        Err(DomError::Unsupported(String::from(
            "target_path 该后端未实现（事件路径不可判定）",
        )))
    }
}

/// 内存 DOM 的节点句柄（编号）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InMemoryNodeId(pub usize);

/// 内存 DOM 内部节点。
#[derive(Debug, Clone)]
enum InMemoryNodeKind {
    Element(String),
    Text(String),
}

/// 内存 DOM 内部节点记录。
#[derive(Debug, Clone)]
struct InMemoryNode {
    kind: InMemoryNodeKind,
    attrs: Vec<(String, String)>,
    props: Vec<(String, MemoryProp)>,
    children: Vec<InMemoryNodeId>,
    /// 当前父节点（`None` = 尚未挂到任何树上）。
    parent: Option<InMemoryNodeId>,
    /// 是否已被从树上摘除（摘除后禁止再写入 —— 设计文档 §5.5.2 规则 T6）。
    detached: bool,
}

/// 内存后端里的一个 property 值。
///
/// ## 为什么带**类型**
///
/// 布尔 property 的**默认值**是 `false`（不是空串）⇒ ⇒ 移除 property 时
/// 要写回**对应类型**的默认值 ⇒ ⇒ 必须记得它当初是**布尔**还是**字符串**。
#[derive(Debug, Clone)]
struct MemoryProp {
    /// 当前值（字符串形式）。
    value: String,
    /// 是否是**布尔** property（`set_property_bool` 写的那些）。
    is_bool: bool,
}

impl MemoryProp {
    /// 该 property 的**默认值**（浏览器语义：没有该属性时的取值）。
    fn default_value(is_bool: bool) -> String {
        if is_bool {
            String::from("false")
        } else {
            String::new()
        }
    }
}

/// 内存 DOM 记录的一次操作。
///
/// 这是**机械判定**设计文档 §5.5.2 六条保护规则（T1–T6）的依据：规则问的
/// 是「引擎按什么顺序写了什么」，操作序列正是那个问题的答案。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomOp {
    /// 建元素节点。
    CreateElement {
        /// 标签名。
        tag: String,
    },
    /// 建文本节点。
    CreateText {
        /// 文本内容。
        data: String,
    },
    /// 写属性。
    SetAttribute {
        /// 属性名。
        name: String,
        /// 属性值。
        value: String,
    },
    /// 删属性。
    RemoveAttribute {
        /// 属性名。
        name: String,
    },
    /// 写 property。
    SetProperty {
        /// property 名。
        name: String,
        /// property 值。
        value: String,
    },
    /// 写**布尔** property（`checked` / `selected` / …）。
    ///
    /// 与 [`Self::SetProperty`] **分开记** ⇒ 「布尔走布尔通道」这件事
    /// 是**可观测的**（否则写错成字符串在记录里看不出来）。
    SetPropertyBool {
        /// property 名。
        name: String,
        /// property 值。
        value: bool,
    },
    /// 读 property（不改树；记录它是为了让「读」在序列里可见）。
    GetProperty {
        /// property 名。
        name: String,
    },
    /// 写文本内容。
    SetText {
        /// 新文本。
        data: String,
    },
    /// **移除** property。
    RemoveProperty {
        /// property 名。
        name: String,
    },
    /// 增删 class token（token 级差分）。
    SetClassTokens {
        /// 要加的 token。
        added: Vec<String>,
        /// 要删的 token。
        removed: Vec<String>,
    },
    /// 追加子节点。
    AppendChild {
        /// 父节点。
        parent: InMemoryNodeId,
    },
    /// 在锚点前插入子节点（`anchor = None` ⇒ 追加到末尾）。
    InsertBefore {
        /// 父节点。
        parent: InMemoryNodeId,
        /// 锚点节点（`None` = 末尾）。
        anchor: Option<InMemoryNodeId>,
    },
    /// 移除子节点。
    RemoveChild {
        /// 父节点。
        parent: InMemoryNodeId,
    },
    /// 挂事件处理器。
    AddListener {
        /// 事件类型。
        kind: EventKind,
    },
    /// 摘掉某节点上**指定事件类型**的处理器（其余类型保留 ⇒ 增量替换）。
    RemoveListener {
        /// 事件类型。
        kind: EventKind,
    },
    /// 摘掉某节点上的全部事件处理器。
    RemoveListeners,
}

/// 一次操作 + 它作用的节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedOp {
    /// 操作内容。
    pub op: DomOp,
    /// 作用的节点。
    ///
    /// 树操作（`AppendChild` / `InsertBefore` / `RemoveChild`）记的是**子节点**
    /// （父节点记在 `DomOp` 里）。
    pub node: InMemoryNodeId,
}

/// 内存 DOM 后端：native 单测用（永远可用，不需浏览器、不需 wasm target）。
///
/// ## 操作序列记录
///
/// 后端**始终**记录自己执行过的操作（[`InMemoryDom::ops`]）⇒ T1–T6 可以被
/// 机械判定（而不必读源码文本 —— 设计文档 §10.2 明令禁止源码镜像断言）。
///
/// ## 两道「不许无界增长」的闸（2026-10-25 修）
///
/// 1. **节点**：摘除时**立即释放载荷**（属性 / property / 子节点 / **监听器**）
///    ⇒ 监听器闭包（它捕获着应用状态）**不再随摘除的节点一起泄漏**。
///    ⇒ 节点**槽位**只增不复用（句柄一旦发出就必须仍能判定为「已摘除」）。
/// 2. **操作日志**：有**上界**（[`InMemoryDom::set_op_capacity`]）⇒ 超出后
///    **丢弃最旧**的记录 ⇒ 长时间运行不会把内存吃光。
#[derive(Debug)]
pub struct InMemoryDom {
    nodes: RefCell<Vec<InMemoryNode>>,
    /// 事件监听器登记表（按 **(节点, 事件类型)** 索引 ⇒ 增量替换）。
    listeners: RefCell<ListenerTable<InMemoryNodeId, crate::events::ElementHandler>>,
    /// ⭐ **脚本化测量表**（**测试用**，不是布局引擎）：节点 → (元素盒, 滚动度量)。
    ///
    /// 引擎不做布局（ADR-0015）⇒ ⇒ 内存后端**不计算**几何，只接受测试脚本注入的几何
    /// ⇒ ⇒ 这样上层（弹层定位 / 滚动物理 / 命中测试）可以在 native 侧被确定性地测试。
    /// 未注入 ⇒ 三个测量方法返回 [`DomError::NotImplemented`]（**显式**不支持）。
    measurements: RefCell<HashMap<InMemoryNodeId, ScriptedMeasurement>>,
    /// ⭐ 测试用 IME 组合态（见 `script_composition`）。
    composing: Cell<bool>,
    /// ⭐ 当前聚焦节点（**测试用**：记录 `focus()` 的效果 ⇒ 焦点策略可被断言）。
    focused: Cell<Option<InMemoryNodeId>>,
    ops: RefCell<VecDeque<RecordedOp>>,
    /// 操作日志的上界（超出后丢弃最旧的记录）。
    op_capacity: Cell<usize>,
}

/// 操作日志的**默认**上界。
///
/// 为什么要有上界：日志是**判据**用的（长跑场景下没人看得完几十万条）⇒
/// ⇒ 留一个足够宽的窗口（够看完一批渲染）⇒ 超出后丢最旧的。
pub const DEFAULT_OP_CAPACITY: usize = 8192;

impl Default for InMemoryDom {
    fn default() -> Self {
        Self {
            nodes: RefCell::new(Vec::new()),
            listeners: RefCell::new(ListenerTable::new()),
            measurements: RefCell::new(HashMap::new()),
            composing: Cell::new(false),
            focused: Cell::new(None),
            ops: RefCell::new(VecDeque::new()),
            op_capacity: Cell::new(DEFAULT_OP_CAPACITY),
        }
    }
}

impl InMemoryDom {
    /// 节点必须存在（测量 API 的公共前置检查）。
    /// ⭐ **脚本化 IME 组合态**（**测试用**）：模拟「输入法正在组合」。
    pub fn script_composition(&self, composing: bool) {
        self.composing.set(composing);
    }

    fn ensure_node_exists(&self, node: &InMemoryNodeId) -> Result<(), DomError> {
        if self.nodes.borrow().get(node.0).is_none() {
            return Err(DomError::NodeNotFound);
        }
        Ok(())
    }

    /// 建一棵空的内存 DOM。
    pub fn new() -> Self {
        Self::default()
    }

    /// ⭐ **注入脚本化测量**（**测试用**）：给某个节点设定元素盒与滚动度量。
    ///
    /// ⚠️ 这不是布局引擎：引擎不做布局（ADR-0015）。这里只是让上层能在 native 侧
    /// 对「测量 → 定位 / 滚动 / 命中」这条链做**确定性**测试。
    pub fn script_measurement(
        &self,
        node: InMemoryNodeId,
        bounding_box: ElementBox,
        scroll_metrics: ScrollMetrics,
    ) {
        self.measurements
            .borrow_mut()
            .insert(node, ScriptedMeasurement::new(bounding_box, scroll_metrics));
    }

    /// 读节点标签（元素节点才有）。
    pub fn tag(&self, node: InMemoryNodeId) -> Option<String> {
        match &self.nodes.borrow().get(node.0)?.kind {
            InMemoryNodeKind::Element(tag) => Some(tag.clone()),
            InMemoryNodeKind::Text(_) => None,
        }
    }

    /// 读属性值。
    pub fn attribute(&self, node: InMemoryNodeId, name: &str) -> Option<String> {
        let nodes = self.nodes.borrow();
        let entry = nodes.get(node.0)?;
        entry
            .attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    }

    /// 读 property 值（**实时**语义：读的是当前值）。
    pub fn property(&self, node: InMemoryNodeId, name: &str) -> Option<String> {
        let nodes = self.nodes.borrow();
        let entry = nodes.get(node.0)?;
        entry
            .props
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.value.clone())
    }

    /// 读文本内容（文本节点自身或子树拼接）。
    pub fn text_content(&self, node: InMemoryNodeId) -> String {
        let nodes = self.nodes.borrow();
        let Some(entry) = nodes.get(node.0) else {
            return String::new();
        };
        match &entry.kind {
            InMemoryNodeKind::Text(data) => data.clone(),
            InMemoryNodeKind::Element(_) => entry
                .children
                .iter()
                .map(|child| self.text_content(*child))
                .collect::<Vec<_>>()
                .join(""),
        }
    }

    /// 节点总数（**活着的**节点数 —— 已摘除的不算）。
    ///
    /// 用途：断言「没有多余节点产生」⇒ 摘除的节点**不算**在里面
    /// （它们已经是垃圾，见 [`InMemoryDom::slot_count`]）。
    pub fn node_count(&self) -> usize {
        self.nodes
            .borrow()
            .iter()
            .filter(|entry| !entry.detached)
            .count()
    }

    /// **槽位**总数（含已摘除的墓碑）。
    ///
    /// ## 为什么槽位只增不复用
    ///
    /// 节点句柄（[`InMemoryNodeId`]）一旦发出就可能被外部长期持有 ⇒
    /// ⇒ 复用槽位会让**过期句柄打到新节点上**（比泄漏更危险）⇒
    /// ⇒ 槽位不复用；但摘除时**载荷**（含监听器闭包）**立即释放** ⇒
    /// ⇒ 真正会泄漏的闭包**不会**泄漏。
    pub fn slot_count(&self) -> usize {
        self.nodes.borrow().len()
    }

    /// 回收**尾部**的墓碑槽位（已摘除且载荷已空的槽位）⇒ 返回回收的个数。
    ///
    /// ## 为什么只回收「尾部」
    ///
    /// 槽位编号 = 下标 ⇒ 回收中间的槽位会让后面所有编号**平移** ⇒
    /// ⇒ 外面持有的句柄全部失效 ⇒ ⇒ 只回收**尾部**那段（编号连续、无外部
    /// 活句柄依赖）⇒ 中间的墓碑保留（它们只是一条 `detached` 标记）。
    ///
    /// ⚠️ 回收之后，**被回收的那些编号**不再可判定（`is_detached` 返回 `false`）。
    pub fn compact(&self) -> usize {
        let mut nodes = self.nodes.borrow_mut();
        let mut reclaimed = 0usize;
        while let Some(last) = nodes.last() {
            if !last.detached {
                break;
            }
            nodes.pop();
            reclaimed += 1;
        }
        reclaimed
    }

    /// 操作日志的上界。
    pub fn op_capacity(&self) -> usize {
        self.op_capacity.get()
    }

    /// 设操作日志的上界（`0` = 不记录）。
    pub fn set_op_capacity(&self, capacity: usize) {
        self.op_capacity.set(capacity);
        let mut ops = self.ops.borrow_mut();
        while ops.len() > capacity {
            ops.pop_front();
        }
    }

    /// ⭐ 派发**带载荷**的事件（`target_value`）—— 与真实浏览器侧的 `input` 事件同语义。
    ///
    /// ## 为什么需要它（而不是复用 [`Self::dispatch`]）
    ///
    /// `dispatch` 造的事件**没有 `target_value`** ⇒ ⇒ 绑定写回拿不到值
    /// ⇒ ⇒ 「DOM → 信号」这条上行路径在 native 侧**无法测**
    /// ⇒ ⇒ 而真实浏览器侧（`WebDom`）会把 `input.value` 填进载荷
    /// ⇒ ⇒ native 与浏览器**语义必须一致**，否则 native 测试是假的。
    ///
    /// 返回实际调用次数（与 [`Self::dispatch`] 同口径）。
    pub fn dispatch_with_value(&self, node: InMemoryNodeId, kind: EventKind, value: &str) -> usize {
        let indices = self.target_path(&node).unwrap_or_default();
        let mut chain: Vec<InMemoryNodeId> = vec![node];
        while let Some(parent) = self.parent(*chain.last().expect("链非空")) {
            chain.push(parent);
        }
        let mut event = crate::events::DomEvent::new(kind, indices).with_target_value(value);
        let mut called = 0;
        for current in chain {
            let handler = self.listeners.borrow().get(&current, kind).cloned();
            if let Some(handler) = handler {
                handler.call(&mut event);
                called += 1;
                if event.is_propagation_stopped() {
                    return called;
                }
            }
        }
        called
    }

    /// 派发事件：自目标节点起，沿祖先链**自下而上**调用处理器。
    ///
    /// 冒泡语义与真实浏览器一致（设计文档 §5.3.3）：目标先执行，祖先后执行；
    /// 任一处理器调用 `stop_propagation()` ⇒ 停止向上传播。
    ///
    /// 返回实际调用次数。
    pub fn dispatch(&self, node: InMemoryNodeId, kind: EventKind) -> usize {
        // 目标路径（从根到目标的子节点下标序列）。
        let indices = self.target_path(&node).unwrap_or_default();

        // 祖先链（目标在前）。
        let mut chain: Vec<InMemoryNodeId> = vec![node];
        while let Some(parent) = self.parent(*chain.last().expect("链非空")) {
            chain.push(parent);
        }

        let mut event = crate::events::DomEvent::new(kind, indices);
        let mut called = 0;
        for current in chain {
            let handler = self.listeners.borrow().get(&current, kind).cloned();
            if let Some(handler) = handler {
                handler.call(&mut event);
                called += 1;
                if event.is_propagation_stopped() {
                    return called;
                }
            }
        }
        called
    }

    /// 节点是否已被摘除（设计文档 §5.5.2 规则 T6 的可判定性）。
    pub fn is_detached(&self, node: InMemoryNodeId) -> bool {
        self.nodes
            .borrow()
            .get(node.0)
            .map(|entry| entry.detached)
            .unwrap_or(false)
    }

    /// 节点当前的父节点（`None` = 未挂到树上，或已被摘除）。
    pub fn parent(&self, node: InMemoryNodeId) -> Option<InMemoryNodeId> {
        self.nodes
            .borrow()
            .get(node.0)
            .and_then(|entry| entry.parent)
    }

    /// 已执行的操作序列（按发生顺序）。
    ///
    /// ⚠️ 日志有**上界**（[`InMemoryDom::set_op_capacity`]）⇒ 超出后**丢弃最旧**
    /// 的记录 ⇒ 机械判据只对**最近一批**有效 ⇒ 断言前先
    /// [`InMemoryDom::clear_ops`]。
    pub fn ops(&self) -> Vec<RecordedOp> {
        self.ops.borrow().iter().cloned().collect()
    }

    /// 清空操作序列（让下一次断言只看「这一批」的操作）。
    pub fn clear_ops(&self) {
        self.ops.borrow_mut().clear();
    }

    /// 追加一个节点记录并返回句柄。
    fn push(&self, kind: InMemoryNodeKind) -> InMemoryNodeId {
        let mut nodes = self.nodes.borrow_mut();
        nodes.push(InMemoryNode {
            kind,
            attrs: Vec::new(),
            props: Vec::new(),
            children: Vec::new(),
            parent: None,
            detached: false,
        });
        InMemoryNodeId(nodes.len() - 1)
    }

    /// 记一次操作（**受上界约束**：超出后丢弃最旧的）。
    fn record(&self, op: DomOp, node: InMemoryNodeId) {
        let capacity = self.op_capacity.get();
        if capacity == 0 {
            return;
        }
        let mut ops = self.ops.borrow_mut();
        ops.push_back(RecordedOp { op, node });
        while ops.len() > capacity {
            ops.pop_front();
        }
    }

    /// 把子树整体标记为已摘除（T6：被移除的节点不参与本批次的写入），
    /// 并**立即释放载荷**（含监听器闭包 ⇒ 闭包捕获的应用状态一起释放）。
    ///
    /// ## 为什么载荷必须**立即**释放（2026-10-25 修）
    ///
    /// 修前摘除只打标记 ⇒ 节点连同 `listeners` 一直留在后端里 ⇒
    /// ⇒ 监听器闭包（捕获着应用状态）**跟着一起泄漏** ⇒
    /// ⇒ 5 轮挂载 / 卸载就把整棵树的闭包留在内存里。
    fn mark_subtree_detached(&self, node: InMemoryNodeId) {
        let mut detached: Vec<InMemoryNodeId> = Vec::new();
        {
            let mut nodes = self.nodes.borrow_mut();
            let mut pending = vec![node];
            while let Some(current) = pending.pop() {
                let Some(entry) = nodes.get_mut(current.0) else {
                    continue;
                };
                entry.detached = true;
                entry.parent = None;
                // 先收孩子，再清孩子（否则递归就断了）。
                pending.extend(entry.children.iter().copied());
                entry.children.clear();
                // 载荷释放：属性 / property 都不再需要。
                entry.attrs.clear();
                entry.props.clear();
                detached.push(current);
            }
        }
        // 监听器在**表**里 ⇒ 逐个摘掉（闭包 ⇒ 释放）。
        let mut listeners = self.listeners.borrow_mut();
        for id in detached {
            listeners.remove_node(&id);
        }
    }
}

impl Dom for InMemoryDom {
    fn focus(&self, node: &InMemoryNodeId) -> Result<(), DomError> {
        self.ensure_node_exists(node)?;
        self.focused.set(Some(*node));
        Ok(())
    }

    fn blur(&self, node: &InMemoryNodeId) -> Result<(), DomError> {
        self.ensure_node_exists(node)?;
        if self.focused.get() == Some(*node) {
            self.focused.set(None);
        }
        Ok(())
    }

    fn focused(&self) -> Option<InMemoryNodeId> {
        self.focused.get()
    }

    fn is_composing(&self) -> bool {
        self.composing.get()
    }

    fn bounding_box(&self, node: &InMemoryNodeId) -> Result<ElementBox, DomError> {
        self.ensure_node_exists(node)?;
        self.measurements
            .borrow()
            .get(node)
            .map(|item| item.bounding_box)
            .ok_or(DomError::NotImplemented)
    }

    fn scroll_metrics(&self, node: &InMemoryNodeId) -> Result<ScrollMetrics, DomError> {
        self.ensure_node_exists(node)?;
        self.measurements
            .borrow()
            .get(node)
            .map(|item| item.scroll_metrics)
            .ok_or(DomError::NotImplemented)
    }

    fn set_scroll_top(&self, node: &InMemoryNodeId, top: f64) -> Result<(), DomError> {
        self.ensure_node_exists(node)?;
        let mut measurements = self.measurements.borrow_mut();
        let item = measurements.get_mut(node).ok_or(DomError::NotImplemented)?;
        item.scroll_metrics.scroll_top = item.scroll_metrics.clamp_scroll_top(top);
        Ok(())
    }

    type Node = InMemoryNodeId;

    fn create_element(&self, tag: &str) -> Result<Self::Node, DomError> {
        let node = self.push(InMemoryNodeKind::Element(tag.to_owned()));
        self.record(
            DomOp::CreateElement {
                tag: tag.to_owned(),
            },
            node,
        );
        Ok(node)
    }

    fn create_text(&self, data: &str) -> Result<Self::Node, DomError> {
        let node = self.push(InMemoryNodeKind::Text(data.to_owned()));
        self.record(
            DomOp::CreateText {
                data: data.to_owned(),
            },
            node,
        );
        Ok(node)
    }

    fn set_attribute(&self, node: &Self::Node, name: &str, value: &str) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        if entry.detached {
            return Err(DomError::NodeDetached);
        }
        if matches!(entry.kind, InMemoryNodeKind::Text(_)) {
            return Err(DomError::Unsupported(String::from(
                "属性只能写在元素节点上",
            )));
        }
        match entry.attrs.iter_mut().find(|(key, _)| key == name) {
            Some(slot) => slot.1 = value.to_owned(),
            None => entry.attrs.push((name.to_owned(), value.to_owned())),
        }
        drop(nodes);
        self.record(
            DomOp::SetAttribute {
                name: name.to_owned(),
                value: value.to_owned(),
            },
            *node,
        );
        Ok(())
    }

    fn remove_attribute(&self, node: &Self::Node, name: &str) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        if entry.detached {
            return Err(DomError::NodeDetached);
        }
        entry.attrs.retain(|(key, _)| key != name);
        drop(nodes);
        self.record(
            DomOp::RemoveAttribute {
                name: name.to_owned(),
            },
            *node,
        );
        Ok(())
    }

    fn set_class_tokens(
        &self,
        node: &Self::Node,
        added: &[String],
        removed: &[String],
    ) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        if entry.detached {
            return Err(DomError::NodeDetached);
        }
        if matches!(entry.kind, InMemoryNodeKind::Text(_)) {
            return Err(DomError::Unsupported(String::from(
                "class 只能设在元素节点上",
            )));
        }
        let current: Vec<String> = entry
            .attrs
            .iter()
            .find(|(key, _)| key == "class")
            .map(|(_, value)| value.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default();
        let mut next: Vec<String> = current
            .into_iter()
            .filter(|token| !removed.contains(token))
            .collect();
        for token in added {
            if !next.contains(token) {
                next.push(token.clone());
            }
        }
        // ⭐ **语义对齐浏览器**（2026-10-25 修）：token 全删之后**保留**
        // `class=""`（浏览器的 `classList` 删完就是空串）⇒
        // ⇒ 修前内存后端把 class 属性**整个删掉** ⇒ 与浏览器不一致
        // ⇒ ⇒ 两侧的属性对照会差一项。
        let value = next.join(" ");
        match entry.attrs.iter_mut().find(|(key, _)| key == "class") {
            Some(slot) => slot.1 = value,
            None => entry.attrs.push((String::from("class"), value)),
        }
        drop(nodes);
        self.record(
            DomOp::SetClassTokens {
                added: added.to_vec(),
                removed: removed.to_vec(),
            },
            *node,
        );
        Ok(())
    }

    fn set_property(&self, node: &Self::Node, name: &str, value: &str) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        if entry.detached {
            return Err(DomError::NodeDetached);
        }
        if matches!(entry.kind, InMemoryNodeKind::Text(_)) {
            return Err(DomError::Unsupported(String::from(
                "property 只能设在元素节点上",
            )));
        }
        let prop = MemoryProp {
            value: value.to_owned(),
            is_bool: false,
        };
        match entry.props.iter_mut().find(|(key, _)| key == name) {
            Some(slot) => slot.1 = prop,
            None => entry.props.push((name.to_owned(), prop)),
        }
        drop(nodes);
        self.record(
            DomOp::SetProperty {
                name: name.to_owned(),
                value: value.to_owned(),
            },
            *node,
        );
        Ok(())
    }

    fn set_property_bool(
        &self,
        node: &Self::Node,
        name: &str,
        value: bool,
    ) -> Result<(), DomError> {
        // 存成 "true"/"false" 只是为了 `get_property`（它返回 `String`）能读回；
        // **记录**用独立的 `SetPropertyBool` ⇒ 通道选择本身可观测。
        let text = if value { "true" } else { "false" };
        {
            let mut nodes = self.nodes.borrow_mut();
            let Some(entry) = nodes.get_mut(node.0) else {
                return Err(DomError::NodeNotFound);
            };
            if entry.detached {
                return Err(DomError::NodeDetached);
            }
            if matches!(entry.kind, InMemoryNodeKind::Text(_)) {
                return Err(DomError::Unsupported(String::from(
                    "property 只能设在元素节点上",
                )));
            }
            let prop = MemoryProp {
                value: text.to_owned(),
                is_bool: true,
            };
            match entry.props.iter_mut().find(|(key, _)| key == name) {
                Some(slot) => slot.1 = prop,
                None => entry.props.push((name.to_owned(), prop)),
            }
        }
        self.record(
            DomOp::SetPropertyBool {
                name: name.to_owned(),
                value,
            },
            *node,
        );
        Ok(())
    }

    fn get_property(&self, node: &Self::Node, name: &str) -> Result<String, DomError> {
        let nodes = self.nodes.borrow();
        let Some(entry) = nodes.get(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        if entry.detached {
            return Err(DomError::NodeDetached);
        }
        // ⭐ **实时语义**（2026-10-25 修）：读的是「**当前**的 property 值」
        // （不是「上一帧虚拟树写的值」）⇒ 与浏览器一致
        // ⇒ ⇒ 用户改过的值会被读到（浏览器的 `input.value` 就是实时的）。
        //
        // ⚠️ 渲染器的跳过条件因此必须以「**目标值是否变化**」为准
        // （见 `renderer::patch_props`），否则会把用户的输入覆盖掉。
        let value = entry
            .props
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.value.clone())
            .unwrap_or_default();
        drop(nodes);
        self.record(
            DomOp::GetProperty {
                name: name.to_owned(),
            },
            *node,
        );
        Ok(value)
    }

    fn remove_property(&self, node: &Self::Node, name: &str) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        if entry.detached {
            return Err(DomError::NodeDetached);
        }
        if matches!(entry.kind, InMemoryNodeKind::Text(_)) {
            return Err(DomError::Unsupported(String::from(
                "property 只能设在元素节点上",
            )));
        }
        // ⭐ **回落到默认值**（浏览器语义）：字符串 ⇒ 空串；布尔 ⇒ `false`。
        //
        // 为什么不是「删掉这条记录」：删掉之后读回来是空串 ⇒ ⇒ 布尔 property
        // 就与浏览器（`false`）**不一致** ⇒ ⇒ 两侧语义必须一致（0 装）。
        match entry.props.iter_mut().find(|(key, _)| key == name) {
            Some(slot) => {
                let is_bool = slot.1.is_bool;
                slot.1.value = MemoryProp::default_value(is_bool);
            }
            None => {
                // 从没写过 ⇒ 回落成默认值（字符串语义）。
                entry.props.push((
                    name.to_owned(),
                    MemoryProp {
                        value: MemoryProp::default_value(false),
                        is_bool: false,
                    },
                ));
            }
        }
        drop(nodes);
        self.record(
            DomOp::RemoveProperty {
                name: name.to_owned(),
            },
            *node,
        );
        Ok(())
    }

    fn set_text(&self, node: &Self::Node, data: &str) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        let Some(entry) = nodes.get_mut(node.0) else {
            return Err(DomError::NodeNotFound);
        };
        if entry.detached {
            return Err(DomError::NodeDetached);
        }
        match &mut entry.kind {
            InMemoryNodeKind::Text(text) => {
                *text = data.to_owned();
            }
            InMemoryNodeKind::Element(_) => {
                return Err(DomError::Unsupported(String::from(
                    "set_text 只适用于文本节点",
                )))
            }
        }
        drop(nodes);
        self.record(
            DomOp::SetText {
                data: data.to_owned(),
            },
            *node,
        );
        Ok(())
    }

    fn append_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError> {
        self.insert_before(parent, child, None)
    }

    fn insert_before(
        &self,
        parent: &Self::Node,
        child: &Self::Node,
        anchor: Option<&Self::Node>,
    ) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        // 先判子节点（不可变读），再动父节点（可变写）——避免同一借用的重叠。
        match nodes.get(child.0) {
            None => return Err(DomError::NodeNotFound),
            Some(entry) if entry.detached => return Err(DomError::NodeDetached),
            Some(_) => {}
        }
        match nodes.get(parent.0) {
            None => return Err(DomError::NodeNotFound),
            Some(entry) if entry.detached => return Err(DomError::NodeDetached),
            Some(_) => {}
        }
        if parent.0 == child.0 {
            return Err(DomError::Unsupported(String::from(
                "节点不能成为自己的子节点",
            )));
        }

        // 从原父节点摘下（真实 DOM 的 insertBefore 语义：移动而非复制）。
        let previous_parent = nodes.get(child.0).and_then(|entry| entry.parent);
        if let Some(previous) = previous_parent {
            if let Some(previous_entry) = nodes.get_mut(previous.0) {
                previous_entry.children.retain(|node| node != child);
            }
        }

        // 定位锚点位置。
        let position = {
            let parent_entry = nodes.get(parent.0).ok_or(DomError::NodeNotFound)?;
            match anchor {
                None => parent_entry.children.len(),
                Some(anchor) => parent_entry
                    .children
                    .iter()
                    .position(|node| node == anchor)
                    .ok_or(DomError::NodeNotFound)?,
            }
        };

        {
            let parent_entry = nodes.get_mut(parent.0).ok_or(DomError::NodeNotFound)?;
            parent_entry.children.insert(position, *child);
        }
        if let Some(child_entry) = nodes.get_mut(child.0) {
            child_entry.parent = Some(*parent);
        }
        drop(nodes);

        let op = match anchor {
            None => DomOp::AppendChild { parent: *parent },
            Some(anchor) => DomOp::InsertBefore {
                parent: *parent,
                anchor: Some(*anchor),
            },
        };
        self.record(op, *child);
        Ok(())
    }

    fn remove_child(&self, parent: &Self::Node, child: &Self::Node) -> Result<(), DomError> {
        let mut nodes = self.nodes.borrow_mut();
        if nodes.get(child.0).is_none() {
            return Err(DomError::NodeNotFound);
        }
        {
            let Some(parent_entry) = nodes.get_mut(parent.0) else {
                return Err(DomError::NodeNotFound);
            };
            if parent_entry.detached {
                return Err(DomError::NodeDetached);
            }
            let position = parent_entry
                .children
                .iter()
                .position(|node| node == child)
                .ok_or(DomError::NodeNotFound)?;
            parent_entry.children.remove(position);
        }
        drop(nodes);
        // 摘除是终态：整棵子树标记为已脱离（T6：被移除的节点不参与本批次写入）。
        self.mark_subtree_detached(*child);
        self.record(DomOp::RemoveChild { parent: *parent }, *child);
        Ok(())
    }

    fn children(&self, parent: &Self::Node) -> Result<Vec<Self::Node>, DomError> {
        let nodes = self.nodes.borrow();
        let Some(entry) = nodes.get(parent.0) else {
            return Err(DomError::NodeNotFound);
        };
        Ok(entry.children.clone())
    }

    fn add_listener(
        &self,
        node: &Self::Node,
        kind: EventKind,
        handler: crate::events::ElementHandler,
    ) -> Result<(), DomError> {
        {
            let nodes = self.nodes.borrow();
            let Some(entry) = nodes.get(node.0) else {
                return Err(DomError::NodeNotFound);
            };
            if entry.detached {
                return Err(DomError::NodeDetached);
            }
        }
        // 覆盖语义：同 (节点, kind) 至多一条。
        self.listeners.borrow_mut().insert(*node, kind, handler);
        self.record(DomOp::AddListener { kind }, *node);
        Ok(())
    }

    fn remove_listeners(&self, node: &Self::Node) -> Result<(), DomError> {
        {
            let nodes = self.nodes.borrow();
            if nodes.get(node.0).is_none() {
                return Err(DomError::NodeNotFound);
            }
        }
        // 摘掉该节点的**全部** kind（闭包 ⇒ 释放）。
        self.listeners.borrow_mut().remove_node(node);
        self.record(DomOp::RemoveListeners, *node);
        Ok(())
    }

    fn remove_listener(&self, node: &Self::Node, kind: EventKind) -> Result<(), DomError> {
        {
            let nodes = self.nodes.borrow();
            if nodes.get(node.0).is_none() {
                return Err(DomError::NodeNotFound);
            }
        }
        // ⭐ **只摘这一个 kind** ⇒ 同元素的其它 kind **不受影响**（既有契约）。
        self.listeners.borrow_mut().remove_kind(node, kind);
        self.record(DomOp::RemoveListener { kind }, *node);
        Ok(())
    }

    fn target_path(&self, node: &Self::Node) -> Result<Vec<usize>, DomError> {
        let nodes = self.nodes.borrow();
        if nodes.get(node.0).is_none() {
            return Err(DomError::NodeNotFound);
        }
        drop(nodes);
        // 从根到目标的子节点下标序列（只数元素 / 文本 ⇒ 与浏览器口径一致）。
        let mut indices: Vec<usize> = Vec::new();
        let mut current = *node;
        while let Some(parent) = self.parent(current) {
            let siblings = self
                .nodes
                .borrow()
                .get(parent.0)
                .map(|entry| entry.children.clone())
                .unwrap_or_default();
            let index = siblings
                .iter()
                .position(|child| *child == current)
                .unwrap_or_default();
            indices.push(index);
            current = parent;
        }
        indices.reverse();
        Ok(indices)
    }
}

/// **未接线**的 DOM 后端节点句柄（占位；没有任何实例能被造出来）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnavailableNode(pub usize);

/// **未接线**的 DOM 后端（P1 桩）。
///
/// 诚实口径：所有操作都**显式报错**（[`DomError::NotImplemented`]），
/// **不假装**「已经在往真实 DOM 写东西」。
///
/// 先例：`ui-gpu` 的 `UnavailableGpuContext`（`GpuError::NotImplemented`）。
#[derive(Debug, Clone, Copy, Default)]
pub struct UnavailableDom;

impl Dom for UnavailableDom {
    type Node = UnavailableNode;

    fn create_element(&self, _tag: &str) -> Result<Self::Node, DomError> {
        Err(DomError::NotImplemented)
    }

    fn create_text(&self, _data: &str) -> Result<Self::Node, DomError> {
        Err(DomError::NotImplemented)
    }

    fn set_attribute(&self, _node: &Self::Node, _name: &str, _value: &str) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn remove_attribute(&self, _node: &Self::Node, _name: &str) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn set_property(&self, _node: &Self::Node, _name: &str, _value: &str) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn set_class_tokens(
        &self,
        _node: &Self::Node,
        _added: &[String],
        _removed: &[String],
    ) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn get_property(&self, _node: &Self::Node, _name: &str) -> Result<String, DomError> {
        Err(DomError::NotImplemented)
    }

    fn set_text(&self, _node: &Self::Node, _data: &str) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn append_child(&self, _parent: &Self::Node, _child: &Self::Node) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn insert_before(
        &self,
        _parent: &Self::Node,
        _child: &Self::Node,
        _anchor: Option<&Self::Node>,
    ) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn remove_child(&self, _parent: &Self::Node, _child: &Self::Node) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn children(&self, _parent: &Self::Node) -> Result<Vec<Self::Node>, DomError> {
        Err(DomError::NotImplemented)
    }

    fn add_listener(
        &self,
        _node: &Self::Node,
        _kind: EventKind,
        _handler: crate::events::ElementHandler,
    ) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn remove_listeners(&self, _node: &Self::Node) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn remove_property(&self, _node: &Self::Node, _name: &str) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn remove_listener(&self, _node: &Self::Node, _kind: EventKind) -> Result<(), DomError> {
        Err(DomError::NotImplemented)
    }

    fn target_path(&self, _node: &Self::Node) -> Result<Vec<usize>, DomError> {
        Err(DomError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {

    // ── G6 缩放/密度标量（台账 L1-G6）────────────────────────────────────

    /// 恒等乘数不改变数值。
    #[test]
    fn identity_scale_does_not_change_values() {
        let scale = MeasurementScale::IDENTITY;
        assert_eq!(scale.css_to_device(100.0), 100.0);
        assert_eq!(scale.device_to_css(100.0), 100.0);
    }

    /// DPR 与密度**相乘**（2.0 × 1.25 = 2.5），且双向互逆。
    #[test]
    fn scale_multiplies_dpr_and_density_and_is_invertible() {
        let scale = MeasurementScale::new(2.0, 1.25);
        assert_eq!(scale.device_pixel_ratio(), 2.0);
        assert_eq!(scale.density(), 1.25);
        assert_eq!(scale.css_to_device(40.0), 100.0, "40 CSS × 2.5 = 100 设备");
        assert_eq!(scale.device_to_css(100.0), 40.0, "必须可逆");
    }

    /// ⭐ **非法乘数不得静默变成 0 尺寸**（0/负数/NaN/∞ ⇒ 按 1.0 处理）。
    #[test]
    fn illegal_scale_falls_back_to_one_instead_of_zero() {
        for bad in [0.0, -2.0, f64::NAN, f64::INFINITY] {
            let scale = MeasurementScale::new(bad, bad);
            assert_eq!(scale.device_pixel_ratio(), 1.0, "非法 dpr {bad} 应回落 1.0");
            assert_eq!(scale.density(), 1.0, "非法 density {bad} 应回落 1.0");
            assert!(scale.css_to_device(10.0) > 0.0, "不得把尺寸压成 0");
        }
    }

    /// 元素盒整体换算。
    #[test]
    fn element_box_scales_to_device_pixels() {
        let scale = MeasurementScale::new(2.0, 1.0);
        let device = scale.box_to_device(ElementBox::new(10.0, 20.0, 30.0, 40.0));
        assert_eq!(device, ElementBox::new(20.0, 40.0, 60.0, 80.0));
    }

    // ── G1 布局测量（台账 L1-G1）──────────────────────────────────────────

    /// **未注入脚本化测量 ⇒ 显式不支持**（不是静默返回 0）。
    ///
    /// 0 装纪律：引擎不做布局（ADR-0015）⇒ ⇒ 内存后端不能假装知道几何
    /// ⇒ ⇒ 三个方法必须返回 [`DomError::NotImplemented`]，让上层立刻发现「没接线」。
    #[test]
    fn measurement_without_scripting_is_explicitly_unimplemented() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("div").expect("建元素必须成功");
        assert!(matches!(
            dom.bounding_box(&node),
            Err(DomError::NotImplemented)
        ));
        assert!(matches!(
            dom.scroll_metrics(&node),
            Err(DomError::NotImplemented)
        ));
        assert!(matches!(
            dom.set_scroll_top(&node, 10.0),
            Err(DomError::NotImplemented)
        ));
    }

    /// **脚本化测量可注入可读回**，且 `set_scroll_top` 会**夹到**合法区间。
    #[test]
    fn scripted_measurement_round_trips_and_clamps_scroll() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("div").expect("建元素必须成功");
        dom.script_measurement(
            node,
            ElementBox::new(12.0, 34.0, 200.0, 100.0),
            ScrollMetrics {
                scroll_top: 0.0,
                scroll_height: 1000.0,
                client_height: 300.0,
            },
        );
        assert_eq!(
            dom.bounding_box(&node),
            Ok(ElementBox::new(12.0, 34.0, 200.0, 100.0))
        );
        let metrics = dom.scroll_metrics(&node).expect("度量必须可读");
        assert_eq!(metrics.max_scroll_top(), 700.0);
        // 超过上界 ⇒ 夹到 700；负值 ⇒ 夹到 0。
        dom.set_scroll_top(&node, 9999.0).expect("设置滚动必须成功");
        assert_eq!(dom.scroll_metrics(&node).expect("可读").scroll_top, 700.0);
        dom.set_scroll_top(&node, -5.0).expect("设置滚动必须成功");
        assert_eq!(dom.scroll_metrics(&node).expect("可读").scroll_top, 0.0);
    }

    /// 元素盒的几何辅助（命中测试的最基本形式）。
    #[test]
    fn element_box_geometry_helpers() {
        let rect = ElementBox::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(rect.right(), 110.0);
        assert_eq!(rect.bottom(), 70.0);
        assert!(rect.contains(10.0, 20.0), "左上角算在内");
        assert!(rect.contains(109.9, 69.9));
        assert!(
            !rect.contains(110.0, 20.0),
            "右边界**不**算在内（半开区间）"
        );
        assert!(!rect.contains(10.0, 70.0), "下边界**不**算在内");
        assert!(!rect.contains(9.9, 20.0));
    }

    /// 内容不足一屏 ⇒ 最大滚动量为 0（不会出现负的可滚动距离）。
    #[test]
    fn scroll_metrics_never_report_negative_range() {
        let metrics = ScrollMetrics {
            scroll_top: 0.0,
            scroll_height: 120.0,
            client_height: 300.0,
        };
        assert_eq!(metrics.max_scroll_top(), 0.0);
        assert_eq!(metrics.clamp_scroll_top(50.0), 0.0);
    }

    /// 对不存在的节点做测量 ⇒ `NodeNotFound`（与「未接线」区分开）。
    #[test]
    fn measuring_a_missing_node_is_not_found() {
        let dom = InMemoryDom::new();
        let _existing = dom.create_element("div").expect("建元素必须成功");
        let mut other = dom.create_element("span").expect("建元素必须成功");
        other = InMemoryNodeId(other.0 + 1000);
        assert!(matches!(
            dom.bounding_box(&other),
            Err(DomError::NodeNotFound)
        ));
    }
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    // ── 事件派发（内存后端的冒泡语义） ──────────────────────────────────

    /// 建一个带处理器的按钮节点（挂在 `parent` 下）。
    fn button_with_handler(
        dom: &InMemoryDom,
        parent: InMemoryNodeId,
        kind: EventKind,
        handler: crate::events::ElementHandler,
    ) -> InMemoryNodeId {
        let node = dom.create_element("button").expect("建节点必须成功");
        dom.add_listener(&node, kind, handler)
            .expect("挂处理器必须成功");
        dom.append_child(&parent, &node).expect("挂载必须成功");
        node
    }

    #[test]
    fn dispatch_calls_handlers_from_the_target_up_to_the_ancestors() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let middle = dom.create_element("div").expect("建节点必须成功");
        dom.append_child(&root, &middle).expect("挂载必须成功");

        let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let root_order = order.clone();
        dom.add_listener(
            &root,
            EventKind::Click,
            crate::events::ElementHandler::new(move |_event| {
                root_order.borrow_mut().push("root");
            }),
        )
        .expect("挂处理器必须成功");
        let target_order = order.clone();
        let target = button_with_handler(
            &dom,
            middle,
            EventKind::Click,
            crate::events::ElementHandler::new(move |_event| {
                target_order.borrow_mut().push("target");
            }),
        );

        assert_eq!(dom.dispatch(target, EventKind::Click), 2);
        assert_eq!(&*order.borrow(), &["target", "root"], "目标先、祖先后");
    }

    /// **反例**：处理器调用 `stop_propagation()` ⇒ 祖先后不再被调用。
    #[test]
    fn dispatch_stops_at_the_handler_that_asks_for_it() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let middle = dom.create_element("div").expect("建节点必须成功");
        dom.append_child(&root, &middle).expect("挂载必须成功");

        let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let root_order = order.clone();
        dom.add_listener(
            &root,
            EventKind::Click,
            crate::events::ElementHandler::new(move |_event| {
                root_order.borrow_mut().push("root");
            }),
        )
        .expect("挂处理器必须成功");
        let target_order = order.clone();
        let target = button_with_handler(
            &dom,
            middle,
            EventKind::Click,
            crate::events::ElementHandler::new(move |event| {
                target_order.borrow_mut().push("target");
                event.stop_propagation();
            }),
        );

        assert_eq!(dom.dispatch(target, EventKind::Click), 1);
        assert_eq!(&*order.borrow(), &["target"]);
    }

    /// **不误报**：另一类事件不会被同类处理器捕获。
    #[test]
    fn dispatch_only_calls_handlers_of_the_same_event_kind() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let clicked = Rc::new(Cell::new(0u32));
        let counter = clicked.clone();
        let node = button_with_handler(
            &dom,
            root,
            EventKind::Click,
            crate::events::ElementHandler::new(move |_event| {
                counter.set(counter.get() + 1);
            }),
        );

        assert_eq!(dom.dispatch(node, EventKind::KeyDown), 0);
        assert_eq!(clicked.get(), 0);
        assert_eq!(dom.dispatch(node, EventKind::Click), 1);
        assert_eq!(clicked.get(), 1);
    }

    #[test]
    fn remove_listeners_clears_the_handlers_of_that_node() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let hits = Rc::new(Cell::new(0u32));
        let counter = hits.clone();
        let node = button_with_handler(
            &dom,
            root,
            EventKind::Click,
            crate::events::ElementHandler::new(move |_event| {
                counter.set(counter.get() + 1);
            }),
        );

        assert_eq!(dom.dispatch(node, EventKind::Click), 1);
        dom.remove_listeners(&node).expect("摘处理器必须成功");
        assert_eq!(dom.dispatch(node, EventKind::Click), 0);
        assert_eq!(hits.get(), 1, "摘掉之后不再触发");
    }

    #[test]
    fn memory_backend_builds_a_readable_tree() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("div").expect("建节点必须成功");
        let text = dom.create_text("你好").expect("建节点必须成功");
        dom.append_child(&root, &text).expect("挂载必须成功");
        dom.set_attribute(&root, "class", "ap-root")
            .expect("写属性必须成功");

        assert_eq!(dom.tag(root), Some(String::from("div")));
        assert_eq!(dom.attribute(root, "class"), Some(String::from("ap-root")));
        assert_eq!(dom.text_content(root), "你好");
        assert_eq!(dom.children(&root).expect("读子节点必须成功"), vec![text]);
    }

    #[test]
    fn attribute_removal_and_text_rewrite_are_in_place() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("input").expect("建节点必须成功");
        dom.set_attribute(&node, "value", "a")
            .expect("写属性必须成功");
        assert_eq!(dom.attribute(node, "value"), Some(String::from("a")));
        dom.set_attribute(&node, "value", "b")
            .expect("写属性必须成功");
        assert_eq!(dom.attribute(node, "value"), Some(String::from("b")));
        dom.remove_attribute(&node, "value")
            .expect("删属性必须成功");
        assert_eq!(dom.attribute(node, "value"), None);

        let text = dom.create_text("x").expect("建节点必须成功");
        dom.set_text(&text, "y").expect("写文本必须成功");
        assert_eq!(dom.text_content(text), "y");
        assert_eq!(dom.node_count(), 2, "改文本不得重建节点");
    }

    #[test]
    fn removing_a_child_detaches_it_from_the_parent() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("ul").expect("建节点必须成功");
        let item = dom.create_element("li").expect("建节点必须成功");
        dom.append_child(&root, &item).expect("挂载必须成功");
        assert_eq!(dom.children(&root).expect("读子节点必须成功").len(), 1);

        dom.remove_child(&root, &item).expect("摘除必须成功");
        assert!(dom.children(&root).expect("读子节点必须成功").is_empty());
        assert!(dom.is_detached(item), "摘除后节点被标记为已脱离");
        assert_eq!(dom.parent(item), None, "摘除后不再有父节点");
    }

    // ── P2 新增：操作序列记录 ─────────────────────────────────────────────

    #[test]
    fn recorded_operations_follow_the_execution_order() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("div").expect("建节点必须成功");
        let child = dom.create_element("span").expect("建节点必须成功");
        dom.set_attribute(&child, "class", "ap-x")
            .expect("写属性必须成功");
        dom.append_child(&root, &child).expect("挂载必须成功");

        let ops = dom.ops();
        assert_eq!(
            ops,
            vec![
                RecordedOp {
                    op: DomOp::CreateElement {
                        tag: String::from("div")
                    },
                    node: root
                },
                RecordedOp {
                    op: DomOp::CreateElement {
                        tag: String::from("span")
                    },
                    node: child
                },
                RecordedOp {
                    op: DomOp::SetAttribute {
                        name: String::from("class"),
                        value: String::from("ap-x")
                    },
                    node: child
                },
                RecordedOp {
                    op: DomOp::AppendChild { parent: root },
                    node: child
                },
            ]
        );
    }

    #[test]
    fn clearing_the_operation_log_keeps_the_tree_intact() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("div").expect("建节点必须成功");
        dom.clear_ops();
        assert!(dom.ops().is_empty());
        assert_eq!(dom.tag(root), Some(String::from("div")), "树不受影响");
    }

    // ── P2 新增：property 与插入语义 ──────────────────────────────────────

    #[test]
    fn properties_are_stored_separately_from_attributes() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("input").expect("建节点必须成功");
        dom.set_property(&node, "value", "abc")
            .expect("写 property 必须成功");
        dom.set_attribute(&node, "value", "abc")
            .expect("写属性必须成功");

        assert_eq!(dom.property(node, "value"), Some(String::from("abc")));
        assert_eq!(dom.attribute(node, "value"), Some(String::from("abc")));
        assert_eq!(
            dom.get_property(&node, "value")
                .expect("读 property 必须成功"),
            String::from("abc")
        );
    }

    #[test]
    fn reading_an_unset_property_yields_an_empty_string() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("input").expect("建节点必须成功");
        assert_eq!(
            dom.get_property(&node, "checked")
                .expect("读 property 必须成功"),
            String::new()
        );
    }

    #[test]
    fn insert_before_places_the_child_at_the_anchor_position() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("ul").expect("建节点必须成功");
        let first = dom.create_element("li").expect("建节点必须成功");
        let second = dom.create_element("li").expect("建节点必须成功");
        let inserted = dom.create_element("li").expect("建节点必须成功");
        dom.append_child(&root, &first).expect("挂载必须成功");
        dom.append_child(&root, &second).expect("挂载必须成功");

        dom.insert_before(&root, &inserted, Some(&first))
            .expect("插入必须成功");

        assert_eq!(
            dom.children(&root).expect("读子节点必须成功"),
            vec![inserted, first, second]
        );
    }

    #[test]
    fn insert_before_with_no_anchor_appends_to_the_end() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("ul").expect("建节点必须成功");
        let first = dom.create_element("li").expect("建节点必须成功");
        let last = dom.create_element("li").expect("建节点必须成功");
        dom.append_child(&root, &first).expect("挂载必须成功");

        dom.insert_before(&root, &last, None).expect("插入必须成功");

        assert_eq!(
            dom.children(&root).expect("读子节点必须成功"),
            vec![first, last]
        );
    }

    #[test]
    fn moving_a_node_between_parents_detaches_it_from_the_old_one() {
        let dom = InMemoryDom::new();
        let old_parent = dom.create_element("ul").expect("建节点必须成功");
        let new_parent = dom.create_element("ol").expect("建节点必须成功");
        let item = dom.create_element("li").expect("建节点必须成功");
        dom.append_child(&old_parent, &item).expect("挂载必须成功");

        dom.append_child(&new_parent, &item).expect("移动必须成功");

        assert!(dom
            .children(&old_parent)
            .expect("读子节点必须成功")
            .is_empty());
        assert_eq!(
            dom.children(&new_parent).expect("读子节点必须成功"),
            vec![item]
        );
        assert_eq!(dom.parent(item), Some(new_parent));
        assert!(
            !dom.is_detached(item),
            "移动不是摘除：节点仍在树上，不该被标记为脱离"
        );
    }

    #[test]
    fn inserting_before_a_missing_anchor_is_reported_as_not_found() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("ul").expect("建节点必须成功");
        let child = dom.create_element("li").expect("建节点必须成功");
        let ghost = InMemoryNodeId(999);

        assert_eq!(
            dom.insert_before(&root, &child, Some(&ghost))
                .expect_err("锚点不存在必须报错"),
            DomError::NodeNotFound
        );
    }

    // ── 判别测试：失败必须显式（不许静默降级）──────────────────────────

    /// 写入**已摘除**的节点必须报错（设计文档 §5.5.2 规则 T6 的可判定性）。
    #[test]
    fn writing_to_a_removed_node_is_reported_as_detached() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("ul").expect("建节点必须成功");
        let item = dom.create_element("li").expect("建节点必须成功");
        dom.append_child(&root, &item).expect("挂载必须成功");
        dom.remove_child(&root, &item).expect("摘除必须成功");

        let err = dom
            .set_attribute(&item, "class", "ap-row")
            .expect_err("写入已摘除节点必须报错");
        assert_eq!(err, DomError::NodeDetached);
        assert_eq!(dom.attribute(item, "class"), None, "被拒绝的写不得留下痕迹");
    }

    /// 挂回**已摘除**的节点必须报错（摘除是终态）。
    #[test]
    fn reattaching_a_removed_node_is_refused() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("ul").expect("建节点必须成功");
        let item = dom.create_element("li").expect("建节点必须成功");
        dom.append_child(&root, &item).expect("挂载必须成功");
        dom.remove_child(&root, &item).expect("摘除必须成功");

        let err = dom
            .append_child(&root, &item)
            .expect_err("重新挂回已摘除节点必须报错");
        assert_eq!(err, DomError::NodeDetached);
    }

    /// 不存在的节点必须报错，不得静默吞掉。
    #[test]
    fn operating_on_a_missing_node_is_reported_as_not_found() {
        let dom = InMemoryDom::new();
        let ghost = InMemoryNodeId(999);
        assert_eq!(
            dom.set_attribute(&ghost, "class", "x")
                .expect_err("不存在的节点必须报错"),
            DomError::NodeNotFound
        );
        assert_eq!(
            dom.children(&ghost).expect_err("不存在的节点必须报错"),
            DomError::NodeNotFound
        );
        assert_eq!(
            dom.get_property(&ghost, "value")
                .expect_err("不存在的节点必须报错"),
            DomError::NodeNotFound
        );
    }

    /// 不适用的操作必须报错（属性写到文本节点上）。
    #[test]
    fn unsupported_operations_are_reported_explicitly() {
        let dom = InMemoryDom::new();
        let text = dom.create_text("x").expect("建节点必须成功");
        let err = dom
            .set_attribute(&text, "class", "x")
            .expect_err("文本节点不接受属性");
        assert!(matches!(err, DomError::Unsupported(_)), "实测：{err:?}");

        let err = dom
            .set_property(&text, "value", "x")
            .expect_err("文本节点不接受 property");
        assert!(matches!(err, DomError::Unsupported(_)), "实测：{err:?}");
    }

    /// 未接线的后端：**每个**操作都显式报 `NotImplemented`。
    #[test]
    fn unavailable_backend_reports_not_implemented_for_every_operation() {
        let dom = UnavailableDom;
        let node = UnavailableNode(0);
        assert_eq!(
            dom.create_element("div").expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.create_text("x").expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.set_attribute(&node, "class", "x")
                .expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.remove_attribute(&node, "class")
                .expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.set_property(&node, "value", "x")
                .expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.get_property(&node, "value")
                .expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.set_text(&node, "x").expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.append_child(&node, &node)
                .expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.insert_before(&node, &node, None)
                .expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.remove_child(&node, &node)
                .expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
        assert_eq!(
            dom.children(&node).expect_err("未接线后端必须报错"),
            DomError::NotImplemented
        );
    }

    /// 错误消息必须可读（0 装纪律：报错要能定位）。
    #[test]
    fn dom_errors_render_readable_messages() {
        assert_eq!(
            DomError::NodeNotFound.to_string(),
            "NodeNotFound: 节点不存在"
        );
        assert_eq!(
            DomError::NodeDetached.to_string(),
            "NodeDetached: 节点已摘除"
        );
        assert!(DomError::NotImplemented
            .to_string()
            .contains("NotImplemented"));
        assert_eq!(
            DomError::Unsupported(String::from("x")).to_string(),
            "Unsupported: x"
        );
    }

    // ── 摘除 ⇒ **立即释放载荷**（2026-10-25 修） ───────────────────────────

    /// ⭐ **摘除子树 ⇒ 监听器闭包一起释放**（闭包捕获的应用状态不再泄漏）。
    ///
    /// ## 缺陷现象（实测）
    ///
    /// 修前：节点只打「已摘除」标记 ⇒ 记录（含 `listeners`）一直留着 ⇒
    /// ⇒ 5 轮挂载 / 卸载后节点全留着 ⇒ ⇒ 闭包捕获的应用状态**一起泄漏**。
    ///
    /// ## 判据
    ///
    /// 处理器捕获的 `Rc` 的**强引用计数**在摘除后**回落** ⇒ 闭包被释放 ✓。
    #[test]
    fn detaching_a_subtree_releases_its_listeners() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let captured = Rc::new(Cell::new(0u32));
        let node = dom.create_element("button").expect("建节点必须成功");
        let handler = crate::events::ElementHandler::new({
            let captured = captured.clone();
            move |_event| {
                captured.set(captured.get() + 1);
            }
        });
        dom.add_listener(&node, EventKind::Click, handler)
            .expect("挂处理器必须成功");
        dom.append_child(&root, &node).expect("挂载必须成功");
        assert_eq!(
            Rc::strong_count(&captured),
            2,
            "处理器捕获着它（测试自己一份 + 闭包一份）"
        );

        dom.remove_child(&root, &node).expect("摘除必须成功");
        assert_eq!(
            Rc::strong_count(&captured),
            1,
            "摘除 ⇒ 监听器闭包**必须**被释放（不许泄漏应用状态）"
        );
    }

    /// **摘除子树 ⇒ 子孙的监听器也一起释放**（不只是顶层）。
    #[test]
    fn detaching_a_subtree_releases_the_listeners_of_its_descendants() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let branch = dom.create_element("div").expect("建节点必须成功");
        let leaf = dom.create_element("button").expect("建节点必须成功");
        let captured = Rc::new(Cell::new(0u32));
        let handler = crate::events::ElementHandler::new({
            let captured = captured.clone();
            move |_event| {
                captured.set(captured.get() + 1);
            }
        });
        dom.add_listener(&leaf, EventKind::Click, handler)
            .expect("挂处理器必须成功");
        dom.append_child(&root, &branch).expect("挂载必须成功");
        dom.append_child(&branch, &leaf).expect("挂载必须成功");
        assert_eq!(Rc::strong_count(&captured), 2);

        dom.remove_child(&root, &branch).expect("摘除必须成功");
        assert_eq!(Rc::strong_count(&captured), 1, "子孙的监听器也必须被释放");
    }

    /// **摘除 ⇒ 整棵子树都被标记为已摘除**（T6 的可判定性 ⇒ 与浏览器同口径）。
    ///
    /// ## 为什么
    ///
    /// 真实 DOM 后端修前**只标记顶层节点** ⇒ 子节点仍可被写入 ⇒
    /// ⇒ 规则 T6 在浏览器里是**假的** ⇒ ⇒ 两侧口径必须一致。
    #[test]
    fn detaching_a_node_marks_its_whole_subtree_as_detached() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let branch = dom.create_element("div").expect("建节点必须成功");
        let leaf = dom.create_element("span").expect("建节点必须成功");
        dom.append_child(&root, &branch).expect("挂载必须成功");
        dom.append_child(&branch, &leaf).expect("挂载必须成功");

        dom.remove_child(&root, &branch).expect("摘除必须成功");
        assert!(dom.is_detached(branch));
        assert!(
            dom.is_detached(leaf),
            "子孙也必须被标记（**不许**只标记顶层 ⇒ T6 在浏览器里会形同虚设）"
        );
        assert_eq!(
            dom.set_attribute(&leaf, "class", "x")
                .expect_err("写入已摘除节点必须报错"),
            DomError::NodeDetached
        );
    }

    /// **摘除 ⇒ 载荷立即释放**（属性 / property 都不再留着）。
    ///
    /// ## 为什么
    ///
    /// 摘除的节点**再也不会被读**（引擎的树里已经没有它）⇒ ⇒ 留着属性 /
    /// property 只是**白占内存**⇒ ⇒ 与监听器一样**立即释放**。
    #[test]
    fn detaching_a_subtree_releases_its_payload() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let item = dom.create_element("li").expect("建节点必须成功");
        dom.set_attribute(&item, "class", "ap-row")
            .expect("写属性必须成功");
        dom.set_property(&item, "value", "x")
            .expect("写 property 必须成功");
        dom.append_child(&root, &item).expect("挂载必须成功");

        dom.remove_child(&root, &item).expect("摘除必须成功");
        assert_eq!(
            dom.attribute(item, "class"),
            None,
            "摘除 ⇒ 载荷释放（属性不留）"
        );
        assert_eq!(
            dom.property(item, "value"),
            None,
            "摘除 ⇒ 载荷释放（property 不留）"
        );
        assert!(dom.is_detached(item), "摘除的**判定**必须仍在");
    }

    /// **摘除 ⇒ 节点不再算「活着的」**（`node_count` 反映真实树的大小）。
    #[test]
    fn detached_nodes_do_not_count_as_live_nodes() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let item = dom.create_element("li").expect("建节点必须成功");
        dom.append_child(&root, &item).expect("挂载必须成功");
        assert_eq!(dom.node_count(), 2);

        dom.remove_child(&root, &item).expect("摘除必须成功");
        assert_eq!(dom.node_count(), 1, "摘除 ⇒ 不再算活着的节点");
        assert_eq!(dom.slot_count(), 2, "槽位保留（句柄必须仍能判定为已摘除）");
        assert!(dom.is_detached(item), "摘除的节点仍可判定");
    }

    /// **槽位只增不复用** ⇒ 但可以显式**压缩**尾部的墓碑。
    #[test]
    fn the_tail_slots_can_be_compacted_away() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let item = dom.create_element("li").expect("建节点必须成功");
        dom.append_child(&root, &item).expect("挂载必须成功");
        dom.remove_child(&root, &item).expect("摘除必须成功");
        assert_eq!(dom.slot_count(), 2);

        assert_eq!(dom.compact(), 1, "尾部墓碑可回收");
        assert_eq!(dom.slot_count(), 1, "只回收了尾部");
        assert_eq!(dom.node_count(), 1, "活着的节点不受影响");
    }

    /// **操作日志有上界** ⇒ 长时间运行不会把内存吃光。
    #[test]
    fn the_operation_log_has_an_upper_bound() {
        let dom = InMemoryDom::new();
        dom.set_op_capacity(8);
        for _ in 0..50 {
            dom.create_element("div").expect("建节点必须成功");
        }
        assert!(
            dom.ops().len() <= 8,
            "日志不许无界增长：实测 {} 条",
            dom.ops().len()
        );
        // 最新的记录**必须**还在（丢的是最旧的）。
        assert!(matches!(
            dom.ops().last().map(|recorded| &recorded.op),
            Some(DomOp::CreateElement { .. })
        ));
    }

    // ── 与浏览器**同语义**（2026-10-25 修） ────────────────────────────────

    /// ⭐ **class token 全删之后留 `class=""`**（浏览器的 `classList` 语义）。
    #[test]
    fn removing_every_class_token_keeps_an_empty_class_attribute() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("div").expect("建节点必须成功");
        dom.set_class_tokens(&node, &[String::from("ap-a")], &[])
            .expect("加 token 必须成功");
        assert_eq!(dom.attribute(node, "class"), Some(String::from("ap-a")));

        dom.set_class_tokens(&node, &[], &[String::from("ap-a")])
            .expect("删 token 必须成功");
        assert_eq!(
            dom.attribute(node, "class"),
            Some(String::new()),
            "与浏览器一致：留 `class=\"\"`（属性**仍在**）"
        );
    }

    /// ⭐ **移除 property ⇒ 回到默认值**（浏览器语义）。
    #[test]
    fn removing_a_property_falls_back_to_its_default_value() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("input").expect("建节点必须成功");
        dom.set_property(&node, "value", "abc")
            .expect("写 property 必须成功");
        dom.remove_property(&node, "value")
            .expect("移除 property 必须成功");
        assert_eq!(
            dom.get_property(&node, "value")
                .expect("读 property 必须成功"),
            String::new(),
            "字符串 property 的默认值是空串"
        );
    }

    /// **布尔 property 的默认值是 `false`**（不是空串）⇒ 与浏览器一致。
    #[test]
    fn removing_a_boolean_property_falls_back_to_false() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("input").expect("建节点必须成功");
        dom.set_property_bool(&node, "checked", true)
            .expect("写 property 必须成功");
        assert_eq!(
            dom.get_property(&node, "checked")
                .expect("读 property 必须成功"),
            String::from("true")
        );

        dom.remove_property(&node, "checked")
            .expect("移除 property 必须成功");
        assert_eq!(
            dom.get_property(&node, "checked")
                .expect("读 property 必须成功"),
            String::from("false"),
            "布尔 property 的默认值是 `false`（与浏览器一致）"
        );
    }

    /// **按 kind 摘监听器** ⇒ 同元素的其它 kind **不受影响**。
    #[test]
    fn removing_one_listener_kind_keeps_the_other_kinds() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let node = dom.create_element("div").expect("建节点必须成功");
        let clicks = Rc::new(Cell::new(0u32));
        let keys = Rc::new(Cell::new(0u32));
        let click_counter = clicks.clone();
        let key_counter = keys.clone();
        dom.add_listener(
            &node,
            EventKind::Click,
            crate::events::ElementHandler::new(move |_event| {
                click_counter.set(click_counter.get() + 1);
            }),
        )
        .expect("挂处理器必须成功");
        dom.add_listener(
            &node,
            EventKind::KeyDown,
            crate::events::ElementHandler::new(move |_event| {
                key_counter.set(key_counter.get() + 1);
            }),
        )
        .expect("挂处理器必须成功");
        dom.append_child(&root, &node).expect("挂载必须成功");

        dom.remove_listener(&node, EventKind::Click)
            .expect("摘监听器必须成功");
        assert_eq!(dom.dispatch(node, EventKind::Click), 0, "click 被摘掉了");
        assert_eq!(
            dom.dispatch(node, EventKind::KeyDown),
            1,
            "keydown **不许**被牵连（既有契约）"
        );
    }

    /// **目标路径**只数元素 / 文本（与浏览器同口径 ⇒ 两侧可比）。
    #[test]
    fn the_target_path_counts_only_elements_and_texts() {
        let dom = InMemoryDom::new();
        let root = dom.create_element("main").expect("建节点必须成功");
        let first = dom.create_element("div").expect("建节点必须成功");
        let second = dom.create_element("span").expect("建节点必须成功");
        let text = dom.create_text("x").expect("建节点必须成功");
        dom.append_child(&root, &first).expect("挂载必须成功");
        dom.append_child(&root, &second).expect("挂载必须成功");
        dom.append_child(&root, &text).expect("挂载必须成功");

        assert_eq!(dom.target_path(&first).expect("读路径必须成功"), vec![0]);
        assert_eq!(dom.target_path(&second).expect("读路径必须成功"), vec![1]);
        assert_eq!(
            dom.target_path(&text).expect("读路径必须成功"),
            vec![2],
            "文本节点也占下标（与浏览器的可见子节点一致）"
        );
    }
}
