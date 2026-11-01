#!/usr/bin/env node
/**
 * 生成设置页对照的**两侧入口页**（每面一页）。
 *
 * ## 为什么用生成器
 *
 * 子面注册表（`web/shared/panels.js`）是**两侧单一真源** ⇒ 入口页若手写，
 * 就会出现「文档说的面」与「跑的面」不一致 ⇒ **入口页由注册表生成**。
 *
 * ## 纪律
 *
 * - **不改生产任何文件**（只读生产组件）；
 * - 生成物**逐字确定**（同一份注册表 ⇒ 同一批文件 ⇒ `--check` 模式可复核漂移）；
 * - 生成物落在**本目录独占**的 `web/{reference,engine}/<id>/index.html`。
 *
 * ## 用法
 *
 * ```bash
 * node scripts/gen-entry-pages.mjs          # 写入
 * node scripts/gen-entry-pages.mjs --check  # 只校验（漂移 ⇒ 退出码 1）
 * ```
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const SETTINGS = resolve(HERE, "..");
const WEB = join(SETTINGS, "web");

const checkOnly = process.argv.includes("--check");

/**
 * 子面注册表（与 `web/shared/panels.js` **同一份**）。
 *
 * ⚠️ 该文件是 ES module ⇒ 这里用**动态 import** 读它（不重复维护一份表）。
 */
async function loadPanels() {
  const module = await import(join(WEB, "shared", "panels.js"));
  return module.PANELS;
}

/** 读文件（不存在 ⇒ 明确报错）。 */
function readOrFail(path, what) {
  if (!existsSync(path)) {
    throw new Error(`${what} 不存在：${path}`);
  }
  return readFileSync(path, "utf8");
}

/** 写文件（`--check` 模式下只比对）。 */
function emit(path, content, label) {
  const current = existsSync(path) ? readFileSync(path, "utf8") : null;
  if (current === content) {
    console.log(`  ✓ ${label} 未漂移`);
    return;
  }
  if (checkOnly) {
    throw new Error(`${label} 已漂移：${path}（请重跑生成脚本）`);
  }
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, content, "utf8");
  console.log(`  → ${label} 已写入 ${path}`);
}

/** 缩进（HTML 注释块用）。 */
function indent(text, spaces) {
  const pad = " ".repeat(spaces);
  return text
    .split("\n")
    .map((line) => (line.trim() ? pad + line : line))
    .join("\n");
}

/** 参考侧入口页。 */
function renderReference(panel) {
  const source = panel.source.replace(/^frontend\//, "../../../../../frontend/");
  return `<!doctype html>
<html lang="zh-CN" data-theme="night">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>参考侧 · ${panel.title}</title>
    <!--
      设计令牌层：**只读引用**生产前端的同一批 CSS（构建期复制到 public/design/）。
      链接顺序与引擎侧**完全一致** ⇒ 两侧的计算样式同源。
    -->
    <link rel="stylesheet" href="/design/tokens.css" />
    <link rel="stylesheet" href="/design/base.css" />
    <link rel="stylesheet" href="/design/essence.css" />
    <link rel="stylesheet" href="/design/hud.css" />
    <link rel="stylesheet" href="/design/motion.css" />
    <link rel="stylesheet" href="/design/prose.css" />
    <link rel="stylesheet" href="/design/shell.css" />
    <!--
      参考侧**不**链入抽取出来的子面 CSS：生产组件自带**作用域化**的
      \`<style>\`（构建期由组件编译器注入）⇒ 参考侧的画面就是生产画面。
    -->
    <!-- 对照专用样式（两侧**逐字相同**地链入）：舞台宽度。 -->
    <link rel="stylesheet" href="/settings-test.css" />
  </head>
  <body>
    <!--
      本面：**${panel.title}**（\`${panel.id}\`）

      - 生产源文件（**只读引入**，不复制、不改）：\`${source}\`
      - 生产行号区间：${panel.ranges}
      - CSS 根 class（引擎侧的作用域前缀）：\`${panel.scope}\`
      - 裁剪选择器（参考侧 / 引擎侧）：${panel.sections.map((sec) => `\`${sec.ref}\` / \`${sec.engine}\``).join("；")}
      - 裁剪方式：${panel.crop === "viewport" ? "整视口" : "元素框"}
      - 固定数据：\`fixtures/${panel.fixture}\`
      - 依赖：${panel.deps.length ? panel.deps.join("、") : "无"}
      - 图标（生产 import 名）：${panel.icons.join("、")}
      - 可信度：**${panel.trust.level}** —— ${panel.trust.why}
    -->
    <div id="app"></div>
    <script type="module" src="../main.js"></script>
  </body>
</html>
`;
}

/** 引擎侧入口页。 */
function renderEngine(panel) {
  return `<!doctype html>
<html lang="zh-CN" data-theme="night">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>引擎侧 · ${panel.title}</title>
    <!--
      设计令牌层：**只读引用**生产前端的同一批 CSS（构建期复制到 public/design/）。
      链接顺序与参考侧**完全一致** ⇒ 两侧的计算样式同源。
    -->
    <link rel="stylesheet" href="/design/tokens.css" />
    <link rel="stylesheet" href="/design/base.css" />
    <link rel="stylesheet" href="/design/essence.css" />
    <link rel="stylesheet" href="/design/hud.css" />
    <link rel="stylesheet" href="/design/motion.css" />
    <link rel="stylesheet" href="/design/prose.css" />
    <link rel="stylesheet" href="/design/shell.css" />
    <!--
      子面 scoped CSS：构建期从生产的 \`<style>\` 块抽取，并按本面**根 class**
      作用域化（做法与组件编译器同构，规则写在抽取脚本里）。
      根 class：\`${panel.scope}\`
    -->
    <link rel="stylesheet" href="/settings-${panel.id}.css" />
    <!-- 对照专用样式（两侧**逐字相同**地链入）：舞台宽度。 -->
    <link rel="stylesheet" href="/settings-test.css" />
  </head>
  <body>
    <!--
      本面：**${panel.title}**（\`${panel.id}\`）

      - 引擎侧形态：wasm 引擎（\`apeireth-ui-demo\`）渲染**本面独立挂载**
      - \`window.__APP_MODE__\` = \`${panel.engine.mode}\`
      - 建议的 Rust 模块：\`${panel.engine.module}\`
      - CSS 根 class：\`${panel.scope}\`
      - 裁剪选择器（参考侧 / 引擎侧）：${panel.sections.map((sec) => `\`${sec.ref}\` / \`${sec.engine}\``).join("；")}
      - 裁剪方式：${panel.crop === "viewport" ? "整视口" : "元素框"}
      - 固定数据：\`fixtures/${panel.fixture}\`（与参考侧**同一份**）
      - 依赖：${panel.deps.length ? panel.deps.join("、") : "无"}
    -->
    <div id="app"></div>
    <script type="module" src="../main.js"></script>
  </body>
</html>
`;
}

async function main() {
  const panels = await loadPanels();
  for (const panel of panels) {
    emit(
      join(WEB, "reference", panel.id, "index.html"),
      renderReference(panel),
      `参考侧入口页 ${panel.id}`,
    );
    emit(
      join(WEB, "engine", panel.id, "index.html"),
      renderEngine(panel),
      `引擎侧入口页 ${panel.id}`,
    );
  }
  console.log("入口页生成完成。");
}

try {
  await main();
} catch (error) {
  console.error(`生成失败：${error.message}`);
  process.exit(1);
}
