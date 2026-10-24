#!/usr/bin/env node
/**
 * 抽取试点对照所需的**单一真源资产**。
 *
 * ## 抽什么
 *
 * | 资产 | 来源 | 去向 |
 * |---|---|---|
 * | 卡片 scoped CSS | 生产组件的 `<style>` 块（**原文**） | `public/pilot-card.css`（引擎页注入） |
 * | 全局设计令牌 CSS | `frontend/companion-desktop/src/lib/design/*.css` | `public/design/*.css`（**只读复制**，两侧同源） |
 * | 图标几何数据 | 上游图标包的 `dist/icons/*.svelte` 里的 `iconNode` | `crates-ui/apeireth-ui-kit/src/icon_data.rs`（生成物） |
 *
 * ## 纪律
 *
 * - **不手抄**：全部由本脚本从源头抽取；
 * - **不改生产文件**：只读打开；
 * - **许可单一**：图标数据生成物带上游许可署名（ISC + Feather 的 MIT 部分）。
 *
 * ## 用法
 *
 * ```bash
 * node scripts/extract-assets.mjs          # 写入
 * node scripts/extract-assets.mjs --check  # 只校验（漂移 ⇒ 退出码 1）
 * ```
 */

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const HARNESS = resolve(HERE, "..");
const REPO = resolve(HARNESS, "../../..");

/** 生产前端根（**只读**）。 */
const FRONTEND = join(REPO, "frontend/companion-desktop");
/** 试点组件（**只读**）。 */
const COMPONENT = join(FRONTEND, "src/lib/components/ApprovalRequestCard.svelte");
/** 设计令牌层（**只读**）。 */
const DESIGN_DIR = join(FRONTEND, "src/lib/design");
/** 上游图标包（**只读**）。 */
const ICON_DIR = join(FRONTEND, "node_modules/lucide-svelte/dist/icons");
/** 上游许可文件（**只读**）。 */
const ICON_LICENSE = join(FRONTEND, "node_modules/lucide-svelte/LICENSE");

/** 试点用到的图标（与生产组件的 import 一一对应）。 */
const ICONS = ["shield-check", "shield-alert", "x", "copy", "check"];

/** 生成物目标。 */
const PUBLIC_DIR = join(HARNESS, "public");
const CARD_CSS = join(PUBLIC_DIR, "pilot-card.css");
const DESIGN_OUT = join(PUBLIC_DIR, "design");
const ICON_RS = join(REPO, "crates-ui/apeireth-ui-kit/src/icon_data.rs");
const MANIFEST = join(PUBLIC_DIR, "provenance.json");
/** 固定数据（两侧同源）。 */
const FIXTURE_JSON = join(HARNESS, "fixtures/pilot-request.json");
/** 固定数据的 Rust 生成物。 */
const FIXTURE_RS = join(REPO, "crates-ui/apeireth-ui-demo/src/fixture_generated.rs");

const checkOnly = process.argv.includes("--check");

/** 读文件（不存在 ⇒ 明确报错，不静默）。 */
function readOrFail(path, what) {
  if (!existsSync(path)) {
    throw new Error(`${what} 不存在：${path}`);
  }
  return readFileSync(path, "utf8");
}

/** 内容指纹（sha256 前 16 位）。 */
function digest(text) {
  return createHash("sha256").update(text).digest("hex").slice(0, 16);
}

/** 写文件（`--check` 模式下只比对）。 */
function emit(path, content, label) {
  const current = existsSync(path) ? readFileSync(path, "utf8") : null;
  if (current === content) {
    console.log(`  ✓ ${label} 未漂移`);
    return;
  }
  if (checkOnly) {
    throw new Error(`${label} 已漂移：${path}（请重跑抽取脚本）`);
  }
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, content, "utf8");
  console.log(`  → ${label} 已写入 ${path}`);
}

// ── ① 卡片 scoped CSS（原文抽取） ────────────────────────────────────────────
function extractCardCss() {
  const source = readOrFail(COMPONENT, "试点组件");
  const open = source.indexOf("<style>");
  const close = source.indexOf("</style>");
  if (open < 0 || close < 0 || close < open) {
    throw new Error("试点组件里找不到 `<style>` 块");
  }
  // **原文**（含缩进）：CSS 资产逐字来自生产组件，不复制、不手抄、不改。
  const css = source.slice(open + "<style>".length, close);
  emit(CARD_CSS, css, "卡片 scoped CSS");
  return css;
}

// ── ② 设计令牌 CSS（只读复制） ───────────────────────────────────────────────
function copyDesignCss() {
  const files = readdirSync(DESIGN_DIR)
    .filter((name) => name.endsWith(".css"))
    .sort();
  for (const name of files) {
    const content = readOrFail(join(DESIGN_DIR, name), "设计令牌 CSS");
    emit(join(DESIGN_OUT, name), content, `设计令牌 ${name}`);
  }
  return files;
}

// ── ③ 图标几何数据（从上游包抽取） ───────────────────────────────────────────
function extractIcons() {
  const license = readOrFail(ICON_LICENSE, "上游许可文件");
  const entries = ICONS.map((name) => {
    const source = readOrFail(join(ICON_DIR, `${name}.svelte`), `图标 ${name}`);
    const match = source.match(/const iconNode = (\[[\s\S]*?\]);/);
    if (!match) {
      throw new Error(`图标 ${name} 里找不到 iconNode`);
    }
    // 上游的 iconNode 是**合法 JSON**（键带引号）⇒ 直接解析，不 eval。
    const nodes = JSON.parse(match[1]);
    return { name, nodes };
  });

  const rust = renderIconRs(entries, license);
  emit(ICON_RS, rust, "图标数据（Rust 生成物）");
  return entries;
}

/** 生成 Rust 图标数据文件。 */
function renderIconRs(entries, license) {
  const constName = (name) =>
    `ICON_${name.replace(/[^a-z0-9]+/gi, "_").toUpperCase()}`;
  const lines = [];
  lines.push("//! 图标数据（**自动生成** —— 勿手改）。");
  lines.push("//!");
  lines.push("//! 生成器：`crates-ui/apeireth-ui-demo/harness/scripts/extract-assets.mjs`");
  lines.push("//! 来源：`frontend/companion-desktop/node_modules/lucide-svelte/dist/icons/*.svelte`");
  lines.push("//!");
  lines.push("//! 上游许可（ISC；其中 Feather 部分为 MIT）—— 原文如下：");
  lines.push("//!");
  for (const line of license.split("\n")) {
    // 去行尾空白（rustfmt 会把它当 diff ⇒ 生成物必须是 rustfmt 干净的）。
    const trimmed = line.replace(/\s+$/, "");
    lines.push(trimmed ? `//! ${trimmed}` : "//!");
  }
  lines.push("");
  lines.push("use crate::icon::{IconData, IconNode};");
  lines.push("");
  for (const entry of entries) {
    lines.push(`/// 图标 \`${entry.name}\`。`);
    lines.push("#[rustfmt::skip]");
    lines.push(`pub const ${constName(entry.name)}: IconData = IconData {`);
    lines.push(`    name: "${entry.name}",`);
    lines.push(`    class: "lucide-icon lucide lucide-${entry.name}",`);
    lines.push("    nodes: &[");
    for (const [tag, attrs] of entry.nodes) {
      const attrPairs = Object.entries(attrs)
        .map(([key, value]) => `("${key}", "${escapeRust(String(value))}")`)
        .join(", ");
      lines.push(
        `        IconNode { tag: "${tag}", attrs: &[${attrPairs}] },`,
      );
    }
    lines.push("    ],");
    lines.push("};");
    lines.push("");
  }
  return lines.join("\n");
}

/** Rust 字符串转义。 */
function escapeRust(text) {
  return text.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

// ── ④ 固定数据（JSON ⇒ Rust 生成物） ─────────────────────────────────────────
function extractFixture() {
  const raw = readOrFail(FIXTURE_JSON, "固定数据");
  const fixture = JSON.parse(raw);
  const lines = [];
  lines.push("//! 固定数据（**自动生成** —— 勿手改）。");
  lines.push("//!");
  lines.push("//! 来源：`crates-ui/apeireth-ui-demo/harness/fixtures/pilot-request.json`");
  lines.push("//! 生成器：`crates-ui/apeireth-ui-demo/harness/scripts/extract-assets.mjs`");
  lines.push("");
  lines.push("use crate::approval_card::ApprovalItem;");
  lines.push("");
  lines.push("/// 对照用的固定审批项（两侧喂同一份数据）。");
  lines.push("pub fn pilot_item() -> ApprovalItem {");
  lines.push("    ApprovalItem {");
  const optional = (key, rustKey) => {
    const value = fixture[key];
    if (value === undefined || value === null) {
      lines.push(`        ${rustKey}: None,`);
    } else if (typeof value === "number") {
      lines.push(`        ${rustKey}: Some(${value}),`);
    } else {
      lines.push(`        ${rustKey}: Some(String::from(${JSON.stringify(value)})),`);
    }
  };
  optional("title", "title");
  optional("commandText", "command_text");
  optional("argumentsSummary", "arguments_summary");
  optional("reason", "reason");
  optional("createdAt", "created_at");
  optional("sandbox", "sandbox");
  optional("cwd", "cwd");
  optional("isolation", "isolation");
  lines.push("    }");
  lines.push("}");
  lines.push("");
  emit(FIXTURE_RS, lines.join("\n"), "固定数据（Rust 生成物）");
  return fixture;
}

// ── 主流程 ────────────────────────────────────────────────────────────────────
function main() {
  console.log(`仓库根：${REPO}`);
  console.log(`生产前端（只读）：${FRONTEND}`);
  const css = extractCardCss();
  const designFiles = copyDesignCss();
  const icons = extractIcons();
  const fixture = extractFixture();

  const manifest = {
    note: "试点对照资产的来源指纹（单一真源的可复核证据）",
    component: {
      path: "frontend/companion-desktop/src/lib/components/ApprovalRequestCard.svelte",
      scoped_css_sha256_16: digest(css),
    },
    design_css: designFiles.map((name) => ({
      path: `frontend/companion-desktop/src/lib/design/${name}`,
      sha256_16: digest(readOrFail(join(DESIGN_DIR, name), "设计令牌 CSS")),
    })),
    fixture: {
      path: "crates-ui/apeireth-ui-demo/harness/fixtures/pilot-request.json",
      sha256_16: digest(readOrFail(FIXTURE_JSON, "固定数据")),
      created_at_utc: new Date(fixture.createdAt).toISOString(),
    },
    icons: icons.map((entry) => ({
      path: `frontend/companion-desktop/node_modules/lucide-svelte/dist/icons/${entry.name}.svelte`,
      nodes: entry.nodes.length,
      sha256_16: digest(
        readOrFail(join(ICON_DIR, `${entry.name}.svelte`), "图标"),
      ),
    })),
  };
  emit(MANIFEST, `${JSON.stringify(manifest, null, 2)}\n`, "来源指纹清单");

  console.log("抽取完成。");
}

try {
  main();
} catch (error) {
  console.error(`抽取失败：${error.message}`);
  process.exit(1);
}
