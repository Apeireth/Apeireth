#!/usr/bin/env node
/**
 * 抽取对照所需的**单一真源资产**。
 *
 * ## 抽什么
 *
 * | 资产 | 来源 | 去向 |
 * |---|---|---|
 * | 卡片 scoped CSS | 生产组件的 `<style>` 块（**原文**） | `public/<slug>-card.css`（引擎页注入） |
 * | 作用域规则 | 同上的 `:global(...)` 规则（**机械派生**） | `public/<slug>-scope.css` |
 * | 全局设计令牌 CSS | `frontend/companion-desktop/src/lib/design/*.css` | `public/design/*.css`（**只读复制**，两侧同源） |
 * | 图标几何数据 | 上游图标包的 `dist/icons/*.svelte` 里的 `iconNode` | `crates-ui/apeireth-ui-kit/src/icon_data.rs`（生成物） |
 * | 固定数据 | `fixtures/*.json` | `crates-ui/apeireth-ui-demo/src/*_generated.rs`（生成物） |
 *
 * ## 纪律
 *
 * - **不手抄**：全部由本脚本从源头抽取；
 * - **不改生产文件**：只读打开；
 * - **许可单一**：图标数据生成物带上游许可署名（ISC + Feather 的 MIT 部分）。
 *
 * ## 多组件与图标别名
 *
 * - 组件清单在本文件顶部（`COMPONENTS`）⇒ 一处增删 ⇒ 抽取与产物同步。
 * - 图标按**生产导入名**（PascalCase ⇒ kebab）找文件；`.svelte` 不存在时读同名的
 *   `.js` 并跟随 `export { default } from "./X.svelte";` 的再导出 ⇒ 落到**真实**
 *   `.svelte`。
 *   ⚠️ 图标的 `name` / `class` 取自**真实** `.svelte` 里的 `<Icon name="…">`
 *   （例：`check-circle-2` ⇒ `circle-check.svelte` ⇒ `lucide-circle-check`）——
 *   生产渲染出来的就是这个 class，用别名会像素不一致。
 *
 * ## 用法
 *
 * ```bash
 * node scripts/extract-assets.mjs          # 写入
 * node scripts/extract-assets.mjs --check  # 只校验（漂移 ⇒ 退出码 1）
 * ```
 */

import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const HARNESS = resolve(HERE, "..");
const REPO = resolve(HARNESS, "../../..");

/** 生产前端根（**只读**）。 */
const FRONTEND = join(REPO, "frontend/companion-desktop");
/** 生产组件目录（**只读**）。 */
const COMPONENT_DIR = join(FRONTEND, "src/lib/components");
/** 设计令牌层（**只读**）。 */
const DESIGN_DIR = join(FRONTEND, "src/lib/design");
/** 上游图标包（**只读**）。 */
const ICON_DIR = join(FRONTEND, "node_modules/lucide-svelte/dist/icons");
/** 上游许可文件（**只读**）。 */
const ICON_LICENSE = join(FRONTEND, "node_modules/lucide-svelte/LICENSE");

/**
 * 生产组件清单（**多组件**：试点 + 工具卡面两面）。
 *
 * | slug | 生产组件 | CSS 产物 |
 * |---|---|---|
 * | `pilot` | `ApprovalRequestCard.svelte` | `public/pilot-card.css` |
 * | `tool-card` | `ToolCallCard.svelte` | `public/tool-card.css` |
 * | `lifecycle-card` | `ToolCallLifecycleCard.svelte` | `public/lifecycle-card.css` |
 */
const COMPONENTS = [
  // 试点组件：样式**原文**注入（页面上没有同名 class 冲突 ⇒ 无需作用域化）。
  { slug: "pilot", file: "ApprovalRequestCard.svelte", css: "pilot-card.css", scope: null },
  // 工具卡面两面：两个组件**共用若干 class 名**（`.tool-header` / `.tool-body` /
  // `.section-label` / `.code-block` / `.error-text` …）且**同名规则的取值不同**
  // （例：`.tool-body` 的 `gap` 一个是 8px、一个是 10px）⇒ 必须按组件**作用域化**
  // （与组件编译器的做法同构，见 `scopeCss`）。
  { slug: "tool-card", file: "ToolCallCard.svelte", css: "tool-card.css", scope: ".tool-card" },
  {
    slug: "lifecycle-card",
    file: "ToolCallLifecycleCard.svelte",
    css: "lifecycle-card.css",
    scope: ".tool-lifecycle-card",
  },
];

/**
 * 图标清单（按**生产导入名** ⇒ kebab 别名）。
 *
 * 前 5 个是试点既有图标（生成物的前半段必须逐字不变 ⇒ 顺序不动）。
 * 后 8 个是工具卡面两面用到的图标。
 */
const ICONS = [
  "shield-check",
  "shield-alert",
  "x",
  "copy",
  "check",
  // 工具卡面（`ToolCallCard` 8 个 + `ToolCallLifecycleCard` 2 个）。
  "chevron-down",
  "chevron-right",
  "wrench",
  "check-circle-2",
  "x-circle",
  "loader-2",
  "clock",
  "alert-triangle",
];

/** 生成物目标。 */
const PUBLIC_DIR = join(HARNESS, "public");
const DESIGN_OUT = join(PUBLIC_DIR, "design");
const ICON_RS = join(REPO, "crates-ui/apeireth-ui-kit/src/icon_data.rs");
const MANIFEST = join(PUBLIC_DIR, "provenance.json");
/** 固定数据（两侧同源）。 */
const FIXTURE_JSON = join(HARNESS, "fixtures/pilot-request.json");
/** 固定数据的 Rust 生成物。 */
const FIXTURE_RS = join(REPO, "crates-ui/apeireth-ui-demo/src/fixture_generated.rs");
/** 工具卡面固定数据（两侧同源）。 */
const TOOL_FIXTURE_JSON = join(HARNESS, "fixtures/toolcard-cases.json");
/** 工具卡面固定数据的 Rust 生成物。 */
const TOOL_FIXTURE_RS = join(
  REPO,
  "crates-ui/apeireth-ui-demo/src/toolcard_fixture_generated.rs",
);

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

/**
 * 生成的 Rust 源码过一遍 `rustfmt`。
 *
 * ## 为什么
 *
 * `cargo fmt --all -- --check` 是门禁 ⇒ **生成物必须 rustfmt 干净**。手写的
 * 生成代码要逐行猜 rustfmt 的排版偏好（很容易漂）⇒ 直接让 rustfmt 落盘。
 *
 * ## 纪律
 *
 * - rustfmt 缺失 ⇒ **显式报错**（不静默跳过 ⇒ 不产出未格式化的生成物）；
 * - rustfmt 失败 ⇒ **显式报错**（带 stderr 原文）。
 */
function formatRust(content, label) {
  const result = spawnSync("rustfmt", ["--edition", "2021", "--emit", "stdout"], {
    input: content,
    encoding: "utf8",
  });
  if (result.error) {
    throw new Error(
      `${label} 需要 rustfmt（生成物必须 rustfmt 干净）：${result.error.message}`,
    );
  }
  if (result.status !== 0) {
    throw new Error(
      `${label} 的 rustfmt 失败（退出码 ${result.status}）：\n${result.stderr}`,
    );
  }
  return result.stdout;
}

/** 写 Rust 生成物（先过 rustfmt ⇒ 门禁 `cargo fmt --check` 恒过）。 */
function emitRust(path, content, label) {
  emit(path, formatRust(content, label), label);
}

// ── ① 卡片 scoped CSS（原文抽取） ────────────────────────────────────────────
/**
 * 抽一个生产组件的 `<style>` 块（**原文**，含缩进）。
 *
 * CSS 资产逐字来自生产组件，不复制、不手抄、不改。
 */
function readStyleBlock(component) {
  const path = join(COMPONENT_DIR, component.file);
  const source = readOrFail(path, `生产组件 ${component.file}`);
  const open = source.indexOf("<style>");
  const close = source.indexOf("</style>");
  if (open < 0 || close < 0 || close < open) {
    throw new Error(`生产组件 ${component.file} 里找不到 \`<style>\` 块`);
  }
  return source.slice(open + "<style>".length, close);
}

/**
 * 把组件样式**作用域化**（与组件编译器的做法同构）。
 *
 * ## 为什么需要这一步
 *
 * 组件样式里的 class 名是**组件私有**的 ⇒ 编译器会给每个组件生成一个作用域
 * class，并把选择器改写成「元素 class + 作用域 class」（`.tool-body.svelte-x`）
 * ⇒ 两个组件的同名 class（`.tool-body` 等）**互不干扰**。
 *
 * 引擎页把样式作为**普通样式表**注入 ⇒ 没有作用域 class ⇒ 两个组件的同名
 * 规则会**互相覆盖**（例：`.tool-body` 的 `gap` 一个 8px、一个 10px ⇒ 画面
 * 不一致）。
 *
 * ## 派生规则（确定性）
 *
 * | 情形 | 处理 |
 * |---|---|
 * | 选择器**以根 class 开头**（根自身的规则） | **原样保留** |
 * | 其余选择器 | **前缀根 class**（`X` ⇒ `根 X`）⇒ 只命中本组件内的元素 |
 * | `:global(…)` | 去掉括号（`X` ⇒ `X`）⇒ 保持**全局**（组件编译器的语义） |
 * | `@` 规则（`@keyframes` 等） | **原样保留** |
 *
 * ⚠️ 边界：前缀法与编译器的「元素 class + 作用域 class」在**特异度**上等价
 * （`.根 X` 与 `.X.作用域` 同为两个 class）⇒ 层叠结果一致；但根自身的规则在
 * 前缀法下少一个 class（特异度低一档）⇒ 当前仓的设计令牌层**没有**命中这些
 * 根 class 的规则 ⇒ 无实际差异（`extract-assets.mjs` 的 `--check` 模式可复核
 * 资产漂移）。
 *
 * @param {string} css 组件的 `<style>` 原文。
 * @param {string} rootSelector 根元素的选择器（`.tool-card` 等）。
 * @returns {string} 作用域化后的 CSS。
 */
function scopeCss(css, rootSelector) {
  let out = "";
  let index = 0;
  while (index < css.length) {
    const open = css.indexOf("{", index);
    if (open < 0) {
      out += css.slice(index);
      break;
    }
    const selectorText = css.slice(index, open);
    const close = matchBrace(css, open);
    const body = css.slice(open, close + 1);
    out += rewriteSelector(selectorText, rootSelector) + body;
    index = close + 1;
  }
  return out;
}

/** 找与 `open` 处 `{` 配对的 `}`（按嵌套深度）。 */
function matchBrace(css, open) {
  let depth = 0;
  for (let index = open; index < css.length; index += 1) {
    if (css[index] === "{") {
      depth += 1;
    } else if (css[index] === "}") {
      depth -= 1;
      if (depth === 0) {
        return index;
      }
    }
  }
  throw new Error("CSS 里花括号未配对");
}

/** 改写一条规则的选择器（保留两侧空白）。 */
function rewriteSelector(selectorText, rootSelector) {
  const leading = selectorText.match(/^\s*/)[0];
  const trailing = selectorText.match(/\s*$/)[0];
  const body = selectorText.slice(leading.length, selectorText.length - trailing.length);
  if (body.startsWith("@")) {
    // `@keyframes` 等 at-rule ⇒ 原样。
    return selectorText;
  }
  const parts = body.split(",").map((part) => {
    const trimmed = part.trim();
    if (trimmed.includes(":global(")) {
      // 全局规则 ⇒ 去掉 `:global(…)` 的括号，其余原样。
      return part.replace(/:global\(([^)]*)\)/g, "$1");
    }
    if (trimmed.startsWith(rootSelector)) {
      const next = trimmed[rootSelector.length];
      // 根自身的规则 ⇒ 原样（避免「根套根」匹配不到）。
      if (next === undefined || !/[a-zA-Z0-9_-]/.test(next)) {
        return part;
      }
    }
    const partLeading = part.match(/^\s*/)[0];
    const partTrailing = part.match(/\s*$/)[0];
    const core = part.slice(partLeading.length, part.length - partTrailing.length);
    return `${partLeading}${rootSelector} ${core}${partTrailing}`;
  });
  return `${leading}${parts.join(",")}${trailing}`;
}

/** 抽多个生产组件的 CSS（试点原文；工具卡面作用域化）。 */
function extractComponentCss() {
  const summary = [];
  for (const component of COMPONENTS) {
    const raw = readStyleBlock(component);
    const css = component.scope ? scopeCss(raw, component.scope) : raw;
    emit(join(PUBLIC_DIR, component.css), css, `${component.slug} scoped CSS`);
    summary.push({ component, raw, css });
  }
  return summary;
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
/**
 * 解析一个图标别名 ⇒ 真实 `.svelte` 文件。
 *
 * - `<alias>.svelte` 存在 ⇒ 就是它；
 * - 否则读 `<alias>.js` ⇒ 跟随 `export { default } from "./X.svelte";` 的再导出
 *   （允许多跳）⇒ 落到真实 `.svelte`。
 */
function resolveIconFile(alias) {
  const direct = join(ICON_DIR, `${alias}.svelte`);
  if (existsSync(direct)) {
    return { alias, file: direct };
  }
  const compiled = join(ICON_DIR, `${alias}.js`);
  if (!existsSync(compiled)) {
    throw new Error(`图标别名 ${alias} 既没有 .svelte 也没有 .js（${ICON_DIR}）`);
  }
  const source = readOrFail(compiled, `图标编译产物 ${alias}.js`);
  const match = source.match(/export\s*\{[^}]*\}\s*from\s*"\.\/([^"]+)"/);
  if (!match) {
    throw new Error(`图标编译产物 ${alias}.js 里找不到再导出语句`);
  }
  const target = match[1];
  if (existsSync(join(ICON_DIR, target))) {
    return { alias, file: join(ICON_DIR, target) };
  }
  // 再导出指向的若是另一个 `.js` ⇒ 继续跟随（多跳）。
  if (target.endsWith(".js")) {
    return resolveIconFile(target.replace(/\.js$/, ""));
  }
  throw new Error(`图标别名 ${alias} 的再导出目标不存在：${target}`);
}

/**
 * 解析图标文件 ⇒ `{ alias, name, nodes }`。
 *
 * `name` 取自**真实** `.svelte` 里的 `<Icon name="…">`（渲染出来的 class 后缀）。
 */
function parseIcon(alias) {
  const resolved = resolveIconFile(alias);
  const source = readOrFail(resolved.file, `图标 ${alias}`);
  const nodeMatch = source.match(/const iconNode = (\[[\s\S]*?\]);/);
  if (!nodeMatch) {
    throw new Error(`图标 ${resolved.file} 里找不到 iconNode`);
  }
  // 上游的 iconNode 是**合法 JSON**（键带引号）⇒ 直接解析，不 eval。
  const nodes = JSON.parse(nodeMatch[1]);
  const nameMatch = source.match(/<Icon\s+name="([^"]+)"/);
  if (!nameMatch) {
    throw new Error(`图标 ${resolved.file} 里找不到 \`<Icon name="…">\``);
  }
  return { alias, file: resolved.file, name: nameMatch[1], nodes };
}

function extractIcons() {
  const license = readOrFail(ICON_LICENSE, "上游许可文件");
  const entries = ICONS.map((alias) => parseIcon(alias));

  const rust = renderIconRs(entries, license);
  emitRust(ICON_RS, rust, "图标数据（Rust 生成物）");
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
  emitRust(FIXTURE_RS, lines.join("\n"), "固定数据（Rust 生成物）");
  return fixture;
}

// ── ⑤ 工具卡面固定数据（JSON ⇒ Rust 生成物） ────────────────────────────────
/**
 * 生成工具卡面两面的固定数据。
 *
 * 真源 = `fixtures/toolcard-cases.json`；参考侧直接 import 该 JSON，
 * 引擎侧由本脚本生成 Rust ⇒ 两侧喂**同一份**。
 */
function extractToolCardFixture() {
  const raw = readOrFail(TOOL_FIXTURE_JSON, "工具卡面固定数据");
  const fixture = JSON.parse(raw);
  const cases = fixture.cases;
  if (!Array.isArray(cases) || cases.length === 0) {
    throw new Error("工具卡面固定数据里没有 `cases` 数组");
  }
  const lines = [];
  lines.push("//! 工具卡面固定数据（**自动生成** —— 勿手改）。");
  lines.push("//!");
  lines.push("//! 来源：`crates-ui/apeireth-ui-demo/harness/fixtures/toolcard-cases.json`");
  lines.push("//! 生成器：`crates-ui/apeireth-ui-demo/harness/scripts/extract-assets.mjs`");
  lines.push("");
  lines.push("use crate::toolcard_fixture::ToolCardCase;");
  lines.push("use apeireth_ui_kit::json::JsonValue;");
  lines.push("use apeireth_ui_kit::tool_card::{");
  lines.push("    ToolCallDetails, ToolCallLifecycleCall, ToolCallLifecycleStatus, ToolCallStatus,");
  lines.push("};");
  lines.push("");
  lines.push("/// 对照用的固定卡面用例（两侧喂同一份数据）。");
  lines.push("pub fn tool_card_cases() -> Vec<ToolCardCase> {");
  lines.push("    vec![");
  for (const item of cases) {
    lines.push("        ToolCardCase {");
    lines.push(`            id: String::from(${JSON.stringify(item.id)}),`);
    lines.push("            tool_call: ToolCallDetails {");
    lines.push(`                status: ToolCallStatus::${toolStatusVariant(item.toolCall.status)},`);
    lines.push(`                name: String::from(${JSON.stringify(item.toolCall.name)}),`);
    lines.push(`                duration_ms: ${numberOrNone(item.toolCall.durationMs)},`);
    lines.push(`                args: ${jsonOrNone(item.toolCall.args)},`);
    lines.push(`                raw_args: ${stringOrNone(item.toolCall.rawArgs)},`);
    lines.push(`                result_summary: ${stringOrNone(item.toolCall.resultSummary)},`);
    lines.push(`                result_full: ${jsonOrNone(item.toolCall.resultFull)},`);
    lines.push(`                error: ${stringOrNone(item.toolCall.error)},`);
    lines.push("            },");
    lines.push("            lifecycle_call: ToolCallLifecycleCall {");
    lines.push(`                id: String::from(${JSON.stringify(item.lifecycleCall.id)}),`);
    lines.push(`                name: String::from(${JSON.stringify(item.lifecycleCall.name)}),`);
    lines.push(
      `                status: ToolCallLifecycleStatus::${lifecycleStatusVariant(
        item.lifecycleCall.status,
      )},`,
    );
    lines.push(`                latency_ms: ${numberOrNone(item.lifecycleCall.latencyMs)},`);
    lines.push(`                error: ${stringOrNone(item.lifecycleCall.error)},`);
    lines.push(`                arguments: ${jsonOrNone(item.lifecycleCall.arguments)},`);
    lines.push(`                result: ${jsonOrNone(item.lifecycleCall.result)},`);
    lines.push("            },");
    lines.push("        },");
  }
  lines.push("    ]");
  lines.push("}");
  lines.push("");
  emitRust(TOOL_FIXTURE_RS, lines.join("\n"), "工具卡面固定数据（Rust 生成物）");
  return fixture;
}

/** 工具卡状态枚举变体名。 */
function toolStatusVariant(status) {
  const table = {
    pending: "Pending",
    running: "Running",
    succeeded: "Succeeded",
    failed: "Failed",
    cancelled: "Cancelled",
  };
  const variant = table[status];
  if (!variant) {
    throw new Error(`工具卡状态未知：${status}`);
  }
  return variant;
}

/** 生命周期卡状态枚举变体名。 */
function lifecycleStatusVariant(status) {
  const table = {
    pending: "Pending",
    running: "Running",
    success: "Success",
    error: "Error",
  };
  const variant = table[status];
  if (!variant) {
    throw new Error(`生命周期卡状态未知：${status}`);
  }
  return variant;
}

/** 数字字段 ⇒ Rust 表达式（缺省 ⇒ `None`；整数值补 `.0` ⇒ 浮点字面量）。 */
function numberOrNone(value) {
  if (value === undefined || value === null) {
    return "None";
  }
  return `Some(${floatLiteral(value)})`;
}

/** 数值 ⇒ 浮点字面量文本（整数值补 `.0`）。 */
function floatLiteral(value) {
  return Number.isInteger(value) ? `${value}.0` : String(value);
}

/** 字符串字段 ⇒ Rust 表达式（缺省 ⇒ `None`）。 */
function stringOrNone(value) {
  if (value === undefined || value === null) {
    return "None";
  }
  return `Some(String::from(${JSON.stringify(value)}))`;
}

/** JSON 值字段 ⇒ Rust 表达式（缺省 ⇒ `None`；`null` ⇒ `Some(JsonValue::Null)`）。 */
function jsonOrNone(value) {
  if (value === undefined) {
    return "None";
  }
  return `Some(${jsonExpression(value)})`;
}

/** JSON 值 ⇒ Rust 表达式（`JsonValue` 构造）。 */
function jsonExpression(value) {
  if (value === null) {
    return "JsonValue::Null";
  }
  if (typeof value === "boolean") {
    return `JsonValue::Bool(${value})`;
  }
  if (typeof value === "number") {
    return `JsonValue::Number(${floatLiteral(value)})`;
  }
  if (typeof value === "string") {
    return `JsonValue::String(String::from(${JSON.stringify(value)}))`;
  }
  if (Array.isArray(value)) {
    const items = value.map((item) => jsonExpression(item)).join(", ");
    return `JsonValue::Array(vec![${items}])`;
  }
  const entries = Object.entries(value).map(
    ([key, item]) =>
      `(String::from(${JSON.stringify(key)}), ${jsonExpression(item)})`,
  );
  return `JsonValue::Object(vec![${entries.join(", ")}])`;
}

// ── 主流程 ────────────────────────────────────────────────────────────────────
function main() {
  console.log(`仓库根：${REPO}`);
  console.log(`生产前端（只读）：${FRONTEND}`);
  const components = extractComponentCss();
  const designFiles = copyDesignCss();
  const icons = extractIcons();
  const fixture = extractFixture();
  const toolFixture = extractToolCardFixture();

  const manifest = {
    note: "对照资产的来源指纹（单一真源的可复核证据）",
    components: components.map(({ component, css }) => ({
      path: `frontend/companion-desktop/src/lib/components/${component.file}`,
      slug: component.slug,
      scoped_css_sha256_16: digest(css),
    })),
    design_css: designFiles.map((name) => ({
      path: `frontend/companion-desktop/src/lib/design/${name}`,
      sha256_16: digest(readOrFail(join(DESIGN_DIR, name), "设计令牌 CSS")),
    })),
    fixture: {
      path: "crates-ui/apeireth-ui-demo/harness/fixtures/pilot-request.json",
      sha256_16: digest(readOrFail(FIXTURE_JSON, "固定数据")),
      created_at_utc: new Date(fixture.createdAt).toISOString(),
    },
    toolcard_fixture: {
      path: "crates-ui/apeireth-ui-demo/harness/fixtures/toolcard-cases.json",
      sha256_16: digest(readOrFail(TOOL_FIXTURE_JSON, "工具卡面固定数据")),
      case_count: toolFixture.cases.length,
    },
    icons: icons.map((entry) => ({
      alias: entry.alias,
      path: `frontend/companion-desktop/node_modules/lucide-svelte/dist/icons/${entry.file.split("/").pop()}`,
      name: entry.name,
      nodes: entry.nodes.length,
      sha256_16: digest(readOrFail(entry.file, "图标")),
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
