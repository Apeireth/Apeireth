//! 图标数据（**自动生成** —— 勿手改）。
//!
//! 生成器：`crates-ui/apeireth-ui-demo/harness/governance/scripts/extract-governance-assets.mjs`
//! 来源：`frontend/companion-desktop/node_modules/lucide-svelte/dist/icons/*.svelte`
//!
//! 上游许可（ISC；其中 Feather 部分为 MIT）—— 原文如下：
//!
//! ISC License
//!
//! Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part of Feather (MIT). All other copyright (c) for Lucide are held by Lucide Contributors 2022.
//!
//! Permission to use, copy, modify, and/or distribute this software for any
//! purpose with or without fee is hereby granted, provided that the above
//! copyright notice and this permission notice appear in all copies.
//!
//! THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
//! WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
//! MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
//! ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
//! WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
//! ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
//! OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
//!

use crate::icon::{IconData, IconNode};

/// 图标 `inbox`。
#[rustfmt::skip]
pub const ICON_INBOX: IconData = IconData {
    name: "inbox",
    class: "lucide-icon lucide lucide-inbox",
    nodes: &[
        IconNode { tag: "polyline", attrs: &[("points", "22 12 16 12 14 15 10 15 8 12 2 12")] },
        IconNode { tag: "path", attrs: &[("d", "M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z")] },
    ],
};

/// 图标 `rotate-ccw`。
#[rustfmt::skip]
pub const ICON_ROTATE_CCW: IconData = IconData {
    name: "rotate-ccw",
    class: "lucide-icon lucide lucide-rotate-ccw",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8")] },
        IconNode { tag: "path", attrs: &[("d", "M3 3v5h5")] },
    ],
};

/// 图标 `radio`。
#[rustfmt::skip]
pub const ICON_RADIO: IconData = IconData {
    name: "radio",
    class: "lucide-icon lucide lucide-radio",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M4.9 19.1C1 15.2 1 8.8 4.9 4.9")] },
        IconNode { tag: "path", attrs: &[("d", "M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.5")] },
        IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "12"), ("r", "2")] },
        IconNode { tag: "path", attrs: &[("d", "M16.2 7.8c2.3 2.3 2.3 6.1 0 8.5")] },
        IconNode { tag: "path", attrs: &[("d", "M19.1 4.9C23 8.8 23 15.1 19.1 19")] },
    ],
};

/// 图标 `message-circle-more`。
#[rustfmt::skip]
pub const ICON_MESSAGE_CIRCLE_MORE: IconData = IconData {
    name: "message-circle-more",
    class: "lucide-icon lucide lucide-message-circle-more",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M7.9 20A9 9 0 1 0 4 16.1L2 22Z")] },
        IconNode { tag: "path", attrs: &[("d", "M8 12h.01")] },
        IconNode { tag: "path", attrs: &[("d", "M12 12h.01")] },
        IconNode { tag: "path", attrs: &[("d", "M16 12h.01")] },
    ],
};

/// 图标 `scroll-text`。
#[rustfmt::skip]
pub const ICON_SCROLL_TEXT: IconData = IconData {
    name: "scroll-text",
    class: "lucide-icon lucide lucide-scroll-text",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M15 12h-5")] },
        IconNode { tag: "path", attrs: &[("d", "M15 8h-5")] },
        IconNode { tag: "path", attrs: &[("d", "M19 17V5a2 2 0 0 0-2-2H4")] },
        IconNode { tag: "path", attrs: &[("d", "M8 21h12a2 2 0 0 0 2-2v-1a1 1 0 0 0-1-1H11a1 1 0 0 0-1 1v1a2 2 0 1 1-4 0V5a2 2 0 1 0-4 0v2a1 1 0 0 0 1 1h3")] },
    ],
};

/// 图标 `clock`。
#[rustfmt::skip]
pub const ICON_CLOCK: IconData = IconData {
    name: "clock",
    class: "lucide-icon lucide lucide-clock",
    nodes: &[
        IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "12"), ("r", "10")] },
        IconNode { tag: "polyline", attrs: &[("points", "12 6 12 12 16 14")] },
    ],
};

/// 图标 `circle-alert`。
#[rustfmt::skip]
pub const ICON_CIRCLE_ALERT: IconData = IconData {
    name: "circle-alert",
    class: "lucide-icon lucide lucide-circle-alert",
    nodes: &[
        IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "12"), ("r", "10")] },
        IconNode { tag: "line", attrs: &[("x1", "12"), ("x2", "12"), ("y1", "8"), ("y2", "12")] },
        IconNode { tag: "line", attrs: &[("x1", "12"), ("x2", "12.01"), ("y1", "16"), ("y2", "16")] },
    ],
};

/// 图标 `loader-circle`。
#[rustfmt::skip]
pub const ICON_LOADER_CIRCLE: IconData = IconData {
    name: "loader-circle",
    class: "lucide-icon lucide lucide-loader-circle",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M21 12a9 9 0 1 1-6.219-8.56")] },
    ],
};

/// 图标 `shield-off`。
#[rustfmt::skip]
pub const ICON_SHIELD_OFF: IconData = IconData {
    name: "shield-off",
    class: "lucide-icon lucide lucide-shield-off",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "m2 2 20 20")] },
        IconNode { tag: "path", attrs: &[("d", "M5 5a1 1 0 0 0-1 1v7c0 5 3.5 7.5 7.67 8.94a1 1 0 0 0 .67.01c2.35-.82 4.48-1.97 5.9-3.71")] },
        IconNode { tag: "path", attrs: &[("d", "M9.309 3.652A12.252 12.252 0 0 0 11.24 2.28a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1v7a9.784 9.784 0 0 1-.08 1.264")] },
    ],
};

/// 图标 `cloud-off`。
#[rustfmt::skip]
pub const ICON_CLOUD_OFF: IconData = IconData {
    name: "cloud-off",
    class: "lucide-icon lucide lucide-cloud-off",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "m2 2 20 20")] },
        IconNode { tag: "path", attrs: &[("d", "M5.782 5.782A7 7 0 0 0 9 19h8.5a4.5 4.5 0 0 0 1.307-.193")] },
        IconNode { tag: "path", attrs: &[("d", "M21.532 16.5A4.5 4.5 0 0 0 17.5 10h-1.79A7.008 7.008 0 0 0 10 5.07")] },
    ],
};

/// 图标 `shield-alert`。
#[rustfmt::skip]
pub const ICON_SHIELD_ALERT: IconData = IconData {
    name: "shield-alert",
    class: "lucide-icon lucide lucide-shield-alert",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z")] },
        IconNode { tag: "path", attrs: &[("d", "M12 8v4")] },
        IconNode { tag: "path", attrs: &[("d", "M12 16h.01")] },
    ],
};

/// 图标 `flask-conical`。
#[rustfmt::skip]
pub const ICON_FLASK_CONICAL: IconData = IconData {
    name: "flask-conical",
    class: "lucide-icon lucide lucide-flask-conical",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M14 2v6a2 2 0 0 0 .245.96l5.51 10.08A2 2 0 0 1 18 22H6a2 2 0 0 1-1.755-2.96l5.51-10.08A2 2 0 0 0 10 8V2")] },
        IconNode { tag: "path", attrs: &[("d", "M6.453 15h11.094")] },
        IconNode { tag: "path", attrs: &[("d", "M8.5 2h7")] },
    ],
};

/// 图标 `chevron-down`。
#[rustfmt::skip]
pub const ICON_CHEVRON_DOWN: IconData = IconData {
    name: "chevron-down",
    class: "lucide-icon lucide lucide-chevron-down",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "m6 9 6 6 6-6")] },
    ],
};

/// 图标 `chevron-right`。
#[rustfmt::skip]
pub const ICON_CHEVRON_RIGHT: IconData = IconData {
    name: "chevron-right",
    class: "lucide-icon lucide lucide-chevron-right",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "m9 18 6-6-6-6")] },
    ],
};
