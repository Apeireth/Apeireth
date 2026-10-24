//! 数据与用户（**设置页 · 数据子面**）—— 生产视图的 Rust 重写。
//!
//! ## 面边界（这一面只回答「长什么样」）
//!
//! | 在本模块 | 不在本模块（**宿主侧**） |
//! |---|---|
//! | 两个分区（`data` / `user`）的**结构 / class 名 / 文本** | **运维读数**的取值（宿主从监督面读，喂进来） |
//! | 「读不到」分支的**占位呈现**（`—` + 说明行） | 「读不到」分支的**判定**（宿主判定） |
//! | **危险动作**的按钮与**意图出口** | **危险动作的执行**（删数据 ⇒ 宿主执行） |
//! | 工作区行的**输入框与按钮** | 工作区目录的**读写**（宿主读写） |
//! | 用户卡的**头像槽 / 昵称 / 签名**输入 | 头像的**压缩**与**本地存取**（宿主做） |
//!
//! ⇒ 数据由**宿主注入**（`Props`）；引擎不碰数据面与宿主能力
//! （设计文档 §1.3 第 6 / 7 条）。
//!
//! ## class 名是视觉契约（设计文档 §9.1）
//!
//! 生产视图的 class 名与本模块**逐字一致**（`.setting-block` / `.cap-card` /
//! `.ops-row` / `.user-card` / `.danger-zone-box` / …）⇒ **不改名**
//! （改名 ⇒ 像素不一致）。
//!
//! ## 空白（**逐字照抄生产 DOM 的浏览器实测 dump**）
//!
//! 生产模板里相邻元素之间的换行会折叠成**一个空格**（文本节点）⇒ 本模块
//! **逐个复现**这些空白文本节点。
//!
//! ⚠️ **两处「前导 / 尾随空格」属于同一个文本节点**（不是独立的空格节点）：
//!
//! - 运维卡标题：图标与文字之间的空格**并进**文字那个节点 ⇒ 文字节点的取值是
//!   `" daemon 状态（只读）"`（**前导空格**）；
//! - 工作区标题 / 用户中心标题：文案后面紧跟一个字段旗标 ⇒ 文字节点的取值是
//!   `"工作区目录 "` / `"用户中心 "`（**尾随空格**）。
//!
//! ## 「读不到」分支（**诚实口径**）
//!
//! 生产的读数映射在「读不到」时给 4 行 `—` + 一条说明 ⇒ **本模块只渲染**
//! 宿主算好的行（含取值 `—`）⇒ 判定在宿主侧（`—` 是**占位串**，不是「假装有值」）。
//!
//! ## 危险动作：**只发意图**
//!
//! 生产的两个清除按钮点下去只**打开二次确认弹层**（弹层是**共享覆盖层**，
//! 不属本面）⇒ 本模块的按钮**只发意图**（[`DangerAction::on_request`]）
//! ⇒ **不执行任何删除** ⇒ 宿主侧负责二次确认与执行。
//!
//! ## 诚实边界
//!
//! - 运维读数 / 工作区目录 / 用户资料 ⇒ **宿主注入**（固定数据）⇒ 本模块
//!   **不读**任何数据面；
//! - 头像的**压缩**（256px JPEG）在宿主侧 ⇒ 本模块只渲染「已有头像」或
//!   「兜底图标」两个分支；
//! - **二次确认弹层**不属本面 ⇒ 本模块只发「请求危险动作」的意图。

use crate::icon::{Icon, IconProps};
use apeireth_ui_core::{Callback, UiComponent};
use apeireth_ui_dom::{AttrValue, VNode};
use apeireth_ui_macro::view;

/// 本面用到的图标数据（**生成物** —— 勿手改）。
///
/// 来源：`frontend/companion-desktop/node_modules/lucide-svelte/dist/icons/*.svelte`
/// （上游图标包 ⇒ 真实 `.svelte` ⇒ `iconNode`）⇒ class 后缀取**真实文件名**。
///
/// ⚠️ 生产 import 名 ⇒ 真实 class：`Trash2` ⇒ `lucide-trash-2`；`Folder` ⇒
/// `lucide-folder`；`RefreshCcw` ⇒ `lucide-refresh-ccw`；`Radar` ⇒
/// `lucide-radar`；`UserRound` ⇒ `lucide-user-round`；`ImagePlus` ⇒
/// `lucide-image-plus`。
///
/// ⚠️ `Check` / `Copy` / `AlertTriangle`（⇒ `lucide-triangle-alert`）已在本 crate
/// 的共享图标数据里 ⇒ **直接复用**，不重复定义。
///
/// 上游许可（ISC；其中 Feather 部分为 MIT）—— 原文如下：
///
/// ISC License
///
/// Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part of Feather (MIT). All other copyright (c) for Lucide are held by Lucide Contributors 2022.
///
/// Permission to use, copy, modify, and/or distribute this software for any purpose with or without fee is hereby granted, provided that the above copyright notice and this permission notice appear in all copies.
///
/// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
pub mod icons {
    use crate::icon::{IconData, IconNode};

    /// 图标 `trash-2`。
    #[rustfmt::skip]
    pub const ICON_TRASH_2: IconData = IconData {
        name: "trash-2",
        class: "lucide-icon lucide lucide-trash-2",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M3 6h18")] },
            IconNode { tag: "path", attrs: &[("d", "M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6")] },
            IconNode { tag: "path", attrs: &[("d", "M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2")] },
            IconNode { tag: "line", attrs: &[("x1", "10"), ("x2", "10"), ("y1", "11"), ("y2", "17")] },
            IconNode { tag: "line", attrs: &[("x1", "14"), ("x2", "14"), ("y1", "11"), ("y2", "17")] },
        ],
    };

    /// 图标 `folder`。
    #[rustfmt::skip]
    pub const ICON_FOLDER: IconData = IconData {
        name: "folder",
        class: "lucide-icon lucide lucide-folder",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z")] },
        ],
    };

    /// 图标 `refresh-ccw`。
    #[rustfmt::skip]
    pub const ICON_REFRESH_CCW: IconData = IconData {
        name: "refresh-ccw",
        class: "lucide-icon lucide lucide-refresh-ccw",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8")] },
            IconNode { tag: "path", attrs: &[("d", "M3 3v5h5")] },
            IconNode { tag: "path", attrs: &[("d", "M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16")] },
            IconNode { tag: "path", attrs: &[("d", "M16 16h5v5")] },
        ],
    };

    /// 图标 `radar`。
    #[rustfmt::skip]
    pub const ICON_RADAR: IconData = IconData {
        name: "radar",
        class: "lucide-icon lucide lucide-radar",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M19.07 4.93A10 10 0 0 0 6.99 3.34")] },
            IconNode { tag: "path", attrs: &[("d", "M4 6h.01")] },
            IconNode { tag: "path", attrs: &[("d", "M2.29 9.62A10 10 0 1 0 21.31 8.35")] },
            IconNode { tag: "path", attrs: &[("d", "M16.24 7.76A6 6 0 1 0 8.23 16.67")] },
            IconNode { tag: "path", attrs: &[("d", "M12 18h.01")] },
            IconNode { tag: "path", attrs: &[("d", "M17.99 11.66A6 6 0 0 1 15.77 16.67")] },
            IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "12"), ("r", "2")] },
            IconNode { tag: "path", attrs: &[("d", "m13.41 10.59 5.66-5.66")] },
        ],
    };

    /// 图标 `user-round`。
    #[rustfmt::skip]
    pub const ICON_USER_ROUND: IconData = IconData {
        name: "user-round",
        class: "lucide-icon lucide lucide-user-round",
        nodes: &[
            IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "8"), ("r", "5")] },
            IconNode { tag: "path", attrs: &[("d", "M20 21a8 8 0 0 0-16 0")] },
        ],
    };

    /// 图标 `image-plus`。
    #[rustfmt::skip]
    pub const ICON_IMAGE_PLUS: IconData = IconData {
        name: "image-plus",
        class: "lucide-icon lucide lucide-image-plus",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M16 5h6")] },
            IconNode { tag: "path", attrs: &[("d", "M19 2v6")] },
            IconNode { tag: "path", attrs: &[("d", "M21 11.5V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7.5")] },
            IconNode { tag: "path", attrs: &[("d", "m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21")] },
            IconNode { tag: "circle", attrs: &[("cx", "9"), ("cy", "9"), ("r", "2")] },
        ],
    };
}

// ── 文案常量（**逐字**取自生产模板） ────────────────────────────────────────

/// 「数据与本地缓存」分区的标题。
pub const DATA_BLOCK_TITLE: &str = "数据与本地缓存";

/// 「数据与本地缓存」分区的说明。
pub const DATA_BLOCK_DESC: &str = "管理客户端本地存储的会话与配置缓存。";

/// 「用户中心」分区的标题（**带尾随空格**：后面紧跟字段旗标）。
pub const USER_BLOCK_TITLE: &str = "用户中心 ";

/// 「用户中心」分区的说明。
pub const USER_BLOCK_DESC: &str =
    "你是谁：头像、昵称与签名。暂只随应用保存在这台机器，不上传、不进后端配置。";

/// 运维卡标题（**带前导空格**：前面紧跟图标 ⇒ 空格与文字同属一个文本节点）。
pub const OPS_CARD_TITLE: &str = " daemon 状态（只读）";

/// 运维卡标题里的图标尺寸（生产 `size={13}`）。
pub const OPS_CARD_ICON_SIZE: u32 = 13;

/// 刷新按钮的文案（空闲态）。
pub const REFRESH_LABEL: &str = "刷新";

/// 刷新按钮的文案（读取中）。
pub const REFRESH_BUSY_LABEL: &str = "读取中…";

/// 启停指引小节的标题。
pub const GUIDANCE_TITLE: &str = "服务启停指引（CLI 命令可复制；界面不代跑）";

/// 复制按钮的文案。
pub const COPY_LABEL: &str = "复制";

/// 复制按钮的文案（已复制）。
pub const COPIED_LABEL: &str = "已复制";

/// 零装边界的注记（**逐字**取自生产的零装说明常量）。
pub const ZERO_INSTALL_NOTE: &str = "本卡只读展示 + 指引：不代跑安装/启停、不加安装按钮。安装/卸载与参数面（-Port / -Bind / -RotateToken）见运维手册 docs/02-guides/server-deployment.md 与 packaging/service/README.md。";

/// 「读不到」时的说明行（**逐字**取自生产的读数映射常量）。
pub const UNAVAILABLE_NOTE: &str = "仅桌面版可读（web 构建无监督面）";

/// 「读不到」时的占位串（**短横线族**；逐字取自生产的占位常量）。
///
/// ⚠️ 生产用的是 **U+2014**（EM DASH）⇒ 本常量**逐字**照抄 ⇒ 像素与 DOM 对照
/// 都按这一个字符走。
pub const DASH: &str = "—";

/// 工作区标题（**带尾随空格**：后面紧跟字段旗标）。
pub const WORKSPACE_TITLE: &str = "工作区目录 ";

/// 工作区输入框的占位文案。
pub const WORKSPACE_PLACEHOLDER: &str = "默认 (应用数据目录)";

/// 工作区输入框的无障碍名。
pub const WORKSPACE_LABEL: &str = "工作区目录";

/// 「更改」按钮的文案。
pub const WORKSPACE_PICK_LABEL: &str = "更改";

/// 工作区行的说明。
pub const WORKSPACE_HINT: &str =
    "路径可直接编辑，失焦或回车提交（失败回填旧值）；新路径在下次启动 sidecar 时生效。";

/// 危险区的标题。
pub const DANGER_TITLE: &str = "危险区域 (Danger Zone)";

/// 危险区的说明。
pub const DANGER_DESC: &str = "对话记录是近期的聊天内容；长期记忆是他从相处中学到的东西，不受影响。两档删除都会同时清后端账本（重启不复活）。";

/// 危险区的「尚未接线」说明。
pub const DANGER_FUTURE_DESC: &str =
    "记忆遗忘（连同长期记忆一起清除）是另一个动作，需单独审批协议——尚未接线。";

/// 危险动作登记表（**动作键 ⇒ 按钮文案**；逐字取自生产模板）。
///
/// ⚠️ 动作键是**意图载荷**（宿主据此执行）⇒ 本模块**只渲染**，
/// **不执行**任何清除。
pub const DANGER_ACTIONS: [(&str, &str); 2] = [
    (
        "clearRecentConversations",
        "清除近期对话记录（保留长期记忆）",
    ),
    ("clearAllSessionData", "清除全部会话数据（保留长期记忆）"),
];

/// 头像槽的兜底图标的尺寸（生产 `size={30}`）。
pub const AVATAR_FALLBACK_ICON_SIZE: u32 = 30;

/// 上传 / 更换头像按钮的文案（无头像）。
pub const UPLOAD_AVATAR_LABEL: &str = "上传头像";

/// 上传 / 更换头像按钮的文案（有头像）。
pub const REPLACE_AVATAR_LABEL: &str = "更换头像";

/// 移除头像按钮的文案。
pub const REMOVE_AVATAR_LABEL: &str = "移除头像";

/// 头像区的说明。
pub const AVATAR_HINT: &str = "自动裁方压成 256px 存本地；点击主界面左上角头像可随时回到这里。";

/// 头像文件选择器的接受类型。
pub const AVATAR_FILE_ACCEPT: &str = "image/*";

/// 头像图片的无障碍名。
pub const AVATAR_ALT: &str = "用户头像";

/// 昵称输入框的标签。
pub const NICKNAME_LABEL: &str = "昵称";

/// 昵称输入框的 id。
pub const NICKNAME_ID: &str = "user-nickname";

/// 昵称输入框的最大长度（生产 `maxlength={32}`）。
pub const NICKNAME_MAX_LENGTH: &str = "32";

/// 昵称输入框的占位文案。
pub const NICKNAME_PLACEHOLDER: &str = "怎么称呼你";

/// 签名输入框的标签。
pub const BIO_LABEL: &str = "一句话签名";

/// 签名输入框的 id。
pub const BIO_ID: &str = "user-bio";

/// 签名输入框的最大长度（生产 `maxlength={120}`）。
pub const BIO_MAX_LENGTH: &str = "120";

/// 签名输入框的占位文案。
pub const BIO_PLACEHOLDER: &str = "写点什么给自己看";

/// 用户信息分组的无障碍名。
pub const USER_GROUP_LABEL: &str = "用户信息";

/// 用户信息区的说明。
pub const USER_HINT: &str =
    "失焦或回车即存本地（无需保存按钮）；只落这台机器的应用存储，暂无云端同步。";

// ── 数据形状（**中性**；宿主注入） ──────────────────────────────────────────

/// 本面覆盖的分区。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataSection {
    /// 数据与本地缓存（运维读数 + 工作区 + 危险区）。
    #[default]
    Data,
    /// 用户中心（头像 + 昵称 + 签名）。
    User,
}

impl DataSection {
    /// 按分区 id 取（**未知 id ⇒ 落回 `data`** ⇒ 与生产的兜底一致）。
    pub fn from_id(id: &str) -> Self {
        match id {
            "user" => Self::User,
            _ => Self::Data,
        }
    }

    /// 分区 id。
    pub fn id(self) -> &'static str {
        match self {
            Self::Data => "data",
            Self::User => "user",
        }
    }
}

/// 读数行（**宿主算好的展示行** ⇒ 组件只渲染）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpsRow {
    /// 行键（宿主用；**不进画面**）。
    pub key: String,
    /// 行标题。
    pub label: String,
    /// 行取值（**读不到时是占位串 `—`** ⇒ 宿主给的）。
    pub value: String,
    /// 口径注记（空串 ⇒ 不渲染注记节点）。
    pub note: String,
}

impl OpsRow {
    /// 建一行读数。
    pub fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        value: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            value: value.into(),
            note: note.into(),
        }
    }
}

/// 启停指引行（**宿主注入**：命令文本逐字来自固定数据）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GuidanceRow {
    /// 行键（复制回执按它判定 ⇒ 宿主用）。
    pub key: String,
    /// 行标题。
    pub label: String,
    /// 命令文本（**逐字**渲染）。
    pub command: String,
    /// 命令说明。
    pub desc: String,
    /// 点「复制」⇒ 意图出口（**宿主侧**写剪贴板）。
    pub on_copy: Option<Callback>,
}

impl GuidanceRow {
    /// 建一行指引（无意图出口）。
    pub fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        command: impl Into<String>,
        desc: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            command: command.into(),
            desc: desc.into(),
            on_copy: None,
        }
    }

    /// 接上「复制这条命令」的意图出口（builder 风格）。
    pub fn on_copy(mut self, on_copy: Callback) -> Self {
        self.on_copy = Some(on_copy);
        self
    }
}

/// 危险动作（**宿主注入**：键是意图载荷 ⇒ 引擎只发意图）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DangerAction {
    /// 动作键（意图载荷）。
    pub key: String,
    /// 按钮文案（**逐字**渲染）。
    pub label: String,
    /// 点按钮 ⇒ 意图出口（**只发意图**；宿主负责二次确认与执行）。
    pub on_request: Option<Callback>,
}

impl DangerAction {
    /// 建一个危险动作（无意图出口）。
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            on_request: None,
        }
    }

    /// 接上「请求执行这个危险动作」的意图出口（builder 风格）。
    pub fn on_request(mut self, on_request: Callback) -> Self {
        self.on_request = Some(on_request);
        self
    }
}

/// 用户资料（**宿主注入**）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UserProfile {
    /// 昵称。
    pub nickname: String,
    /// 签名。
    pub bio: String,
    /// 头像（**已压缩好的 data URL**；空 ⇒ 渲染兜底图标）。
    ///
    /// ⚠️ 压缩在**宿主侧** ⇒ 本模块**不碰**任何图片处理。
    pub avatar: Option<String>,
}

// ── 组件 ────────────────────────────────────────────────────────────────────

/// 数据与用户面板的 props（**宿主注入** ⇒ 组件是纯函数）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DataPanelsProps {
    /// 当前分区（`data` / `user`）。
    pub section: DataSection,
    /// 运维读数行（侧车状态面）。
    pub status_rows: Vec<OpsRow>,
    /// 运维读数行（常驻探测面）。
    pub probe_rows: Vec<OpsRow>,
    /// 启停指引行。
    pub guidance: Vec<GuidanceRow>,
    /// 已复制的行键（空串 ⇒ 没有回执 ⇒ 渲染「复制」）。
    pub copied_key: String,
    /// 复制失败的说明（空串 ⇒ 不渲染错误行）。
    pub copy_error: String,
    /// 零装边界的注记。
    pub zero_install_note: String,
    /// 读数进行中（按钮禁用 + 文案切换 + 图标转圈 class）。
    pub daemon_loading: bool,
    /// 工作区目录（**宿主持有**；输入框的值由宿主写 property）。
    pub workspace_dir: String,
    /// 工作区读写进行中（按钮禁用）。
    pub workspace_loading: bool,
    /// 工作区错误（空串 ⇒ 不渲染错误行）。
    pub workspace_error: String,
    /// 危险动作表。
    pub danger_actions: Vec<DangerAction>,
    /// 用户资料。
    pub user_profile: UserProfile,
    /// 用户资料错误（空串 ⇒ 不渲染错误行）。
    pub user_profile_error: String,
    /// 「刷新」⇒ 意图出口（宿主重新读数）。
    pub on_refresh: Option<Callback>,
    /// 「更改」⇒ 意图出口（宿主打开目录选择器）。
    pub on_pick_workspace: Option<Callback>,
    /// 工作区输入 ⇒ 意图出口（宿主写值并提交）。
    pub on_workspace_input: Option<Callback>,
    /// 上传 / 更换头像 ⇒ 意图出口（宿主打开文件选择器）。
    pub on_request_avatar: Option<Callback>,
    /// 移除头像 ⇒ 意图出口（宿主写空头像）。
    pub on_remove_avatar: Option<Callback>,
    /// 头像文件已选 ⇒ 意图出口（宿主压缩并存）。
    pub on_avatar_file_changed: Option<Callback>,
    /// 昵称改动 ⇒ 意图出口。
    pub on_nickname_input: Option<Callback>,
    /// 签名改动 ⇒ 意图出口。
    pub on_bio_input: Option<Callback>,
}

/// 数据与用户面板组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DataPanels;

impl UiComponent for DataPanels {
    type Props = DataPanelsProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        match props.section {
            DataSection::Data => render_data_panel(props),
            DataSection::User => render_user_panel(props),
        }
    }
}

/// 渲染「数据与本地缓存」分区。
fn render_data_panel(props: &DataPanelsProps) -> VNode {
    // ⚠️ `view!` 里**大写开头的标识符**被当成**组件名** ⇒ 文案常量先落到
    // 小写局部变量再用（否则宏会把常量当组件去 `builder()`）。
    let title = DATA_BLOCK_TITLE;
    let desc = DATA_BLOCK_DESC;

    view! {
        div(class = "setting-block") {
            h3(class = "block-title") { title }
            " "
            p(class = "block-desc") { desc }
            " "
            { render_ops_card(props) }
            " "
            { render_workspace_card(props) }
            " "
            { render_danger_zone(props) }
        }
    }
}

/// 渲染运维读数卡（只读展示 + 启停指引）。
fn render_ops_card(props: &DataPanelsProps) -> VNode {
    let guidance_title = GUIDANCE_TITLE;
    let card_title = OPS_CARD_TITLE;
    let zero_note = props.zero_install_note.clone();
    let refresh_label = if props.daemon_loading {
        REFRESH_BUSY_LABEL
    } else {
        REFRESH_LABEL
    };
    // 生产用 `disabled={daemonLoading}` ⇒ 忙碌时属性反射成**空串**（不是 `"true"`）。
    let disabled = if props.daemon_loading {
        AttrValue::Text(String::new())
    } else {
        AttrValue::Bool(false)
    };
    let radar_icon = Icon::render(&IconProps::new(&icons::ICON_RADAR, OPS_CARD_ICON_SIZE));
    // 生产把转圈 class 挂在图标上（`class={daemonLoading ? 'spin' : ''}`）。
    let refresh_icon = if props.daemon_loading {
        Icon::render(
            &IconProps::new(&icons::ICON_REFRESH_CCW, OPS_CARD_ICON_SIZE).extra_class("spin"),
        )
    } else {
        Icon::render(&IconProps::new(
            &icons::ICON_REFRESH_CCW,
            OPS_CARD_ICON_SIZE,
        ))
    };
    let on_refresh = props.on_refresh.clone();

    view! {
        div(class = "cap-card") {
            div(class = "cap-card-head") {
                span(class = "cap-card-title") {
                    { radar_icon }
                    card_title
                }
                " "
                button(
                    class = "quiet-button",
                    disabled = disabled.clone(),
                    onclick = {
                        move |_| {
                            if let Some(callback) = on_refresh.as_ref() {
                                callback.call();
                            }
                        }
                    }
                ) {
                    { refresh_icon }
                    " "
                    span { refresh_label }
                }
            }
            " "
            div(class = "ops-rows") {
                for row in props.status_rows.iter() {
                    { render_ops_row(row) }
                }
                " "
                for row in props.probe_rows.iter() {
                    { render_ops_row(row) }
                }
            }
            " "
            div(class = "ops-guidance") {
                div(class = "ops-guidance-title") { guidance_title }
                " "
                for row in props.guidance.iter() {
                    { render_command_row(row, props) }
                }
                " "
                p(class = "field-hint") { zero_note }
                " "
                if !props.copy_error.is_empty() {
                    p(class = "field-hint error-hint") { props.copy_error.clone() }
                }
            }
        }
    }
}

/// 渲染一行读数（标签 / 取值 / 注记）。
fn render_ops_row(row: &OpsRow) -> VNode {
    let row_label = row.label.clone();
    let value = row.value.clone();
    let note = row.note.clone();
    let has_note = !note.is_empty();

    view! {
        div(class = "ops-row") {
            span(class = "ops-label") { row_label }
            " "
            span(class = "ops-value") { value }
            " "
            if has_note {
                span(class = "ops-note") { note }
            }
        }
    }
}

/// 渲染一行启停指引（标题 + 复制按钮 + 命令 + 说明）。
fn render_command_row(row: &GuidanceRow, props: &DataPanelsProps) -> VNode {
    let cmd_label = row.label.clone();
    let command = row.command.clone();
    let desc = row.desc.clone();
    let copied = props.copied_key == row.key && !row.key.is_empty();
    let copy_label = if copied { COPIED_LABEL } else { COPY_LABEL };
    // 生产：已复制 ⇒ `Check` 图标；否则 ⇒ `Copy` 图标（**结构分支**：只产出一个图标）。
    let copy_icon = if copied {
        Icon::render(&IconProps::new(&crate::icon_data::ICON_CHECK, 12))
    } else {
        Icon::render(&IconProps::new(&crate::icon_data::ICON_COPY, 12))
    };
    let on_copy = row.on_copy.clone();

    view! {
        div(class = "ops-command") {
            div(class = "ops-command-head") {
                strong { cmd_label }
                " "
                button(
                    class = "quiet-button",
                    onclick = {
                        move |_| {
                            if let Some(callback) = on_copy.as_ref() {
                                callback.call();
                            }
                        }
                    }
                ) {
                    { copy_icon }
                    " "
                    span { copy_label }
                }
            }
            " "
            code(class = "ops-command-text") { command }
            " "
            small { desc }
        }
    }
}

/// 渲染工作区行（输入框 + 「更改」按钮）。
fn render_workspace_card(props: &DataPanelsProps) -> VNode {
    let title = WORKSPACE_TITLE;
    let placeholder = WORKSPACE_PLACEHOLDER;
    let label = WORKSPACE_LABEL;
    let pick_label = WORKSPACE_PICK_LABEL;
    let hint = WORKSPACE_HINT;
    let error = props.workspace_error.clone();
    let has_error = !error.is_empty();
    // 生产用 `disabled={workspaceLoading}` ⇒ 忙碌时属性是**空串**。
    let disabled = if props.workspace_loading {
        AttrValue::Text(String::new())
    } else {
        AttrValue::Bool(false)
    };
    let folder_icon = Icon::render(&IconProps::new(&icons::ICON_FOLDER, 13));
    let on_pick = props.on_pick_workspace.clone();
    let on_input = props.on_workspace_input.clone();

    view! {
        div(class = "info-card") {
            strong(class = "info-title") { title }
            " "
            div(class = "workspace-row") {
                input(
                    class = "workspace-path workspace-input",
                    type = "text",
                    placeholder = placeholder,
                    aria-label = label,
                    oninput = {
                        move |_| {
                            if let Some(callback) = on_input.as_ref() {
                                callback.call();
                            }
                        }
                    }
                )
                " "
                button(
                    class = "quiet-button",
                    disabled = disabled.clone(),
                    onclick = {
                        move |_| {
                            if let Some(callback) = on_pick.as_ref() {
                                callback.call();
                            }
                        }
                    }
                ) {
                    { folder_icon }
                    " "
                    span { pick_label }
                }
            }
            " "
            if has_error {
                p(class = "field-hint error-hint") { error }
            }
            " "
            p(class = "field-hint") { hint }
        }
    }
}

/// 渲染危险区（两档清除 + 未接线说明）。
fn render_danger_zone(props: &DataPanelsProps) -> VNode {
    let title = DANGER_TITLE;
    let desc = DANGER_DESC;
    let future = DANGER_FUTURE_DESC;
    let danger_icon = Icon::render(
        &IconProps::new(&crate::icon_data::ICON_TRIANGLE_ALERT, 16).extra_class("danger-icon"),
    );
    // ⚠️ 生产模板里两个按钮**之间**有一个换行 ⇒ 折叠成**一个空格**；
    // ⇒ 按钮与分隔空格**交替**推进列表（末位**不**带分隔空格）。
    let mut buttons: Vec<VNode> = Vec::new();
    for (index, action) in props.danger_actions.iter().enumerate() {
        if index > 0 {
            buttons.push(VNode::text(" "));
        }
        buttons.push(render_danger_button(action));
    }

    view! {
        div(class = "danger-zone-box") {
            div(class = "danger-head") {
                { danger_icon }
                " "
                strong { title }
            }
            " "
            p(class = "danger-desc") { desc }
            " "
            div(class = "danger-actions") {
                for button in buttons {
                    { button }
                }
            }
            " "
            p(class = "danger-desc danger-future") { future }
        }
    }
}

/// 渲染一个危险动作按钮（**只发意图** ⇒ 不执行任何清除）。
fn render_danger_button(action: &DangerAction) -> VNode {
    let action_label = action.label.clone();
    let trash_icon = Icon::render(&IconProps::new(&icons::ICON_TRASH_2, 13));
    let on_request = action.on_request.clone();

    view! {
        button(
            class = "danger-button",
            onclick = {
                move |_| {
                    if let Some(callback) = on_request.as_ref() {
                        callback.call();
                    }
                }
            }
        ) {
            { trash_icon }
            " "
            span { action_label }
        }
    }
}

/// 渲染「用户中心」分区。
fn render_user_panel(props: &DataPanelsProps) -> VNode {
    let title = USER_BLOCK_TITLE;
    let desc = USER_BLOCK_DESC;
    let hint = USER_HINT;

    view! {
        div(class = "setting-block") {
            h3(class = "block-title") { title }
            " "
            p(class = "block-desc") { desc }
            " "
            { render_user_card(props) }
            " "
            { render_user_group(props) }
            " "
            if !props.user_profile_error.is_empty() {
                p(class = "field-hint error-hint") { props.user_profile_error.clone() }
            }
            " "
            p(class = "field-hint") { hint }
        }
    }
}

/// 渲染用户卡（头像槽 + 上传 / 移除按钮 + 隐藏文件输入）。
fn render_user_card(props: &DataPanelsProps) -> VNode {
    let avatar = props.user_profile.avatar.clone();
    let has_avatar = avatar
        .as_deref()
        .map(|url| !url.is_empty())
        .unwrap_or(false);
    let upload_label = if has_avatar {
        REPLACE_AVATAR_LABEL
    } else {
        UPLOAD_AVATAR_LABEL
    };
    let fallback_icon = Icon::render(&IconProps::new(
        &icons::ICON_USER_ROUND,
        AVATAR_FALLBACK_ICON_SIZE,
    ));
    let remove_label = REMOVE_AVATAR_LABEL;
    let hint = AVATAR_HINT;
    let on_request = props.on_request_avatar.clone();
    let on_remove = props.on_remove_avatar.clone();
    let on_file = props.on_avatar_file_changed.clone();

    view! {
        div(class = "user-card") {
            div(class = "user-avatar-slot") {
                if has_avatar {
                    img(
                        class = "user-avatar-img",
                        src = avatar.unwrap_or_default(),
                        alt = AVATAR_ALT
                    )
                } else {
                    span(class = "user-avatar-fallback") {
                        { fallback_icon }
                    }
                }
            }
            " "
            div(class = "user-avatar-side") {
                div(class = "user-avatar-actions") {
                    button(
                        class = "quiet-button",
                        onclick = {
                            move |_| {
                                if let Some(callback) = on_request.as_ref() {
                                    callback.call();
                                }
                            }
                        }
                    ) {
                        { Icon::render(&IconProps::new(&icons::ICON_IMAGE_PLUS, 13)) }
                        " "
                        span { upload_label }
                    }
                    " "
                    if has_avatar {
                        button(
                            class = "quiet-button",
                            onclick = {
                                move |_| {
                                    if let Some(callback) = on_remove.as_ref() {
                                        callback.call();
                                    }
                                }
                            }
                        ) {
                            { Icon::render(&IconProps::new(&icons::ICON_TRASH_2, 13)) }
                            " "
                            span { remove_label }
                        }
                    }
                }
                " "
                input(
                    type = "file",
                    accept = AVATAR_FILE_ACCEPT,
                    hidden = AttrValue::Text(String::new()),
                    onchange = {
                        move |_| {
                            if let Some(callback) = on_file.as_ref() {
                                callback.call();
                            }
                        }
                    }
                )
                " "
                p(class = "field-hint") { hint }
            }
        }
    }
}

/// 渲染用户信息分组（昵称 + 签名）。
fn render_user_group(props: &DataPanelsProps) -> VNode {
    let nickname_label = NICKNAME_LABEL;
    let nickname_placeholder = NICKNAME_PLACEHOLDER;
    let bio_label = BIO_LABEL;
    let bio_placeholder = BIO_PLACEHOLDER;
    let on_nickname = props.on_nickname_input.clone();
    let on_bio = props.on_bio_input.clone();

    view! {
        div(class = "text-group", role = "group", aria-label = USER_GROUP_LABEL) {
            div(class = "form-group") {
                label(for = NICKNAME_ID) { nickname_label }
                " "
                input(
                    id = NICKNAME_ID,
                    type = "text",
                    maxlength = NICKNAME_MAX_LENGTH,
                    placeholder = nickname_placeholder,
                    oninput = {
                        move |_| {
                            if let Some(callback) = on_nickname.as_ref() {
                                callback.call();
                            }
                        }
                    }
                )
            }
            " "
            div(class = "form-group") {
                label(for = BIO_ID) { bio_label }
                " "
                input(
                    id = BIO_ID,
                    type = "text",
                    maxlength = BIO_MAX_LENGTH,
                    placeholder = bio_placeholder,
                    oninput = {
                        move |_| {
                            if let Some(callback) = on_bio.as_ref() {
                                callback.call();
                            }
                        }
                    }
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! 数据与用户面板的**行为**断言（渲染结果 ⇒ 结构 / class / 文本 / 属性），
    //! 不读源码（设计文档 §10.2 的「钉行为」口径）。

    use super::*;
    use apeireth_ui_core::Runtime;

    /// 收集全部 class token（去重、保序）。
    fn classes(node: &VNode, out: &mut Vec<String>) {
        for token in node.class_tokens() {
            if !out.contains(&token) {
                out.push(token);
            }
        }
        for child in node.children() {
            classes(child, out);
        }
    }

    /// 数某个 class token 出现了多少次（**不去重**）。
    fn count_class(node: &VNode, token: &str) -> usize {
        let mut count = 0usize;
        for candidate in node.class_tokens() {
            if candidate == token {
                count += 1;
            }
        }
        for child in node.children() {
            count += count_class(child, token);
        }
        count
    }

    /// 收集文本节点（**逐个**，含空白）。
    fn text_nodes(node: &VNode, out: &mut Vec<String>) {
        match node {
            VNode::Element(element) => {
                for child in &element.children {
                    text_nodes(child, out);
                }
            }
            VNode::Text(text) => out.push(text.data.clone()),
            VNode::Fragment(children) => {
                for child in children {
                    text_nodes(child, out);
                }
            }
        }
    }

    /// 收集全部属性（`标签@属性` ⇒ 值）。
    fn attributes(node: &VNode, out: &mut Vec<(String, String)>) {
        if let VNode::Element(element) = node {
            for (name, value) in &element.attrs {
                if let Some(text) = value.to_attr_string() {
                    out.push((format!("{}@{name}", element.tag), text));
                }
            }
        }
        for child in node.children() {
            attributes(child, out);
        }
    }

    /// 一行读数。
    fn row(key: &str, label: &str, value: &str, note: &str) -> OpsRow {
        OpsRow::new(key, label, value, note)
    }

    /// 一组最小 props（**「读不到」态**：四行占位 + 一条说明）。
    fn props() -> DataPanelsProps {
        DataPanelsProps {
            section: DataSection::Data,
            status_rows: vec![
                row("state", "侧车连接态", DASH, UNAVAILABLE_NOTE),
                row("ownership", "归属", DASH, UNAVAILABLE_NOTE),
                row("endpoint", "端点", DASH, UNAVAILABLE_NOTE),
                row("version", "版本", DASH, UNAVAILABLE_NOTE),
            ],
            probe_rows: vec![row("resident", "常驻服务探测", DASH, UNAVAILABLE_NOTE)],
            guidance: vec![GuidanceRow::new(
                "status",
                "看状态（只读）",
                "pwsh -NoProfile -File packaging/service/apeireth-service.ps1 status",
                "任务安装态/常驻占位/健康检查/令牌在否（只报 present/absent，永不回显值）。",
            )],
            copied_key: String::new(),
            copy_error: String::new(),
            zero_install_note: String::from(ZERO_INSTALL_NOTE),
            daemon_loading: false,
            workspace_dir: String::from("/home/user/projects/demo"),
            workspace_loading: false,
            workspace_error: String::new(),
            danger_actions: vec![
                DangerAction::new(DANGER_ACTIONS[0].0, DANGER_ACTIONS[0].1),
                DangerAction::new(DANGER_ACTIONS[1].0, DANGER_ACTIONS[1].1),
            ],
            user_profile: UserProfile {
                nickname: String::from("小林"),
                bio: String::from("写点给自己看。"),
                avatar: None,
            },
            user_profile_error: String::new(),
            on_refresh: None,
            on_pick_workspace: None,
            on_workspace_input: None,
            on_request_avatar: None,
            on_remove_avatar: None,
            on_avatar_file_changed: None,
            on_nickname_input: None,
            on_bio_input: None,
        }
    }

    #[test]
    fn the_data_panel_renders_the_production_class_names() {
        let node = DataPanels::render(&props());
        let mut found = Vec::new();
        classes(&node, &mut found);
        for class in [
            "setting-block",
            "block-title",
            "block-desc",
            "cap-card",
            "cap-card-head",
            "cap-card-title",
            "quiet-button",
            "ops-rows",
            "ops-row",
            "ops-label",
            "ops-value",
            "ops-note",
            "ops-guidance",
            "ops-guidance-title",
            "ops-command",
            "ops-command-head",
            "ops-command-text",
            "field-hint",
            "info-card",
            "info-title",
            "workspace-row",
            "workspace-path",
            "workspace-input",
            "danger-zone-box",
            "danger-head",
            "danger-icon",
            "danger-desc",
            "danger-future",
            "danger-actions",
            "danger-button",
        ] {
            assert!(
                found.iter().any(|token| token == class),
                "class `{class}` 未出现在渲染结果里：{found:?}"
            );
        }
    }

    #[test]
    fn the_data_panel_renders_the_production_text() {
        let node = DataPanels::render(&props());
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        for text in [
            DATA_BLOCK_TITLE,
            DATA_BLOCK_DESC,
            OPS_CARD_TITLE,
            REFRESH_LABEL,
            GUIDANCE_TITLE,
            COPY_LABEL,
            ZERO_INSTALL_NOTE,
            WORKSPACE_TITLE,
            WORKSPACE_HINT,
            DANGER_TITLE,
            DANGER_DESC,
            DANGER_FUTURE_DESC,
            DANGER_ACTIONS[0].1,
            DANGER_ACTIONS[1].1,
        ] {
            assert!(
                texts.iter().any(|value| value == text),
                "文本 {text:?} 未出现在渲染结果里：{texts:?}"
            );
        }
    }

    #[test]
    fn the_card_title_keeps_the_leading_space_of_the_production_text_node() {
        let node = DataPanels::render(&props());
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        // 图标与文字之间的换行折叠成**一个空格**，且与文字**同属一个文本节点**。
        assert!(
            texts.iter().any(|value| value == " daemon 状态（只读）"),
            "运维卡标题的文本节点：{texts:?}"
        );
        assert!(
            !texts.iter().any(|value| value == "daemon 状态（只读）"),
            "运维卡标题的空格被拆成了独立节点：{texts:?}"
        );
    }

    #[test]
    fn the_titles_with_a_field_flag_keep_their_trailing_space() {
        let node = DataPanels::render(&props());
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            texts.iter().any(|value| value == "工作区目录 "),
            "工作区标题的尾随空格丢了：{texts:?}"
        );

        let mut user_props = props();
        user_props.section = DataSection::User;
        let node = DataPanels::render(&user_props);
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            texts.iter().any(|value| value == "用户中心 "),
            "用户中心标题的尾随空格丢了：{texts:?}"
        );
    }

    #[test]
    fn the_unreadable_branch_shows_the_placeholder_and_the_note() {
        let node = DataPanels::render(&props());
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        // 五行读数（4 行状态 + 1 行探测）⇒ 五个占位串。
        assert_eq!(
            texts.iter().filter(|value| *value == DASH).count(),
            5,
            "读不到 ⇒ 5 行占位：{texts:?}"
        );
        assert_eq!(
            texts
                .iter()
                .filter(|value| *value == UNAVAILABLE_NOTE)
                .count(),
            5,
            "读不到 ⇒ 5 条说明：{texts:?}"
        );
    }

    #[test]
    fn the_readable_branch_shows_the_values_and_no_placeholder() {
        let mut props = props();
        props.status_rows = vec![row("state", "侧车连接态", "运行中", "pid 4821 · 重启 0 次")];
        props.probe_rows = vec![row(
            "resident",
            "常驻服务探测",
            "在跑（/health 应答）",
            "占位 http://127.0.0.1:8081 · pid 4822",
        )];
        let node = DataPanels::render(&props);
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.iter().any(|value| value == "运行中"), "{texts:?}");
        assert!(
            texts
                .iter()
                .any(|value| value == "占位 http://127.0.0.1:8081 · pid 4822"),
            "{texts:?}"
        );
        assert!(
            !texts.iter().any(|value| value == DASH),
            "有读数 ⇒ 不出现占位：{texts:?}"
        );
    }

    #[test]
    fn a_row_without_a_note_has_no_note_node() {
        let mut props = props();
        props.status_rows = vec![row("state", "侧车连接态", "运行中", "")];
        props.probe_rows = vec![row("resident", "常驻服务探测", "在跑", "")];
        let node = DataPanels::render(&props);
        assert_eq!(count_class(&node, "ops-note"), 0);
    }

    #[test]
    fn the_danger_buttons_carry_the_production_labels() {
        let node = DataPanels::render(&props());
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert_eq!(count_class(&node, "danger-button"), 2);
        assert!(
            texts
                .iter()
                .any(|value| value == "清除近期对话记录（保留长期记忆）"),
            "{texts:?}"
        );
        assert!(
            texts
                .iter()
                .any(|value| value == "清除全部会话数据（保留长期记忆）"),
            "{texts:?}"
        );
    }

    #[test]
    fn the_danger_button_only_emits_an_intent() {
        let runtime = Runtime::new();
        let requested = runtime.signal(String::new());
        let sink = requested.clone();
        let mut props = props();
        props.danger_actions = vec![DangerAction::new(DANGER_ACTIONS[0].0, DANGER_ACTIONS[0].1)
            .on_request(Callback::new(move || {
                sink.set(String::from("clearRecentConversations"));
            }))];
        let node = DataPanels::render(&props);

        // 组件是纯函数：渲染本身不改状态。
        assert_eq!(requested.get(), "");

        let handler = first_handler(&node, "danger-button").expect("危险按钮上挂了处理器");
        let mut event =
            apeireth_ui_dom::DomEvent::new(apeireth_ui_dom::EventKind::Click, Vec::new());
        handler.call(&mut event);
        assert_eq!(requested.get(), "clearRecentConversations");
    }

    #[test]
    fn the_user_panel_renders_the_production_class_names() {
        let mut props = props();
        props.section = DataSection::User;
        let node = DataPanels::render(&props);
        let mut found = Vec::new();
        classes(&node, &mut found);
        for class in [
            "user-card",
            "user-avatar-slot",
            "user-avatar-fallback",
            "user-avatar-side",
            "user-avatar-actions",
            "text-group",
            "form-group",
            "field-hint",
        ] {
            assert!(
                found.iter().any(|token| token == class),
                "class `{class}` 未出现在渲染结果里：{found:?}"
            );
        }
    }

    #[test]
    fn the_user_panel_renders_the_production_text() {
        let mut props = props();
        props.section = DataSection::User;
        let node = DataPanels::render(&props);
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        for text in [
            USER_BLOCK_TITLE,
            USER_BLOCK_DESC,
            UPLOAD_AVATAR_LABEL,
            AVATAR_HINT,
            NICKNAME_LABEL,
            BIO_LABEL,
            USER_HINT,
        ] {
            assert!(
                texts.iter().any(|value| value == text),
                "文本 {text:?} 未出现在渲染结果里：{texts:?}"
            );
        }
    }

    #[test]
    fn the_user_panel_keeps_the_native_input_attributes() {
        let mut props = props();
        props.section = DataSection::User;
        let node = DataPanels::render(&props);
        let mut attrs = Vec::new();
        attributes(&node, &mut attrs);
        for (name, value) in [
            ("input@id", NICKNAME_ID),
            ("input@maxlength", NICKNAME_MAX_LENGTH),
            ("input@placeholder", NICKNAME_PLACEHOLDER),
            ("input@type", "text"),
            ("label@for", NICKNAME_ID),
        ] {
            assert!(
                attrs.iter().any(|(key, text)| key == name && text == value),
                "{name}={value} 未出现在属性表里：{attrs:?}"
            );
        }
        assert!(
            attrs
                .iter()
                .any(|(name, value)| name == "div@role" && value == "group"),
            "{attrs:?}"
        );
        assert!(
            attrs
                .iter()
                .any(|(name, value)| name == "div@aria-label" && value == USER_GROUP_LABEL),
            "{attrs:?}"
        );
    }

    #[test]
    fn the_avatar_file_input_is_hidden_and_accepts_images() {
        let mut props = props();
        props.section = DataSection::User;
        let node = DataPanels::render(&props);
        let mut attrs = Vec::new();
        attributes(&node, &mut attrs);
        assert!(
            attrs
                .iter()
                .any(|(name, value)| name == "input@type" && value == "file"),
            "{attrs:?}"
        );
        assert!(
            attrs
                .iter()
                .any(|(name, value)| name == "input@accept" && value == AVATAR_FILE_ACCEPT),
            "{attrs:?}"
        );
        // 静态 `hidden` ⇒ 属性**存在**且取值是**空串**（不是缺失）。
        assert!(
            attrs
                .iter()
                .any(|(name, value)| name == "input@hidden" && value.is_empty()),
            "{attrs:?}"
        );
    }

    #[test]
    fn the_avatar_fallback_appears_without_an_avatar() {
        let mut props = props();
        props.section = DataSection::User;
        let node = DataPanels::render(&props);
        assert_eq!(count_class(&node, "user-avatar-fallback"), 1);
        assert_eq!(count_class(&node, "user-avatar-img"), 0);
        assert_eq!(
            count_class(&node, "quiet-button"),
            1,
            "没有头像 ⇒ 只有上传按钮"
        );
    }

    #[test]
    fn the_avatar_image_replaces_the_fallback_when_present() {
        let mut props = props();
        props.section = DataSection::User;
        props.user_profile.avatar = Some(String::from("data:image/png;base64,AAAA"));
        let node = DataPanels::render(&props);
        assert_eq!(count_class(&node, "user-avatar-fallback"), 0);
        assert_eq!(count_class(&node, "user-avatar-img"), 1);
        // 有头像 ⇒ 上传 / 移除两个按钮。
        assert_eq!(count_class(&node, "quiet-button"), 2);
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            texts.iter().any(|value| value == REPLACE_AVATAR_LABEL),
            "{texts:?}"
        );
        assert!(
            texts.iter().any(|value| value == REMOVE_AVATAR_LABEL),
            "{texts:?}"
        );
    }

    #[test]
    fn the_input_values_are_not_attributes() {
        // 生产用 `bind:value` ⇒ 值是 **property**（DOM 上**没有** `value` 属性）。
        let mut props = props();
        props.section = DataSection::User;
        let node = DataPanels::render(&props);
        let mut attrs = Vec::new();
        attributes(&node, &mut attrs);
        assert!(
            !attrs.iter().any(|(name, _)| name == "input@value"),
            "输入框的值**不能**写成属性：{attrs:?}"
        );
    }

    #[test]
    fn the_workspace_input_has_no_value_attribute() {
        let node = DataPanels::render(&props());
        let mut attrs = Vec::new();
        attributes(&node, &mut attrs);
        assert!(
            attrs
                .iter()
                .any(|(name, value)| name == "input@placeholder" && value == WORKSPACE_PLACEHOLDER),
            "{attrs:?}"
        );
        assert!(
            attrs
                .iter()
                .any(|(name, value)| name == "input@aria-label" && value == WORKSPACE_LABEL),
            "{attrs:?}"
        );
        assert!(
            !attrs.iter().any(|(name, _)| name == "input@value"),
            "工作区输入框的值**不能**写成属性：{attrs:?}"
        );
    }

    #[test]
    fn the_busy_state_disables_the_refresh_button_and_switches_the_label() {
        let mut props = props();
        props.daemon_loading = true;
        let node = DataPanels::render(&props);
        let mut attrs = Vec::new();
        attributes(&node, &mut attrs);
        let disabled: Vec<&String> = attrs
            .iter()
            .filter(|(name, _)| name == "button@disabled")
            .map(|(_, value)| value)
            .collect();
        assert_eq!(disabled.len(), 1, "{disabled:?}");
        assert_eq!(disabled[0], "", "生产用 disabled 绑定 ⇒ 属性是空串");
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            texts.iter().any(|value| value == REFRESH_BUSY_LABEL),
            "{texts:?}"
        );
        // 转圈 class 挂在图标上。
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "spin"), "{found:?}");
    }

    #[test]
    fn the_idle_state_has_no_disabled_attribute() {
        let node = DataPanels::render(&props());
        let mut attrs = Vec::new();
        attributes(&node, &mut attrs);
        assert!(
            !attrs.iter().any(|(name, _)| name == "button@disabled"),
            "空闲态 ⇒ 不出现 disabled 属性：{attrs:?}"
        );
    }

    #[test]
    fn the_copy_receipt_switches_the_button_content() {
        let mut props = props();
        props.copied_key = String::from("status");
        let node = DataPanels::render(&props);
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.iter().any(|value| value == COPIED_LABEL), "{texts:?}");
        assert!(!texts.iter().any(|value| value == COPY_LABEL), "{texts:?}");
        // 结构分支：只产出**一个**图标节点（`Check` 而不是 `Copy`）。
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            found.iter().any(|token| token == "lucide-check"),
            "{found:?}"
        );
        assert!(
            !found.iter().any(|token| token == "lucide-copy"),
            "{found:?}"
        );
    }

    #[test]
    fn the_copy_error_row_appears_only_with_an_error() {
        let mut props = props();
        props.copy_error = String::from("复制失败（剪贴板不可用）：请手动选中命令文本复制。");
        let node = DataPanels::render(&props);
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "error-hint"), "{found:?}");
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            texts
                .iter()
                .any(|value| value == "复制失败（剪贴板不可用）：请手动选中命令文本复制。"),
            "{texts:?}"
        );
    }

    #[test]
    fn the_workspace_error_row_appears_only_with_an_error() {
        let mut props = props();
        props.workspace_error = String::from("设置工作区目录失败，请检查路径权限后重试。");
        let node = DataPanels::render(&props);
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            texts
                .iter()
                .any(|value| value == "设置工作区目录失败，请检查路径权限后重试。"),
            "{texts:?}"
        );
    }

    #[test]
    fn the_section_switch_picks_the_other_panel() {
        let data = DataPanels::render(&props());
        let mut data_texts = Vec::new();
        text_nodes(&data, &mut data_texts);
        assert!(data_texts.iter().any(|value| value == DATA_BLOCK_TITLE));

        let mut user_props = props();
        user_props.section = DataSection::User;
        let user = DataPanels::render(&user_props);
        let mut user_texts = Vec::new();
        text_nodes(&user, &mut user_texts);
        assert!(user_texts.iter().any(|value| value == USER_BLOCK_TITLE));
        assert!(!user_texts.iter().any(|value| value == DATA_BLOCK_TITLE));
    }

    #[test]
    fn the_section_id_round_trips() {
        assert_eq!(DataSection::from_id("data"), DataSection::Data);
        assert_eq!(DataSection::from_id("user"), DataSection::User);
        // 未知 id ⇒ 落回 `data`（与生产的兜底一致）。
        assert_eq!(DataSection::from_id("nope"), DataSection::Data);
        assert_eq!(DataSection::User.id(), "user");
    }

    #[test]
    fn the_component_is_a_pure_function_of_its_props() {
        let plain = props();
        let first = DataPanels::render(&plain);
        let second = DataPanels::render(&plain);
        let mut first_classes = Vec::new();
        let mut second_classes = Vec::new();
        classes(&first, &mut first_classes);
        classes(&second, &mut second_classes);
        assert_eq!(first_classes, second_classes, "class 集合相同");
        let mut first_texts = Vec::new();
        let mut second_texts = Vec::new();
        text_nodes(&first, &mut first_texts);
        text_nodes(&second, &mut second_texts);
        assert_eq!(first_texts, second_texts, "文本节点逐字相同");
        let mut first_attrs = Vec::new();
        let mut second_attrs = Vec::new();
        attributes(&first, &mut first_attrs);
        attributes(&second, &mut second_attrs);
        assert_eq!(first_attrs, second_attrs, "属性逐字相同");
    }

    #[test]
    fn the_component_carries_no_colour_or_duration_literals() {
        // 令牌纪律：组件里不许出现颜色字面量。
        let node = DataPanels::render(&props());
        let mut attrs = Vec::new();
        attributes(&node, &mut attrs);
        for (name, value) in attrs {
            assert!(
                !value.starts_with('#') && !value.starts_with("rgb"),
                "属性 `{name}` 出现了颜色字面量：{value}"
            );
        }
    }

    /// 取第一个带指定 class 的元素上的处理器。
    fn first_handler(node: &VNode, class: &str) -> Option<apeireth_ui_dom::ElementHandler> {
        if let VNode::Element(element) = node {
            if node.class_tokens().iter().any(|token| token == class) {
                return element.handlers.first().map(|(_, handler)| handler.clone());
            }
        }
        for child in node.children() {
            if let Some(handler) = first_handler(child, class) {
                return Some(handler);
            }
        }
        None
    }
}
