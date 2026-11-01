#!/usr/bin/env node
/**
 * 抽取工具卡面批二（**工具栏 + 工具网格**）的对照资产。
 *
 * ## 抽什么
 *
 * | 资产 | 来源 | 去向 |
 * |---|---|---|
 * | 网格 scoped CSS | 生产 `ToolsView.svelte` 的 `<style>` 块里**属于本面的规则** | `public/toolgrid/tool-grid.css` |
 * | 固定数据 | `fixtures/toolgrid-cases.json` | `crates-ui/apeireth-ui-demo/src/toolgrid_fixture_generated.rs` |
 *
 * ## 为什么自带「规则切分 + 作用域化」
 *
 * 共享抽取器（`extract-assets.mjs --manifest`）按**整个组件**取 `<style>` 块
 * ⇒ 而生产的 `ToolsView.svelte` 里**混了三个面**（工具卡面 / 治理卷宗面 /
 * 状态条面）⇒ 整份抽进来会把**别的面**的样式一起带走（且与别的组件可能同名
 * 冲突 —— 批一踩过：`.tool-body` 的 `gap` 一个 8px 一个 10px）。
 * ⇒ 本脚本按**规则选择器白名单**只取本面的规则，并按「与组件编译器同构」的
 * 办法作用域化（根 class 前缀；`:global(…)` 保持全局；`@` 规则原样）。
 *
 * ## 白名单怎么定
 *
 * 白名单是**选择器原文**（逐字）⇒ 抽取时**逐条比对**；命中不了的规则会被
 * **列出来**（⇒ 报告「没抽走哪些规则」⇒ 别的面的样式不会悄悄跟进来）。
 *
 * ## 纪律
 *
 * - **不手抄**：CSS 逐字来自生产组件；固定数据逐字来自 fixture JSON；
 * - **不改生产文件**：只读打开；
 * - `--check` 模式只校验 ⇒ 漂移 ⇒ 退出码 1。
 *
 * ## 用法
 *
 * ```bash
 * node scripts/extract-toolgrid.mjs          # 写入
 * node scripts/extract-toolgrid.mjs --check  # 只校验（漂移 ⇒ 退出码 1）
 * ```
 */

import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const HARNESS = resolve(HERE, "..");
const REPO = resolve(HARNESS, "../../..");

/** 生产视图（**只读**）：工具栏 + 工具网格所在的那个文件。 */
const VIEW_FILE = join(
  REPO,
  "frontend/companion-desktop/src/lib/views/ToolsView.svelte",
);
/** 固定数据（两侧同源）。 */
const FIXTURE_JSON = join(HARNESS, "fixtures/toolgrid-cases.json");
/** 网格 scoped CSS 产物。 */
const GRID_CSS = join(HARNESS, "public/toolgrid/tool-grid.css");
/** 固定数据的 Rust 生成物。 */
const FIXTURE_RS = join(
  REPO,
  "crates-ui/apeireth-ui-demo/src/toolgrid_fixture_generated.rs",
);
/** 本面资产的溯源记录。 */
const PROVENANCE = join(HARNESS, "public/toolgrid/provenance-toolgrid.json");

/**
 * 本面的规则白名单（**选择器原文**，逐字比对）。
 *
 * 分组：工具栏 / 网格容器 / 网格与卡片。
 */
const SELECTORS = [
  // ── 工具栏 ──
  ".tools-toolbar",
  ".search-input-wrap",
  ":global(.search-icon)",
  ".search-input-wrap input",
  ".search-input-wrap input:focus",
  ".clear-search-btn",
  ".tools-summary-meta",
  ".tools-summary-meta b",
  // ── 网格容器（网格的呈现上下文：内缩 + 滚动）──
  ".tools-container",
  // ── 网格与卡片 ──
  ".tools-grid",
  ".tool-card-box",
  ".tool-card-box:hover",
  ".card-head",
  ".tool-id-row",
  ".tool-icon-box",
  ".tool-title",
  ".tool-desc",
  ".card-foot",
  ".source-pill",
  ".details-link",
  ".details-link:hover",
];

/** 作用域根 class（引擎页的舞台；参考侧不加载本资产）。 */
const SCOPE = ".toolgrid-stage";

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

/** 生成物过一遍 `rustfmt`（门禁 `cargo fmt --check` 恒过）。 */
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

// ── CSS 切分与作用域化 ───────────────────────────────────────────────────────

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

/** 选择器归一（去首尾空白 + 折叠内部空白）。 */
function normalizeSelector(text) {
  return text.trim().replace(/\s+/g, " ");
}

/**
 * 按白名单切分顶层规则。
 *
 * @param {string} css `<style>` 块原文。
 * @param {string[]} selectors 白名单（选择器原文）。
 * @returns {{css: string, kept: string[], dropped: string[]}}
 *   `css` = 命中规则的原文拼接（保序、保缩进）；`dropped` = 未命中的选择器。
 */
function pickRules(css, selectors) {
  const wanted = new Set(selectors.map(normalizeSelector));
  const keptText = [];
  const kept = [];
  const dropped = [];
  let index = 0;
  while (index < css.length) {
    const open = css.indexOf("{", index);
    if (open < 0) {
      break;
    }
    const selectorText = css.slice(index, open);
    const close = matchBrace(css, open);
    const normalized = normalizeSelector(selectorText);
    if (wanted.has(normalized)) {
      // 保留原文（含缩进与换行）⇒ 产物与生产逐字同源。
      keptText.push(selectorText + css.slice(open, close + 1));
      kept.push(normalized);
    } else {
      dropped.push(normalized);
    }
    index = close + 1;
  }
  return { css: keptText.join(""), kept, dropped };
}

/**
 * 把样式**作用域化**（与组件编译器的做法同构）。
 *
 * | 情形 | 处理 |
 * |---|---|
 * | 选择器**以根 class 开头**（根自身的规则） | **原样保留** |
 * | 其余选择器 | **前缀根 class** ⇒ 只命中本组件内的元素 |
 * | `:global(…)` | 去掉括号 ⇒ 保持**全局**（组件编译器的语义） |
 * | `@` 规则（`@keyframes` 等） | **原样保留** |
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

/** 改写一条规则的选择器（保留两侧空白）。 */
function rewriteSelector(selectorText, rootSelector) {
  const leading = selectorText.match(/^\s*/)[0];
  const trailing = selectorText.match(/\s*$/)[0];
  const body = selectorText.slice(leading.length, selectorText.length - trailing.length);
  if (body.startsWith("@")) {
    return selectorText;
  }
  const parts = body.split(",").map((part) => {
    const trimmed = part.trim();
    if (trimmed.includes(":global(")) {
      return part.replace(/:global\(([^)]*)\)/g, "$1");
    }
    if (trimmed.startsWith(rootSelector)) {
      const next = trimmed[rootSelector.length];
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

/** 抽 `<style>` 块原文。 */
function readStyleBlock(path, what) {
  const source = readOrFail(path, what);
  const open = source.indexOf("<style>");
  const close = source.indexOf("</style>");
  if (open < 0 || close < 0 || close < open) {
    throw new Error(`${what} 里找不到 \`<style>\` 块`);
  }
  return source.slice(open + "<style>".length, close);
}

// ── 固定数据 ⇒ Rust ─────────────────────────────────────────────────────────

/** Rust 字符串转义。 */
function escapeRust(text) {
  return text.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

/** 生成固定数据的 Rust 源码。 */
function renderFixtureRs(fixture) {
  const lines = [];
  lines.push("//! 工具卡面固定数据（**自动生成** —— 勿手改）。");
  lines.push("//!");
  lines.push("//! 来源：`crates-ui/apeireth-ui-demo/harness/fixtures/toolgrid-cases.json`");
  lines.push("//! 生成器：`crates-ui/apeireth-ui-demo/harness/scripts/extract-toolgrid.mjs`");
  lines.push("");
  lines.push("use apeireth_ui_kit::tool_grid::ToolSummary;");
  lines.push("use super::toolgrid_fixture::ToolGridCase;");
  lines.push("");
  lines.push("/// 对照用的固定工具清单（两侧喂同一份数据）。");
  lines.push("pub fn toolgrid_tools() -> Vec<ToolSummary> {");
  lines.push("    vec![");
  for (const tool of fixture.tools) {
    lines.push("        ToolSummary {");
    lines.push(`            name: String::from(${JSON.stringify(tool.name)}),`);
    if (tool.description === undefined || tool.description === null) {
      lines.push("            description: None,");
    } else {
      lines.push(
        `            description: Some(String::from(${JSON.stringify(tool.description)})),`,
      );
    }
    lines.push(`            available: ${tool.available === true ? "true" : "false"},`);
    lines.push("        },");
  }
  lines.push("    ]");
  lines.push("}");
  lines.push("");
  lines.push("/// 对照用的固定用例（每个用例 = 一个搜索词）。");
  lines.push("pub fn toolgrid_cases() -> Vec<ToolGridCase> {");
  lines.push("    vec![");
  for (const item of fixture.cases) {
    lines.push("        ToolGridCase {");
    lines.push(`            id: String::from(${JSON.stringify(item.id)}),`);
    lines.push(`            query: String::from(${JSON.stringify(item.query)}),`);
    lines.push("        },");
  }
  lines.push("    ]");
  lines.push("}");
  lines.push("");
  return lines.join("\n");
}

// ── 主流程 ────────────────────────────────────────────────────────────────────

function main() {
  console.log(`仓库根：${REPO}`);
  console.log(`生产视图（只读）：${VIEW_FILE}`);

  // ① 网格 scoped CSS（区块级）。
  const raw = readStyleBlock(VIEW_FILE, "生产视图");
  const picked = pickRules(raw, SELECTORS);
  const missing = SELECTORS.filter(
    (selector) => !picked.kept.includes(normalizeSelector(selector)),
  );
  if (missing.length > 0) {
    throw new Error(
      `白名单里的选择器在生产视图里没找到：${missing.join(" / ")} ⇒ 生产样式已变，请复核白名单`,
    );
  }
  console.log(
    `  · 区块级抽取：命中 ${picked.kept.length} 条 / 未取走 ${picked.dropped.length} 条（属别的面）`,
  );
  for (const selector of picked.dropped) {
    console.log(`      - ${selector}`);
  }
  const scoped = scopeCss(picked.css, SCOPE);
  emit(GRID_CSS, scoped, "工具网格 scoped CSS");

  // ② 固定数据 ⇒ Rust。
  const fixture = JSON.parse(readOrFail(FIXTURE_JSON, "工具卡面固定数据"));
  const rs = formatRust(renderFixtureRs(fixture), "工具卡面固定数据（Rust 生成物）");
  emit(FIXTURE_RS, rs, "工具卡面固定数据（Rust 生成物）");

  // ③ 溯源记录。
  const provenance = {
    note: "本面资产的来源指纹（单一真源的可复核证据）",
    view: {
      path: "frontend/companion-desktop/src/lib/views/ToolsView.svelte",
      block_selectors: SELECTORS,
      excluded_selectors: picked.dropped,
      scoped_css_sha256_16: digest(scoped),
    },
    fixture: {
      path: "crates-ui/apeireth-ui-demo/harness/fixtures/toolgrid-cases.json",
      sha256_16: digest(readOrFail(FIXTURE_JSON, "工具卡面固定数据")),
      tool_count: fixture.tools.length,
      case_count: fixture.cases.length,
    },
  };
  emit(
    PROVENANCE,
    `${JSON.stringify(provenance, null, 2)}\n`,
    "本面溯源记录",
  );

  console.log("抽取完成。");
}

try {
  main();
} catch (error) {
  console.error(`抽取失败：${error.message}`);
  process.exit(1);
}
