#!/usr/bin/env node
/**
 * 抽取状态条面的**单一真源资产**（本地抽取器；**不碰共享抽取器**）。
 *
 * ## 为什么是本地抽取器
 *
 * 共享抽取器（`harness/scripts/extract-assets.mjs`）是**共享点**（别的面正在
 * 施工）⇒ 本面**不改**它。本脚本吃**同一份 manifest 形状**
 * （`face.manifest.json`）⇒ 主线日后给共享抽取器补 `--manifest` 模式时，本
 * 面的 manifest 可**原样复用**。
 *
 * ## 抽什么
 *
 * | 资产 | 来源 | 去向 |
 * |---|---|---|
 * | 组件 scoped CSS | 生产组件的 `<style>` 块（**原文**） | `public/<slug>.css`（引擎页注入） |
 * | 设计令牌 CSS | `frontend/companion-desktop/src/lib/design/*.css` | `public/design/*.css`（**只读复制**） |
 * | 图标几何数据 | 上游图标包的 `dist/icons/*.svelte` 里的 `iconNode` | `crates-ui/apeireth-ui-kit/src/icons_status_bar.rs`（生成物） |
 * | 固定数据 | `fixtures/statusbar-cases.json` | `engine-host/src/fixture_generated.rs`（生成物） |
 *
 * ## 纪律
 *
 * - **不手抄**：全部由本脚本从源头抽取；
 * - **不改生产文件**：只读打开；
 * - **作用域化**：与组件编译器同构（见 `scopeCss`）。
 *
 * ## 用法
 *
 * ```bash
 * node scripts/extract-statusbar-assets.mjs          # 写入
 * node scripts/extract-statusbar-assets.mjs --check  # 只校验（漂移 ⇒ 退出码 1）
 * ```
 */

import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const FACE = resolve(HERE, "..");
const REPO = resolve(FACE, "../../../..");

/** 生产前端根（**只读**）。 */
const FRONTEND = join(REPO, "frontend/companion-desktop");
/** 设计令牌层（**只读**）。 */
const DESIGN_DIR = join(FRONTEND, "src/lib/design");
/** 上游图标包（**只读**）。 */
const ICON_DIR = join(FRONTEND, "node_modules/lucide-svelte/dist/icons");
/** 上游许可文件（**只读**）。 */
const ICON_LICENSE = join(FRONTEND, "node_modules/lucide-svelte/LICENSE");

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
 * `cargo fmt --all -- --check` 是门禁 ⇒ **生成物必须 rustfmt 干净**。
 * rustfmt 缺失 / 失败 ⇒ **显式报错**（不静默跳过）。
 */
function formatRust(content, label) {
  const result = spawnSync("rustfmt", ["--edition", "2021", "--emit", "stdout"], {
    input: content,
    encoding: "utf8",
  });
  if (result.error) {
    throw new Error(`${label} 需要 rustfmt（生成物必须 rustfmt 干净）：${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(`${label} 的 rustfmt 失败（退出码 ${result.status}）：\n${result.stderr}`);
  }
  return result.stdout;
}

/** 写 Rust 生成物（先过 rustfmt ⇒ 门禁 `cargo fmt --check` 恒过）。 */
function emitRust(path, content, label) {
  emit(path, formatRust(content, label), label);
}

// ── ① 组件 scoped CSS（原文抽取 + 作用域化） ─────────────────────────────────

/**
 * 抽一个生产组件的 `<style>` 块（**原文**，含缩进）。
 */
function readStyleBlock(component) {
  const path = join(REPO, component.path);
  const source = readOrFail(path, `生产组件 ${component.path}`);
  const open = source.indexOf("<style>");
  const close = source.indexOf("</style>");
  if (open < 0 || close < 0 || close < open) {
    throw new Error(`生产组件 ${component.path} 里找不到 \`<style>\` 块`);
  }
  return source.slice(open + "<style>".length, close);
}

/**
 * 把组件样式**作用域化**（与组件编译器的做法同构）。
 *
 * | 情形 | 处理 |
 * |---|---|
 * | 选择器**以根 class 开头**（根自身的规则） | **原样保留** |
 * | 其余选择器 | **前缀根 class**（`X` ⇒ `根 X`）⇒ 只命中本组件内的元素 |
 * | `:global(…)` | 去掉括号（`X` ⇒ `X`）⇒ 保持**全局** |
 * | `@` 规则（`@keyframes` 等） | **原样保留** |
 * | **注释** | **原样保留**（提到规则之前；见 `extractComments`） |
 *
 * ⚠️ **注释必须先摘出来**：否则「根规则前的注释」会让根检测失效 ⇒ 根选择器被
 * 改写成 `.根 .根`（后代）⇒ **根样式整条丢失**。
 * （共享抽取器的 `scopeCss` **没有**这一步 ⇒ 那是一条**潜伏的**共享缺陷：
 *  批一/批二的组件样式块里恰好没有注释 ⇒ 没被触发。）
 *
 * ⚠️ **本 manifest 里 `scope: null` 的组件**（状态徽标）**不做作用域化**：
 * 它的变体规则（`.amber` 等）作用在**根元素自身**
 * （`class="status-badge amber normal"`）⇒ 前缀法会改写成后代选择器
 * （`.status-badge .amber`）⇒ **样式全丢**。
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
    const { comments, selector } = extractComments(selectorText);
    const head = comments.length > 0 ? `\n${comments.join("\n")}\n` : "";
    out += head + rewriteSelector(selector, rootSelector) + body;
    index = close + 1;
  }
  return out;
}

/**
 * 把一段「注释 + 选择器」文本里的**注释摘出来**（原文保留）。
 *
 * 注释在 CSS 里只是分隔符 ⇒ 摘出来前置**不改变**渲染结果；但它会让「选择器
 * 以根 class 开头」的判定失效 ⇒ 必须先摘。
 */
function extractComments(raw) {
  const comments = [];
  const selector = raw.replace(/\/\*[\s\S]*?\*\//g, (match) => {
    comments.push(match);
    return " ";
  });
  return { comments, selector };
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

// ── ② 图标（**扫描**生产组件的图标导入 ⇒ 本面 0 图标） ──────────────────────

/**
 * 扫描一个生产组件里从图标包导入的名字。
 *
 * 导入形态（生产侧）：`import { A, B } from 'lucide-svelte';`
 */
function scanIconImports(source) {
  const names = [];
  const pattern = /import\s*\{([^}]*)\}\s*from\s*['"]lucide-svelte['"]/g;
  let match = pattern.exec(source);
  while (match) {
    for (const raw of match[1].split(",")) {
      const name = raw.trim().split(/\s+as\s+/)[0].trim();
      if (name) {
        names.push(name);
      }
    }
    match = pattern.exec(source);
  }
  return names;
}

/** 图标别名（PascalCase ⇒ kebab）。 */
function iconAlias(name) {
  return name
    .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1-$2")
    .toLowerCase();
}

/** 解析一个图标别名 ⇒ 真实 `.svelte` 文件（跟随 `.js` 再导出）。 */
function resolveIconFile(alias) {
  const direct = join(ICON_DIR, `${alias}.svelte`);
  if (existsSync(direct)) {
    return direct;
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
    return join(ICON_DIR, target);
  }
  if (target.endsWith(".js")) {
    return resolveIconFile(target.replace(/\.js$/, ""));
  }
  throw new Error(`图标别名 ${alias} 的再导出目标不存在：${target}`);
}

/** 解析图标文件 ⇒ `{ name, nodes }`（`name` 取真实 `.svelte` 的 `<Icon name>`）。 */
function parseIcon(alias) {
  const file = resolveIconFile(alias);
  const source = readOrFail(file, `图标 ${alias}`);
  const nodeMatch = source.match(/const iconNode = (\[[\s\S]*?\]);/);
  if (!nodeMatch) {
    throw new Error(`图标 ${file} 里找不到 iconNode`);
  }
  const nodes = JSON.parse(nodeMatch[1]);
  const nameMatch = source.match(/<Icon\s+name="([^"]+)"/);
  if (!nameMatch) {
    throw new Error(`图标 ${file} 里找不到 \`<Icon name="…">\``);
  }
  return { alias, file, name: nameMatch[1], nodes };
}

/** 生成图标数据文件（**零图标**时只留说明 ⇒ 不产生未使用的 import）。 */
function renderIconRs(entries) {
  const constName = (name) => `ICON_${name.replace(/[^a-z0-9]+/gi, "_").toUpperCase()}`;
  const lines = [];
  lines.push("//! 图标数据（**自动生成** —— 勿手改）。");
  lines.push("//!");
  lines.push(
    "//! 生成器：`crates-ui/apeireth-ui-demo/harness/statusbar/scripts/extract-statusbar-assets.mjs`",
  );
  lines.push("//! 来源：`frontend/companion-desktop/node_modules/lucide-svelte/dist/icons/*.svelte`");
  lines.push("//!");
  if (entries.length === 0) {
    lines.push("//! ## 本面（状态条面）**零图标**");
    lines.push("//!");
    lines.push(
      "//! 生产组件**没有**从图标包引入任何图标（状态条只有 `import type`，徽标无 import）",
    );
    lines.push("//! ⇒ 本文件**不含**任何图标数据。");
    lines.push("//!");
    lines.push("//! ⚠️ 这份「空」是**漂移哨兵**：日后生产组件一旦引入图标，抽取器会把图标");
    lines.push("//! 数据写进本文件 ⇒ `--check` 立刻报漂移。");
    lines.push("");
    return lines.join("\n");
  }
  lines.push("//! 上游许可（ISC；其中 Feather 部分为 MIT）—— 原文如下：");
  lines.push("//!");
  const license = readOrFail(ICON_LICENSE, "上游许可文件");
  for (const line of license.split("\n")) {
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
      lines.push(`        IconNode { tag: "${tag}", attrs: &[${attrPairs}] },`);
    }
    lines.push("    ],");
    lines.push("};");
    lines.push("");
  }
  return lines.join("\n");
}

/** Rust 字符串转义。 */
function escapeRust(text) {
  let out = "";
  for (const ch of text) {
    if (ch === "\\") {
      out += "\\\\";
    } else if (ch === '"') {
      out += '\\"';
    } else if (ch === "\n") {
      out += "\\n";
    } else if (ch === "\r") {
      out += "\\r";
    } else if (ch === "\t") {
      out += "\\t";
    } else if (ch < " ") {
      out += `\\u{${ch.codePointAt(0).toString(16).toUpperCase()}}`;
    } else {
      out += ch;
    }
  }
  return out;
}

/** Rust 字符串字面量（含引号）。 */
function rustString(text) {
  return `"${escapeRust(text)}"`;
}

// ── ③ 固定数据（JSON ⇒ Rust 生成物） ─────────────────────────────────────────

/** 色态变体名。 */
function toneVariant(tone) {
  const table = { quiet: "Quiet", warn: "Warn", danger: "Danger" };
  const variant = table[tone];
  if (!variant) {
    throw new Error(`色态未知：${tone}`);
  }
  return variant;
}

/** 徽标变体名。 */
function badgeVariant(variant) {
  const table = {
    amber: "Amber",
    green: "Green",
    blue: "Blue",
    danger: "Danger",
    neutral: "Neutral",
    dim: "Dim",
  };
  const name = table[variant];
  if (!name) {
    throw new Error(`徽标变体未知：${variant}`);
  }
  return name;
}

/** 徽标尺寸名。 */
function badgeSize(size) {
  const table = { small: "Small", normal: "Normal" };
  const name = table[size];
  if (!name) {
    throw new Error(`徽标尺寸未知：${size}`);
  }
  return name;
}

/** 生成固定数据的 Rust 文件。 */
function renderFixtureRs(fixture) {
  const cases = fixture.cases;
  if (!Array.isArray(cases) || cases.length === 0) {
    throw new Error("固定数据里没有 `cases` 数组");
  }
  const lines = [];
  lines.push("//! 固定数据（**自动生成** —— 勿手改）。");
  lines.push("//!");
  lines.push(
    "//! 来源：`crates-ui/apeireth-ui-demo/harness/statusbar/fixtures/statusbar-cases.json`",
  );
  lines.push(
    "//! 生成器：`crates-ui/apeireth-ui-demo/harness/statusbar/scripts/extract-statusbar-assets.mjs`",
  );
  lines.push("//!");
  lines.push("use crate::fixture::{StatusBarCase, StatusBarCaseItem};");
  lines.push(
    "use apeireth_ui_kit::status_bar::{StatusBadgeSize, StatusBadgeVariant, StatusTone};",
  );
  lines.push("");
  lines.push("/// 对照用的固定用例（两侧喂同一份数据）。");
  lines.push("pub fn status_bar_cases() -> Vec<StatusBarCase> {");
  lines.push("    vec![");
  for (const item of cases) {
    lines.push("        StatusBarCase {");
    lines.push(`            id: String::from(${rustString(item.id)}),`);
    lines.push("            items: vec![");
    for (const cell of item.items) {
      lines.push("                StatusBarCaseItem {");
      lines.push(`                    key: String::from(${rustString(cell.key)}),`);
      lines.push(`                    text: String::from(${rustString(cell.text)}),`);
      lines.push(`                    tone: StatusTone::${toneVariant(cell.tone)},`);
      lines.push(`                    title: String::from(${rustString(cell.title)}),`);
      lines.push(`                    clickable: ${cell.clickable ? "true" : "false"},`);
      lines.push(`                    sim: ${cell.sim ? "true" : "false"},`);
      lines.push("                },");
    }
    lines.push("            ],");
    lines.push(`            badge_variant: StatusBadgeVariant::${badgeVariant(item.badge.variant)},`);
    lines.push(`            badge_label: String::from(${rustString(item.badge.label)}),`);
    lines.push(`            badge_size: StatusBadgeSize::${badgeSize(item.badge.size)},`);
    lines.push("        },");
  }
  lines.push("    ]");
  lines.push("}");
  lines.push("");
  return lines.join("\n");
}

// ── 主流程 ────────────────────────────────────────────────────────────────────

function main() {
  const manifestPath = join(FACE, "face.manifest.json");
  const manifest = JSON.parse(readOrFail(manifestPath, "抽取清单"));
  console.log(`仓库根：${REPO}`);
  console.log(`生产前端（只读）：${FRONTEND}`);

  const outDir = resolve(FACE, manifest.out);

  // ① 组件 CSS。
  for (const component of manifest.components) {
    const raw = readStyleBlock(component);
    const css = component.scope ? scopeCss(raw, component.scope) : raw;
    emit(join(outDir, component.css), css, `${component.slug} CSS`);
  }

  // ② 设计令牌 CSS（只读复制）。
  const designOut = resolve(FACE, manifest.design_out);
  for (const name of readdirSync(DESIGN_DIR).filter((n) => n.endsWith(".css")).sort()) {
    emit(join(designOut, name), readOrFail(join(DESIGN_DIR, name), "设计令牌 CSS"), `设计令牌 ${name}`);
  }

  // ③ 图标：扫描生产组件的图标导入（**清单与扫描结果必须一致**）。
  const scanned = [];
  for (const component of manifest.components) {
    const source = readOrFail(join(REPO, component.path), `生产组件 ${component.path}`);
    for (const name of scanIconImports(source)) {
      scanned.push({ alias: iconAlias(name), source: component.path });
    }
  }
  const declared = (manifest.icons ?? []).map((alias) => ({ alias, source: "manifest" }));
  const scannedAliases = scanned.map((entry) => entry.alias).sort();
  const declaredAliases = declared.map((entry) => entry.alias).sort();
  if (JSON.stringify(scannedAliases) !== JSON.stringify(declaredAliases)) {
    throw new Error(
      `图标清单与生产组件的实际导入不一致：清单 ${JSON.stringify(declaredAliases)}` +
        ` / 实测 ${JSON.stringify(scannedAliases)}（请更新 manifest 的 icons）`,
    );
  }
  const entries = declaredAliases.map((alias) => parseIcon(alias));
  emitRust(resolve(FACE, manifest.icons_out), renderIconRs(entries), "图标数据（Rust 生成物）");

  // ④ 固定数据。
  const fixture = JSON.parse(readOrFail(resolve(FACE, manifest.fixture), "固定数据"));
  emitRust(
    resolve(FACE, manifest.fixture_out),
    renderFixtureRs(fixture),
    "固定数据（Rust 生成物）",
  );

  console.log("抽取完成。");
}

try {
  main();
} catch (error) {
  console.error(`抽取失败：${error.message}`);
  process.exit(1);
}
