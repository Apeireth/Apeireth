#!/usr/bin/env node
/**
 * 治理卷宗面的**单一真源资产**抽取器（**只读生产前端**）。
 *
 * ## 抽什么
 *
 * | 资产 | 来源 | 去向 |
 * |---|---|---|
 * | 组件 scoped CSS | 生产组件的 `<style>` 块（经 **Svelte 编译器**产出） | `public/<css>`（引擎页注入） |
 * | 图标几何数据 | 上游图标包的 `dist/icons/*.svelte` 里的 `iconNode` | `crates-ui/apeireth-ui-kit/src/icons_governance.rs`（生成物） |
 * | 设计令牌 CSS | `frontend/companion-desktop/src/lib/design/*.css` | `public/design/*.css`（**只读复制**，两侧同源） |
 *
 * ## 为什么 CSS 走「编译器产出」而不是「手抄 `<style>` 块」
 *
 * 生产组件的 `<style>` 块里的选择器是**组件私有**的（Svelte 编译器会给每个
 * 组件生成一个作用域 class，把选择器改写成「元素 class + 作用域 class」）。
 * 引擎页把样式当**普通样式表**注入 ⇒ 没有作用域 class ⇒ 同名 class 会互相
 * 覆盖（批一实测：`.tool-body` 的 `gap` 一个 8px 一个 10px ⇒ 整卡位移 2px）。
 *
 * ⇒ 本抽取器**调用生产自己的编译器**（`svelte/compiler`，只读打开生产源码）
 * 拿到**与参考侧逐字同源**的 scoped CSS，再把「作用域 class」替换成
 * 「作用域锚」（见下）⇒ 引擎页与参考侧的**匹配集合与特异度都一致**。
 *
 * ## 作用域锚：`.svelte-xxx` ⇒ `:is(<根 class>, <根 class> *)`
 *
 * Svelte 的作用域 class 落在**组件模板的每个元素**上 ⇒ 它的语义是
 * 「本组件的元素」。引擎侧没有这个 class ⇒ 用**根 class 的后代闭包**等价
 * 替换：
 *
 * - `:is(.gov-tab, .gov-tab *)` = 根元素本身 + 它的所有后代；
 * - `:is()` 的特异度取参数中最高者（一个 class）⇒ **与原 `.svelte-xxx`
 *   完全相同** ⇒ 层叠结果一致（设计令牌层里确有 `.amber` / `.on` / `.small`
 *   等同名 class，实测会互相盖 ⇒ 这一步不是可选优化，是正确性要求）。
 *
 * ⚠️ **边界**：作用域锚是「组件元素集合」的**超集**（子组件的元素也在根的
 * 后代里）⇒ 只有当**不同组件之间没有同名 class**时才等价。本面已实测：
 * 四个 Tab 与六个共享组件之间**无同名 class**（唯一的交集是 Tab 之间的
 * `.gov-*`，而四个 Tab **各自一页**、CSS 不同时加载 ⇒ 无冲突）。
 *
 * ## 纪律
 *
 * - **不手抄**：CSS 与图标几何全部由本脚本从源头派生；
 * - **不改生产文件**：只读打开；
 * - **许可单一**：图标数据生成物带上游许可署名（ISC + Feather 的 MIT 部分）。
 *
 * ## 用法
 *
 * ```bash
 * node scripts/extract-governance-assets.mjs            # 写入
 * node scripts/extract-governance-assets.mjs --check    # 只校验（漂移 ⇒ 退出码 1）
 * ```
 */

import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const HERE = dirname(fileURLToPath(import.meta.url));
const HARNESS = resolve(HERE, "..");
const REPO = resolve(HARNESS, "../../../..");
const MANIFEST_FILE = join(HARNESS, "face.manifest.json");

/** 生产前端根（**只读**）。 */
const FRONTEND = join(REPO, "frontend/companion-desktop");
/** 上游图标包（**只读**）。 */
const ICON_DIR = join(FRONTEND, "node_modules/lucide-svelte/dist/icons");
/** 上游许可文件（**只读**）。 */
const ICON_LICENSE = join(FRONTEND, "node_modules/lucide-svelte/LICENSE");
/** 设计令牌层（**只读**）。 */
const DESIGN_DIR = join(FRONTEND, "src/lib/design");

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
 * 生成的 Rust 源码过一遍 `rustfmt`（生成物必须 rustfmt 干净 ⇒ 门禁恒过）。
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

/** 写 Rust 生成物（先过 rustfmt）。 */
function emitRust(path, content, label) {
  emit(path, formatRust(content, label), label);
}

// ── ① 生产组件的 scoped CSS（经 Svelte 编译器） ─────────────────────────────

/**
 * 载入**生产自己的** Svelte 编译器（只读）。
 *
 * ⚠️ 用 `createRequire` 从**生产前端的 node_modules** 解析 ⇒ 不引入第二个
 * Svelte 版本 ⇒ 编译口径与参考侧（vite 的 svelte 插件）**同源**。
 */
function loadSvelteCompiler() {
  const require = createRequire(join(FRONTEND, "package.json"));
  try {
    const compiler = require("svelte/compiler");
    if (typeof compiler.compile !== "function") {
      throw new Error("svelte/compiler 没有 compile 函数");
    }
    return compiler.compile;
  } catch (error) {
    throw new Error(
      `生产前端的 node_modules 里找不到 svelte 编译器（${error.message}）；` +
        "可只读复用主仓依赖目录（ln -sfn <repo>/frontend/companion-desktop/node_modules <frontend>/companion-desktop/node_modules）",
    );
  }
}

/**
 * 把 Svelte 的作用域 class 换成**作用域锚**。
 *
 * - `.svelte-xxx`（作为 class 出现）⇒ `:is(<根>, <根> *)`；
 * - `@keyframes svelte-xxx-<名>` ⇒ `@keyframes <slug>-<名>`（动画名全局唯一，
 *   否则四个 Tab 的同名关键帧会互相覆盖）。
 *
 * @param {string} css Svelte 编译器产出的 CSS。
 * @param {string} scopeClass 根 class（带点，如 `.gov-tab`）。
 * @param {string} slug 组件短名（关键帧名前缀）。
 */
function anchorScope(css, scopeClass, slug) {
  const anchor = `:is(${scopeClass}, ${scopeClass} *)`;
  // 作用域 class 形如 `svelte-1g4meyy`（7 位小写字母数字）。
  const scopeNames = [...new Set(css.match(/svelte-[0-9a-z]+/g) || [])];
  let out = css;
  for (const name of scopeNames) {
    // 关键帧名（`svelte-xxx-gov-rot`）先处理 ⇒ 避免被下面的 class 替换截断。
    out = out.split(`${name}-`).join(`${slug}-`);
    out = out.split(`.${name}`).join(anchor);
    out = out.split(name).join(`${slug}-scope`);
  }
  return out;
}

/**
 * 抽一个生产组件的 scoped CSS（编译器产出 ⇒ 与参考侧同源）。
 *
 * @param {{slug: string, path: string, css: string, scope: string}} component
 * @returns {{css: string, scopeNames: string[]}} 产物与用到的作用域 class。
 */
function extractComponentCss(compile, component) {
  const absolute = join(REPO, component.path);
  const source = readOrFail(absolute, `生产组件 ${component.path}`);
  const compiled = compile(source, { filename: component.path });
  const raw = compiled.css ? compiled.css.code : "";
  if (!raw.trim()) {
    throw new Error(`生产组件 ${component.path} 没有产出 CSS`);
  }
  const scopeNames = [...new Set(raw.match(/svelte-[0-9a-z]+/g) || [])];
  const css = anchorScope(raw, component.scope, component.slug);
  return { css, scopeNames };
}

/** 抽全部组件的 CSS。 */
function extractComponents(manifest) {
  const compile = loadSvelteCompiler();
  const outDir = resolve(HARNESS, manifest.out);
  const summary = [];
  for (const component of manifest.components) {
    const { css, scopeNames } = extractComponentCss(compile, component);
    emit(join(outDir, component.css), css, `${component.slug} scoped CSS`);
    summary.push({ component, css, scopeNames });
  }
  return summary;
}

// ── ② 设计令牌 CSS（只读复制） ───────────────────────────────────────────────
function copyDesignCss(manifest) {
  const outDir = resolve(HARNESS, manifest.out, "design");
  const files = readdirSync(DESIGN_DIR)
    .filter((name) => name.endsWith(".css"))
    .sort();
  for (const name of files) {
    const content = readOrFail(join(DESIGN_DIR, name), "设计令牌 CSS");
    emit(join(outDir, name), content, `设计令牌 ${name}`);
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
  if (target.endsWith(".js")) {
    return resolveIconFile(target.replace(/\.js$/, ""));
  }
  throw new Error(`图标别名 ${alias} 的再导出目标不存在：${target}`);
}

/** 解析图标文件 ⇒ `{ alias, name, nodes }`。 */
function parseIcon(alias) {
  const resolved = resolveIconFile(alias);
  const source = readOrFail(resolved.file, `图标 ${alias}`);
  const nodeMatch = source.match(/const iconNode = (\[[\s\S]*?\]);/);
  if (!nodeMatch) {
    throw new Error(`图标 ${resolved.file} 里找不到 iconNode`);
  }
  const nodes = JSON.parse(nodeMatch[1]);
  const nameMatch = source.match(/<Icon\s+name="([^"]+)"/);
  if (!nameMatch) {
    throw new Error(`图标 ${resolved.file} 里找不到 \`<Icon name="…">\``);
  }
  return { alias, file: resolved.file, name: nameMatch[1], nodes };
}

function extractIcons(manifest) {
  const license = readOrFail(ICON_LICENSE, "上游许可文件");
  const entries = manifest.icons.map((alias) => parseIcon(alias));
  const rust = renderIconRs(entries, license);
  emitRust(resolve(HARNESS, manifest.icons_out), rust, "图标数据（Rust 生成物）");
  return entries;
}

/** 生成 Rust 图标数据文件。 */
function renderIconRs(entries, license) {
  const constName = (name) =>
    `ICON_${name.replace(/[^a-z0-9]+/gi, "_").toUpperCase()}`;
  const lines = [];
  lines.push("//! 图标数据（**自动生成** —— 勿手改）。");
  lines.push("//!");
  lines.push("//! 生成器：`crates-ui/apeireth-ui-demo/harness/governance/scripts/extract-governance-assets.mjs`");
  lines.push("//! 来源：`frontend/companion-desktop/node_modules/lucide-svelte/dist/icons/*.svelte`");
  lines.push("//!");
  lines.push("//! 上游许可（ISC；其中 Feather 部分为 MIT）—— 原文如下：");
  lines.push("//!");
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

/** Rust 字符串字面量。 */
function rustString(value) {
  return JSON.stringify(String(value));
}

/** Rust 可选字符串（`None` / `Some(...)`）。 */
function rustOptString(value) {
  if (value === undefined || value === null) return "None";
  return `Some(String::from(${rustString(value)}))`;
}

/** Rust 可选整数。 */
function rustOptInt(value) {
  if (value === undefined || value === null) return "None";
  return `Some(${Math.trunc(value)})`;
}

/** Rust `bool`。 */
function rustBool(value) {
  return value === true ? "true" : "false";
}

/** Rust `usize`。 */
function rustUsize(value) {
  return String(Math.max(0, Math.trunc(value ?? 0)));
}

/** 能力清单 ⇒ Rust 表达式。 */
function rustManifest(manifest) {
  const groups = (manifest.capabilities || []).map((group) => {
    const caps = (group.capabilities || []).map((cap) => {
      const parts = [
        `id: String::from(${rustString(cap.id)})`,
        `supported: ${rustBool(cap.supported)}`,
        `available: ${cap.available === undefined || cap.available === null ? "None" : `Some(${rustBool(cap.available)})`}`,
        `reason: ${rustOptString(cap.reason)}`,
        `alias_of: ${rustOptString(cap.alias_of)}`,
      ];
      return `            Capability {\n                ${parts.join(",\n                ")},\n            }`;
    });
    return `        CapabilityGroup {\n            capabilities: vec![\n${caps.join(",\n")},\n            ],\n        }`;
  });
  return `CapabilityManifest {\n        capabilities: vec![\n${groups.join(",\n")},\n        ],\n    }`;
}

/** 生成 fixture 的 Rust 产物。 */
function renderFixtureRust(fixture) {
  const lines = [];
  lines.push("//! 固定数据（**自动生成** —— 勿手改）。");
  lines.push("//!");
  lines.push(
    "// 来源：`crates-ui/apeireth-ui-demo/harness/governance/fixtures/governance-cases.json`",
  );
  lines.push(
    "// 生成器：`crates-ui/apeireth-ui-demo/harness/governance/scripts/extract-governance-assets.mjs`",
  );
  lines.push("//!");
  lines.push("// ⚠️ 时刻一律以**相对基准时刻的偏移**（毫秒）保存 ⇒ 运行期由宿主换算成");
  lines.push("//    RFC 3339 文本（参考侧与引擎侧的基准时刻由对照 harness 统一注入）。");
  lines.push("");
  lines.push("use apeireth_ui_kit::governance_tab::{Capability, CapabilityGroup, CapabilityManifest};");
  lines.push("");
  lines.push("/// 会话（固定数据）。");
  lines.push("#[derive(Debug, Clone, PartialEq, Eq)]");
  lines.push("pub struct FixtureSession {");
  lines.push("    /// 会话 id。");
  lines.push("    pub id: String,");
  lines.push("    /// 会话标题（空 ⇒ `None`）。");
  lines.push("    pub title: Option<String>,");
  lines.push("}");
  lines.push("");
  lines.push("/// 待签文书（固定数据；时刻为相对基准的偏移）。");
  lines.push("#[derive(Debug, Clone, PartialEq, Eq)]");
  lines.push("pub struct FixtureApproval {");
  lines.push("    /// 文书 id。");
  lines.push("    pub approval_id: String,");
  lines.push("    /// 所属会话 id。");
  lines.push("    pub session: String,");
  lines.push("    /// 工具名。");
  lines.push("    pub tool_name: String,");
  lines.push("    /// 治理理由（空串 ⇒ 缺省文案）。");
  lines.push("    pub governance_reason: String,");
  lines.push("    /// 命令文本。");
  lines.push("    pub command_text: Option<String>,");
  lines.push("    /// 参数摘要。");
  lines.push("    pub arguments_summary: Option<String>,");
  lines.push("    /// 创建时刻偏移（毫秒；`0` ⇒ 无）。");
  lines.push("    pub created_offset_ms: i64,");
  lines.push("    /// 过期时刻偏移（毫秒；`0` ⇒ 无）。");
  lines.push("    pub expires_offset_ms: i64,");
  lines.push("}");
  lines.push("");
  lines.push("/// 授权（固定数据；时刻为相对基准的偏移）。");
  lines.push("#[derive(Debug, Clone, PartialEq, Eq)]");
  lines.push("pub struct FixtureGrant {");
  lines.push("    /// 授权 id。");
  lines.push("    pub id: String,");
  lines.push("    /// 授权名。");
  lines.push("    pub name: String,");
  lines.push("    /// 入账时刻偏移（毫秒；`0` ⇒ 未记录）。");
  lines.push("    pub activated_offset_ms: i64,");
  lines.push("}");
  lines.push("");
  lines.push("/// span（固定数据；时刻为相对基准的偏移）。");
  lines.push("#[derive(Debug, Clone, PartialEq, Eq)]");
  lines.push("pub struct FixtureSpan {");
  lines.push("    /// span id。");
  lines.push("    pub span_id: String,");
  lines.push("    /// 所属轨迹 id。");
  lines.push("    pub trace_id: String,");
  lines.push("    /// 父 span id。");
  lines.push("    pub parent_span_id: Option<String>,");
  lines.push("    /// kind。");
  lines.push("    pub kind: String,");
  lines.push("    /// actor。");
  lines.push("    pub actor: String,");
  lines.push("    /// status。");
  lines.push("    pub status: String,");
  lines.push("    /// 摘要。");
  lines.push("    pub summary: Option<String>,");
  lines.push("    /// 开始时刻偏移（毫秒）。");
  lines.push("    pub started_offset_ms: i64,");
  lines.push("    /// 结束时刻偏移（毫秒；`None` ⇒ 进行中）。");
  lines.push("    pub ended_offset_ms: Option<i64>,");
  lines.push("    /// 会话 id。");
  lines.push("    pub session_id: Option<String>,");
  lines.push("}");
  lines.push("");
  lines.push("/// 轨迹（固定数据；时刻为相对基准的偏移）。");
  lines.push("#[derive(Debug, Clone, PartialEq, Eq)]");
  lines.push("pub struct FixtureTrace {");
  lines.push("    /// 轨迹 id。");
  lines.push("    pub trace_id: String,");
  lines.push("    /// span 数。");
  lines.push("    pub span_count: i64,");
  lines.push("    /// 开始时刻偏移（毫秒）。");
  lines.push("    pub started_offset_ms: i64,");
  lines.push("    /// 根 span。");
  lines.push("    pub root: FixtureSpan,");
  lines.push("}");
  lines.push("");
  lines.push("/// 事件留痕（固定数据；时刻为相对基准的偏移）。");
  lines.push("#[derive(Debug, Clone, PartialEq, Eq)]");
  lines.push("pub struct FixtureAuditEvent {");
  lines.push("    /// 时刻偏移（毫秒）。");
  lines.push("    pub ts_offset_ms: i64,");
  lines.push("    /// 事件名。");
  lines.push("    pub event: String,");
  lines.push("    /// 服务名。");
  lines.push("    pub service: String,");
  lines.push("    /// 明细。");
  lines.push("    pub detail: Option<String>,");
  lines.push("}");
  lines.push("");
  lines.push("/// 守卫状态（固定数据）。");
  lines.push("#[derive(Debug, Clone, PartialEq, Eq)]");
  lines.push("pub struct FixtureGuardStatus {");
  lines.push("    /// 守卫是否启用。");
  lines.push("    pub enabled: bool,");
  lines.push("    /// 快速守卫是否装配。");
  lines.push("    pub fast_guard_active: bool,");
  lines.push("    /// 链式守卫是否装配。");
  lines.push("    pub chain_guard_active: bool,");
  lines.push("    /// 意图守卫是否装配。");
  lines.push("    pub intent_guard_active: bool,");
  lines.push("    /// 跨轮监测是否装配。");
  lines.push("    pub cross_turn_monitoring_active: bool,");
  lines.push("    /// 评估总数。");
  lines.push("    pub total_evaluations: i64,");
  lines.push("    /// 放行数。");
  lines.push("    pub total_allowed: i64,");
  lines.push("    /// 否决数。");
  lines.push("    pub total_denied: i64,");
  lines.push("    /// 转人工数。");
  lines.push("    pub total_approval_required: i64,");
  lines.push("    /// ML 分类器是否可用。");
  lines.push("    pub ml_classifier_available: bool,");
  lines.push("    /// ML 模型版本。");
  lines.push("    pub ml_model_version: Option<String>,");
  lines.push("}");
  lines.push("");
  lines.push("/// 守卫事件（固定数据；时刻为相对基准的偏移）。");
  lines.push("#[derive(Debug, Clone, PartialEq)]");
  lines.push("pub struct FixtureGuardEvent {");
  lines.push("    /// 时刻偏移（毫秒）。");
  lines.push("    pub timestamp_offset_ms: i64,");
  lines.push("    /// 会话 id。");
  lines.push("    pub session_id: String,");
  lines.push("    /// 轨迹 id。");
  lines.push("    pub trace_id: String,");
  lines.push("    /// 轮次。");
  lines.push("    pub round: i64,");
  lines.push("    /// 能力 id。");
  lines.push("    pub capability_id: String,");
  lines.push("    /// 守卫阶段。");
  lines.push("    pub stage: String,");
  lines.push("    /// 决策。");
  lines.push("    pub decision: String,");
  lines.push("    /// 风险分。");
  lines.push("    pub risk_score: f64,");
  lines.push("    /// 理由。");
  lines.push("    pub reasons: Vec<String>,");
  lines.push("    /// 证据。");
  lines.push("    pub evidence: Vec<String>,");
  lines.push("}");
  lines.push("");
  lines.push("/// 对照用例（固定数据）。");
  lines.push("#[derive(Debug, Clone, PartialEq, Eq)]");
  lines.push("pub struct FixtureCase {");
  lines.push("    /// 用例 id。");
  lines.push("    pub id: String,");
  lines.push("    /// 能力清单键（`None` ⇒ 清单未到）。");
  lines.push("    pub manifest: Option<String>,");
  lines.push("    /// 数据行为（`default` / `empty` / `error` / `loading`）。");
  lines.push("    pub data: String,");
  lines.push("    /// 动作序列（DOM 点击）。");
  lines.push("    pub actions: Vec<FixtureAction>,");
  lines.push("}");
  lines.push("");
  lines.push("/// 用例动作（**DOM 点击** ⇒ 两侧同名同义）。");
  lines.push("#[derive(Debug, Clone, PartialEq, Eq)]");
  lines.push("pub struct FixtureAction {");
  lines.push("    /// 选择器。");
  lines.push("    pub click: String,");
  lines.push("    /// 第几个匹配项。");
  lines.push("    pub index: usize,");
  lines.push("}");
  lines.push("");

  // ── 能力清单 ────────────────────────────────────────────────────────────
  lines.push("/// 全部能力清单（键 ⇒ 清单）。");
  lines.push("pub fn manifests() -> Vec<(&'static str, CapabilityManifest)> {");
  lines.push("    vec![");
  for (const [key, manifest] of Object.entries(fixture.manifests)) {
    lines.push(`        (${rustString(key)}, ${rustManifest(manifest)}),`);
  }
  lines.push("    ]");
  lines.push("}");
  lines.push("");

  // ── 会话与 inbox ────────────────────────────────────────────────────────
  lines.push("/// 会话清单。");
  lines.push("pub fn sessions() -> Vec<FixtureSession> {");
  lines.push("    vec![");
  for (const session of fixture.sessions) {
    lines.push("        FixtureSession {");
    lines.push(`            id: String::from(${rustString(session.id)}),`);
    lines.push(`            title: ${rustOptString(session.title || null)},`);
    lines.push("        },");
  }
  lines.push("    ]");
  lines.push("}");
  lines.push("");
  lines.push("/// 某会话的待签 inbox。");
  lines.push("pub fn inbox(session_id: &str) -> Vec<FixtureApproval> {");
  lines.push("    let items: &[FixtureApproval] = match session_id {");
  for (const [sessionId, items] of Object.entries(fixture.inbox)) {
    lines.push(`        ${rustString(sessionId)} => &[`);
    for (const item of items) {
      lines.push("            FixtureApproval {");
      lines.push(`                approval_id: String::from(${rustString(item.approval_id)}),`);
      lines.push(`                session: String::from(${rustString(item.session)}),`);
      lines.push(`                tool_name: String::from(${rustString(item.tool_name)}),`);
      lines.push(`                governance_reason: String::from(${rustString(item.governance_reason ?? "")}),`);
      lines.push(`                command_text: ${rustOptString(item.command_text || null)},`);
      lines.push(`                arguments_summary: ${rustOptString(item.arguments_summary || null)},`);
      lines.push(`                created_offset_ms: ${Math.trunc(item.created_offset_ms ?? 0)},`);
      lines.push(`                expires_offset_ms: ${Math.trunc(item.expires_offset_ms ?? 0)},`);
      lines.push("            },");
    }
    lines.push("        ],");
  }
  lines.push("        _ => &[],");
  lines.push("    };");
  lines.push("    items.to_vec()");
  lines.push("}");
  lines.push("");

  // ── 授权 ───────────────────────────────────────────────────────────────
  lines.push("/// 授权清单。");
  lines.push("pub fn grants() -> Vec<FixtureGrant> {");
  lines.push("    vec![");
  for (const grant of fixture.grants) {
    lines.push("        FixtureGrant {");
    lines.push(`            id: String::from(${rustString(grant.id)}),`);
    lines.push(`            name: String::from(${rustString(grant.name)}),`);
    lines.push(`            activated_offset_ms: ${Math.trunc(grant.activated_offset_ms ?? 0)},`);
    lines.push("        },");
  }
  lines.push("    ]");
  lines.push("}");
  lines.push("");

  // ── 轨迹与 span ─────────────────────────────────────────────────────────
  const rustSpan = (span) => {
    const parts = [
      `span_id: String::from(${rustString(span.span_id)})`,
      `trace_id: String::from(${rustString(span.trace_id)})`,
      `parent_span_id: ${rustOptString(span.parent_span_id)}`,
      `kind: String::from(${rustString(span.kind)})`,
      `actor: String::from(${rustString(span.actor)})`,
      `status: String::from(${rustString(span.status)})`,
      `summary: ${rustOptString(span.summary || null)}`,
      `started_offset_ms: ${Math.trunc(span.started_offset_ms ?? 0)}`,
      `ended_offset_ms: ${rustOptInt(span.ended_offset_ms)}`,
      `session_id: ${rustOptString(span.session_id)}`,
    ];
    return `FixtureSpan {\n                ${parts.join(",\n                ")},\n            }`;
  };
  lines.push("/// 轨迹清单。");
  lines.push("pub fn traces() -> Vec<FixtureTrace> {");
  lines.push("    vec![");
  for (const trace of fixture.traces) {
    lines.push("        FixtureTrace {");
    lines.push(`            trace_id: String::from(${rustString(trace.trace_id)}),`);
    lines.push(`            span_count: ${Math.trunc(trace.span_count ?? 0)},`);
    lines.push(`            started_offset_ms: ${Math.trunc(trace.started_offset_ms ?? 0)},`);
    lines.push(`            root: ${rustSpan(trace.root_span)},`);
    lines.push("        },");
  }
  lines.push("    ]");
  lines.push("}");
  lines.push("");
  lines.push("/// 某轨迹的 span 树。");
  lines.push("pub fn spans(trace_id: &str) -> Vec<FixtureSpan> {");
  lines.push("    let items: &[FixtureSpan] = match trace_id {");
  for (const [traceId, spans] of Object.entries(fixture.spans)) {
    lines.push(`        ${rustString(traceId)} => &[`);
    for (const span of spans) {
      lines.push(`            ${rustSpan(span)},`);
    }
    lines.push("        ],");
  }
  lines.push("        _ => &[],");
  lines.push("    };");
  lines.push("    items.to_vec()");
  lines.push("}");
  lines.push("");

  // ── 事件留痕 ───────────────────────────────────────────────────────────
  lines.push("/// 事件留痕。");
  lines.push("pub fn audit_events() -> Vec<FixtureAuditEvent> {");
  lines.push("    vec![");
  for (const event of fixture.audit_events) {
    lines.push("        FixtureAuditEvent {");
    lines.push(`            ts_offset_ms: ${Math.trunc(event.ts_offset_ms ?? 0)},`);
    lines.push(`            event: String::from(${rustString(event.event)}),`);
    lines.push(`            service: String::from(${rustString(event.service)}),`);
    lines.push(`            detail: ${rustOptString(event.detail || null)},`);
    lines.push("        },");
  }
  lines.push("    ]");
  lines.push("}");
  lines.push("");

  // ── 守卫 ───────────────────────────────────────────────────────────────
  const status = fixture.guard_status;
  lines.push("/// 守卫状态。");
  lines.push("pub fn guard_status() -> FixtureGuardStatus {");
  lines.push("    FixtureGuardStatus {");
  lines.push(`        enabled: ${rustBool(status.enabled)},`);
  lines.push(`        fast_guard_active: ${rustBool(status.fast_guard_active)},`);
  lines.push(`        chain_guard_active: ${rustBool(status.chain_guard_active)},`);
  lines.push(`        intent_guard_active: ${rustBool(status.intent_guard_active)},`);
  lines.push(`        cross_turn_monitoring_active: ${rustBool(status.cross_turn_monitoring_active)},`);
  lines.push(`        total_evaluations: ${Math.trunc(status.total_evaluations ?? 0)},`);
  lines.push(`        total_allowed: ${Math.trunc(status.total_allowed ?? 0)},`);
  lines.push(`        total_denied: ${Math.trunc(status.total_denied ?? 0)},`);
  lines.push(`        total_approval_required: ${Math.trunc(status.total_approval_required ?? 0)},`);
  lines.push(`        ml_classifier_available: ${rustBool(status.ml_classifier_available)},`);
  lines.push(`        ml_model_version: ${rustOptString(status.ml_model_version)},`);
  lines.push("    }");
  lines.push("}");
  lines.push("");
  lines.push("/// 守卫事件。");
  lines.push("pub fn guard_events() -> Vec<FixtureGuardEvent> {");
  lines.push("    vec![");
  for (const event of fixture.guard_events) {
    lines.push("        FixtureGuardEvent {");
    lines.push(`            timestamp_offset_ms: ${Math.trunc(event.timestamp_offset_ms ?? 0)},`);
    lines.push(`            session_id: String::from(${rustString(event.session_id)}),`);
    lines.push(`            trace_id: String::from(${rustString(event.trace_id)}),`);
    lines.push(`            round: ${Math.trunc(event.round ?? 0)},`);
    lines.push(`            capability_id: String::from(${rustString(event.capability_id)}),`);
    lines.push(`            stage: String::from(${rustString(event.stage)}),`);
    lines.push(`            decision: String::from(${rustString(event.decision)}),`);
    lines.push(`            risk_score: ${Number(event.risk_score)},`);
    lines.push(`            reasons: vec![${event.reasons.map((r) => `String::from(${rustString(r)})`).join(", ")}],`);
    lines.push(`            evidence: vec![${event.evidence.map((r) => `String::from(${rustString(r)})`).join(", ")}],`);
    lines.push("        },");
  }
  lines.push("    ]");
  lines.push("}");
  lines.push("");

  // ── 用例 ───────────────────────────────────────────────────────────────
  for (const [tab, cases] of Object.entries(fixture.cases)) {
    lines.push(`/// ${tab} 面的对照用例。`);
    lines.push(`pub fn cases_${tab}() -> Vec<FixtureCase> {`);
    lines.push("    vec![");
    for (const item of cases) {
      lines.push("        FixtureCase {");
      lines.push(`            id: String::from(${rustString(item.id)}),`);
      lines.push(`            manifest: ${rustOptString(item.manifest)},`);
      lines.push(`            data: String::from(${rustString(item.data)}),`);
      lines.push("            actions: vec![");
      for (const action of item.actions || []) {
        lines.push("                FixtureAction {");
        lines.push(`                    click: String::from(${rustString(action.click)}),`);
        lines.push(`                    index: ${rustUsize(action.index)},`);
        lines.push("                },");
      }
      lines.push("            ],");
      lines.push("        },");
    }
    lines.push("    ]");
    lines.push("}");
    lines.push("");
  }

  lines.push("/// 错误文案（固定数据）。");
  lines.push("pub fn error_text() -> String {");
  lines.push(`    String::from(${rustString(fixture.error_text)})`);
  lines.push("}");
  lines.push("");

  return lines.join("\n");
}

/** 生成 fixture 的 Rust 产物（写入 `manifest.fixture.out`）。 */
function extractFixture(manifest) {
  const source = readOrFail(resolve(HARNESS, manifest.fixture.src), "固定数据");
  const fixture = JSON.parse(source);
  const rust = renderFixtureRust(fixture);
  emitRust(resolve(HARNESS, manifest.fixture.out), rust, "固定数据（Rust 生成物）");
  return fixture;
}

// ── 主流程 ────────────────────────────────────────────────────────────────────
function main() {
  const manifest = JSON.parse(readOrFail(MANIFEST_FILE, "抽取清单"));
  console.log(`仓库根：${REPO}`);
  console.log(`生产前端（只读）：${FRONTEND}`);

  const components = extractComponents(manifest);
  const designFiles = copyDesignCss(manifest);
  const icons = extractIcons(manifest);
  const fixture = extractFixture(manifest);

  const provenance = {
    note: "治理卷宗面对照资产的来源指纹（单一真源的可复核证据）",
    components: components.map(({ component, css, scopeNames }) => ({
      path: component.path,
      slug: component.slug,
      scope: component.scope,
      scope_classes: scopeNames,
      scoped_css_sha256_16: digest(css),
    })),
    design_css: designFiles.map((name) => ({
      path: `frontend/companion-desktop/src/lib/design/${name}`,
      sha256_16: digest(readOrFail(join(DESIGN_DIR, name), "设计令牌 CSS")),
    })),
    fixture: {
      path: "crates-ui/apeireth-ui-demo/harness/governance/fixtures/governance-cases.json",
      sha256_16: digest(
        readOrFail(resolve(HARNESS, manifest.fixture.src), "固定数据"),
      ),
      cases: Object.values(fixture.cases).reduce(
        (total, list) => total + list.length,
        0,
      ),
    },
    icons: icons.map((entry) => ({
      alias: entry.alias,
      path: `frontend/companion-desktop/node_modules/lucide-svelte/dist/icons/${entry.file.split("/").pop()}`,
      name: entry.name,
      nodes: entry.nodes.length,
      sha256_16: digest(readOrFail(entry.file, "图标")),
    })),
  };
  emit(
    resolve(HARNESS, manifest.out, "provenance.json"),
    `${JSON.stringify(provenance, null, 2)}\n`,
    "来源指纹清单",
  );

  console.log("抽取完成。");
}

try {
  main();
} catch (error) {
  console.error(`抽取失败：${error.message}`);
  process.exit(1);
}
