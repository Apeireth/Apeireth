//! 图标数据（**自动生成** —— 勿手改）。
//!
//! 生成器：`crates-ui/apeireth-ui-demo/harness/scripts/extract-assets.mjs`
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

/// 图标 `shield-check`。
#[rustfmt::skip]
pub const ICON_SHIELD_CHECK: IconData = IconData {
    name: "shield-check",
    class: "lucide-icon lucide lucide-shield-check",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z")] },
        IconNode { tag: "path", attrs: &[("d", "m9 12 2 2 4-4")] },
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

/// 图标 `x`。
#[rustfmt::skip]
pub const ICON_X: IconData = IconData {
    name: "x",
    class: "lucide-icon lucide lucide-x",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M18 6 6 18")] },
        IconNode { tag: "path", attrs: &[("d", "m6 6 12 12")] },
    ],
};

/// 图标 `copy`。
#[rustfmt::skip]
pub const ICON_COPY: IconData = IconData {
    name: "copy",
    class: "lucide-icon lucide lucide-copy",
    nodes: &[
        IconNode { tag: "rect", attrs: &[("width", "14"), ("height", "14"), ("x", "8"), ("y", "8"), ("rx", "2"), ("ry", "2")] },
        IconNode { tag: "path", attrs: &[("d", "M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2")] },
    ],
};

/// 图标 `check`。
#[rustfmt::skip]
pub const ICON_CHECK: IconData = IconData {
    name: "check",
    class: "lucide-icon lucide lucide-check",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M20 6 9 17l-5-5")] },
    ],
};
