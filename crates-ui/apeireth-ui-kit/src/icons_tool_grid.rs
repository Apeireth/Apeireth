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

/// 图标 `search`。
#[rustfmt::skip]
pub const ICON_SEARCH: IconData = IconData {
    name: "search",
    class: "lucide-icon lucide lucide-search",
    nodes: &[
        IconNode { tag: "circle", attrs: &[("cx", "11"), ("cy", "11"), ("r", "8")] },
        IconNode { tag: "path", attrs: &[("d", "m21 21-4.3-4.3")] },
    ],
};

/// 图标 `cpu`。
#[rustfmt::skip]
pub const ICON_CPU: IconData = IconData {
    name: "cpu",
    class: "lucide-icon lucide lucide-cpu",
    nodes: &[
        IconNode { tag: "rect", attrs: &[("width", "16"), ("height", "16"), ("x", "4"), ("y", "4"), ("rx", "2")] },
        IconNode { tag: "rect", attrs: &[("width", "6"), ("height", "6"), ("x", "9"), ("y", "9"), ("rx", "1")] },
        IconNode { tag: "path", attrs: &[("d", "M15 2v2")] },
        IconNode { tag: "path", attrs: &[("d", "M15 20v2")] },
        IconNode { tag: "path", attrs: &[("d", "M2 15h2")] },
        IconNode { tag: "path", attrs: &[("d", "M2 9h2")] },
        IconNode { tag: "path", attrs: &[("d", "M20 15h2")] },
        IconNode { tag: "path", attrs: &[("d", "M20 9h2")] },
        IconNode { tag: "path", attrs: &[("d", "M9 2v2")] },
        IconNode { tag: "path", attrs: &[("d", "M9 20v2")] },
    ],
};

/// 图标 `external-link`。
#[rustfmt::skip]
pub const ICON_EXTERNAL_LINK: IconData = IconData {
    name: "external-link",
    class: "lucide-icon lucide lucide-external-link",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M15 3h6v6")] },
        IconNode { tag: "path", attrs: &[("d", "M10 14 21 3")] },
        IconNode { tag: "path", attrs: &[("d", "M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6")] },
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
