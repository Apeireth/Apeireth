#!/usr/bin/env node
/**
 * 抽取设置页对照所需的**单一真源资产**。
 *
 * ## 抽什么
 *
 * | 资产 | 来源（**只读**） | 去向 |
 * |---|---|---|
 * | 子面 scoped CSS | 生产组件 / 视图的 `<style>` 块（**原文**） | `public/settings-<面 id>.css` |
 * | 共享设计层 CSS | `frontend/companion-desktop/src/lib/design/*.css` | `public/design/*.css`（**只读复制**，两侧同源） |
 *  *
 * ## 「按子面分片」的规则（**确定性，可复核**）
 *
 * 生产 `SettingsView.svelte` 的 `<style>` 块是**一份**（1,539 行）⇒ 6 个子面
 * 共用 ⇒ 若整份注入，6 个子面的同名规则（`.cap-row` / `.form-group` / …）
 * 会**互相覆盖**（批一已实测：`.tool-body` 的 `gap` 一个 8px、一个 10px ⇒
 * 2px 整卡位移）⇒ **必须分片 + 各自前缀根 class**。
 *
 * ### 分片算法
 *
 * 1. 对每个子面，收集它**自己**的 markup 区间 + **共享** markup 区间
 *    （`{#snippet fieldFlag}`）里出现过的**全部 class 名**（含 `class:foo`
 *    指令）⇒ 得「该面的 class 线索集」；
 * 2. 对 `<style>` 块里的**每一条规则**，取出它的**全部 class 选择器**；
 * 3. 规则归给子面 P ⇔ **P 的线索集包含该规则的全部 class 选择器**
 *    （⇒ 少一个就不归 ⇒ 宁可漏判也不误判？**不**：漏判会让规则消失 ⇒
 *    判据取「全部包含」正是为了**不误挂**；误挂（多一份）只是重复，**无害**）；
 * 4. `:global(…)` 规则**保持全局** ⇒ 每个子面文件都带一份（文本相同 ⇒ 无冲突）；
 * 5. **自检**：生产 `<style>` 的每一条规则**至少**归给一个子面 ⇒ 否则
 *    **显式报错**（⇒ 规则不会静默丢失）。
 *
 * ### 作用域化（与组件编译器同构）
 *
 * 复用批一的 `scopeCss()`：前缀根 class、`:global(…)` 保持全局、`@` 规则原样。
 *
 * ## 纪律
 *
 * - **不手抄**：全部由本脚本从源头抽取；
 * - **不改生产文件**：只读打开；
 * - **不碰 HTTP / SSE 契约**：本脚本只处理样式与静态资源。
 *
 * ## 用法
 *
 * ```bash
 * node scripts/extract-settings-assets.mjs          # 写入
 * node scripts/extract-settings-assets.mjs --check  # 只校验（漂移 ⇒ 退出码 1）
 * ```
 */

import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const SETTINGS = resolve(HERE, "..");
const REPO = resolve(SETTINGS, "../../../..");
const FRONTEND = join(REPO, "frontend/companion-desktop");
const DESIGN_DIR = join(FRONTEND, "src/lib/design");
const PUBLIC_DIR = join(SETTINGS, "public");
const DESIGN_OUT = join(PUBLIC_DIR, "design");

const checkOnly = process.argv.includes("--check");

/** 读文件（不存在 ⇒ 明确报错，不静默）。 */
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
    throw new Error(`${label} 已漂移：${path}（请重跑抽取脚本）`);
  }
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, content, "utf8");
  console.log(`  → ${label} 已写入 ${path}`);
}

// ── 作用域化（与批一 `harness/scripts/extract-assets.mjs` 同构） ────────────────

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

/**
 * 改写一条规则的选择器（保留两侧空白）。
 *
 * ## 三种情形（与组件编译器的行为同构）
 *
 * | 情形 | 处理 | 依据 |
 * |---|---|---|
 * | **整条都是 `:global(…)`** | 去掉括号，**保持全局**（不加根 class） | 实测：生产 `SettingsView` 编译出 `.spin { }` / `.success-icon { }`（**无**作用域 class） |
 * | **混合**（一部分 `:global(…)`、一部分不是） | **前缀根 class** + 去掉 `:global` 括号 | 实测：生产 `ThemeSettingsPanel` 的 `.bg-actions :global(.quiet-btn)` 编译出 `.bg-actions.svelte-x .quiet-btn` ⇒ 作用域 class 落在**前一个** compound 上 |
 * | 普通 | 前缀根 class（根自身的规则原样） | 实测：`.cap-row-nested .cap-icon` ⇒ `.cap-row-nested.svelte-x .cap-icon:where(.svelte-x)` ⇒ 特异度 0,3,0 ≡ 本实现的前缀式 0,3,0 |
 */
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
      const withoutGlobal = trimmed.replace(/:global\([^)]*\)/g, "");
      if (withoutGlobal.trim() === "") {
        // 整条都是 global ⇒ 保持全局。
        return part.replace(/:global\(([^)]*)\)/g, "$1");
      }
      // 混合 ⇒ 前缀根 class（作用域 class 落在前一个 compound 上）。
      const partLeading = part.match(/^\s*/)[0];
      const partTrailing = part.match(/\s*$/)[0];
      const core = part
        .slice(partLeading.length, part.length - partTrailing.length)
        .replace(/:global\(([^)]*)\)/g, "$1");
      return `${partLeading}${rootSelector} ${core}${partTrailing}`;
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

/** 把一段 CSS 按根 class 作用域化。 */
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

// ── 规则切分 ────────────────────────────────────────────────────────────────

/** 把一段 CSS 切成「选择器文本 + 规则体」的序列。 */
function splitRules(css) {
  const rules = [];
  let index = 0;
  while (index < css.length) {
    const open = css.indexOf("{", index);
    if (open < 0) {
      break;
    }
    const selectorText = css.slice(index, open);
    const close = matchBrace(css, open);
    const body = css.slice(open, close + 1);
    rules.push({selectorText, body});
    index = close + 1;
  }
  return rules;
}

/** 取一条规则的 class 选择器（去重）。 */
function classSelectors(selectorText) {
  const classes = new Set();
  for (const part of selectorText.split(",")) {
    for (const match of part.matchAll(/\.([A-Za-z0-9_-]+)/g)) {
      classes.add(match[1]);
    }
  }
  return [...classes];
}

/** 规则是否是 `:global(…)` 规则（⇒ 保持全局）。 */
function isGlobalRule(selectorText) {
  return selectorText.includes(":global(");
}

/**
 * 规则是否**整条**都是 `:global(…)`（⇒ 保持全局）。
 *
 * 「混合」形态（`.bg-actions :global(.quiet-btn)`）**不是**整条 global ⇒
 * 作用域 class 落在前一个 compound 上 ⇒ 要前缀根 class。
 */
function isFullyGlobalRule(selectorText) {
  return selectorText
    .split(",")
    .every((part) => part.replace(/:global\([^)]*\)/g, "").trim() === "");
}

// ── markup 的 class 线索 ────────────────────────────────────────────────────

/** 收集一段 markup 里出现过的全部 class 名。 */
function collectClasses(markup) {
  const classes = new Set();
  for (const match of markup.matchAll(/class\s*=\s*"([^"]*)"/g)) {
    for (const token of match[1].split(/\s+/)) {
      if (token) {
        classes.add(token);
      }
    }
  }
  for (const match of markup.matchAll(/class:([A-Za-z0-9_-]+)/g)) {
    classes.add(match[1]);
  }
  return classes;
}

/** 读一个源文件的行数组。 */
function readLines(path) {
  return readOrFail(path, "生产源文件").split("\n");
}

/** 读一个源文件的 `<style>` 块（原文）。 */
function readStyleBlock(path, what) {
  const source = readOrFail(path, what);
  const open = source.indexOf("<style>");
  const close = source.indexOf("</style>");
  if (open < 0 || close < 0 || close < open) {
    throw new Error(`${what} 里找不到 \`<style>\` 块`);
  }
  return source.slice(open + "<style>".length, close);
}

/** 取若干行区间拼成的文本。 */
function sliceRanges(lines, ranges) {
  const chunks = [];
  for (const [from, to] of ranges) {
    chunks.push(lines.slice(from - 1, to).join("\n"));
  }
  return chunks.join("\n");
}

// ── 主流程 ──────────────────────────────────────────────────────────────────

async function main() {
  console.log(`仓库根：${REPO}`);
  console.log(`生产前端（只读）：${FRONTEND}`);

  const { PANELS, SHARED_MARKUP_RANGES } = await import(join(SETTINGS, "web/shared/panels.js"));

  // 每个面一份 CSS 产物。
  //
  // ## class 线索从哪来
  //
  // ① 本面**自己**的 markup 区间（`markupRanges`）；
  // ② **额外**的 markup 区间（`extraMarkup`：定义在别的文件、但渲染进本面的
  //    片段 —— 例：主题面板外面裹着 `SettingsView` 的 `appearance` 分支）；
  // ③ **共享**区间（`SHARED_MARKUP_RANGES`：`{#snippet fieldFlag}`）。
  const perPanel = new Map();
  for (const panel of PANELS) {
    const chunks = [];
    chunks.push(sliceRanges(readLines(join(REPO, panel.source)), panel.markupRanges));
    for (const extra of panel.extraMarkup ?? []) {
      chunks.push(sliceRanges(readLines(join(REPO, extra.source)), extra.ranges));
    }
    if (panel.source === SHARED_MARKUP_RANGES.source) {
      chunks.push(
        sliceRanges(readLines(join(REPO, SHARED_MARKUP_RANGES.source)), SHARED_MARKUP_RANGES.ranges),
      );
    }
    perPanel.set(panel.id, {
      panel,
      classes: collectClasses(chunks.join("\n")),
    });
  }

  /**
   * 按**源文件**归组。
   *
   * ⚠️ **必须按源文件遍历一次**（而不是「每个面遍历一次」）——否则同一个源文件
   * （`SettingsView.svelte` 被 5 个面共用）的规则会被**重复归**多次 ⇒
   * 产物里出现**同一规则的多份拷贝**（本批实测过这个坑）。
   */
  const sources = new Map();
  for (const panel of PANELS) {
    for (const cssSource of panel.cssSources ?? [{source: panel.source}]) {
      const path = join(REPO, cssSource.source);
      if (!sources.has(path)) {
        sources.set(path, {
          path,
          css: readStyleBlock(path, `生产源文件 ${cssSource.source}`),
          panels: [],
        });
      }
      sources.get(path).panels.push(panel);
    }
  }

  // 分片：把每条规则归给「线索集包含它全部 class 选择器」的面。
  const assigned = new Map(PANELS.map((panel) => [panel.id, []]));
  const unassigned = [];

  for (const source of sources.values()) {
    for (const rule of splitRules(source.css)) {
      const classes = classSelectors(rule.selectorText);
      if (isGlobalRule(rule.selectorText)) {
        if (isFullyGlobalRule(rule.selectorText)) {
          // 整条 global ⇒ 保持全局 ⇒ 每个面都带一份（文本相同 ⇒ 无冲突）。
          for (const panel of PANELS) {
            assigned.get(panel.id).push(rule);
          }
          continue;
        }
        // 混合（部分 global）⇒ 作用域 class 落在**本规则所属源文件**对应的面
        // ⇒ 只归给那些面（跨文件归会引入无关前缀）。
        for (const panel of source.panels) {
          assigned.get(panel.id).push(rule);
        }
        continue;
      }
      if (classes.length === 0) {
        // 无 class 选择器（纯元素选择器）⇒ 每个面都带一份（文本相同 ⇒ 无冲突）。
        for (const panel of PANELS) {
          assigned.get(panel.id).push(rule);
        }
        continue;
      }
      const owners = PANELS.filter((panel) => {
        // ⚠️ **该面必须「拥有」这条规则的源文件**（并行面 M1 实测发现的缺陷）：
        // 只按 class 名判归属，会让**别的组件**里的同名 class 规则漏进来
        // （实测：`SettingsView.svelte` 的 `.preset-chip` 规则漏进向导分片
        //  ⇒ 两条规则特异度相同 ⇒ 后写的**琥珀色**赢 ⇒ 引擎侧芯片变琥珀、
        //  参考侧是**蓝色** ⇒ 像素差 4.89%）。
        // ⇒ 判据加一条「源文件必须是本面的源文件」。
        // ⚠️ 不影响「公共片段按使用面复制」的设计：`SettingsView` 系的几个面
        // **共享同一个源文件** ⇒ 它们在 `source.panels` 里都在 ⇒ 照旧各得一份。
        if (!source.panels.includes(panel)) {
          return false;
        }
        const set = perPanel.get(panel.id).classes;
        return classes.every((name) => set.has(name));
      });
      if (owners.length === 0) {
        unassigned.push(`${rule.selectorText.trim()}  →  class ${classes.join(".")}`);
        continue;
      }
      for (const panel of owners) {
        assigned.get(panel.id).push(rule);
      }
    }
  }

  if (unassigned.length > 0) {
    throw new Error(
      `有 ${unassigned.length} 条生产规则**没有归到任何子面**（⇒ 会静默丢失）：\n  ${unassigned.join("\n  ")}`,
    );
  }

  // 产出：每面一份作用域化 CSS。
  for (const panel of PANELS) {
    const rules = assigned.get(panel.id);
    const raw = rules.map((rule) => `${rule.selectorText}${rule.body}`).join("");
    const scoped = scopeCss(raw, panel.scope);
    emit(join(PUBLIC_DIR, `settings-${panel.id}.css`), scoped, `${panel.id} scoped CSS`);
  }

  // 共享设计层（只读复制 ⇒ 两侧同源）。
  const designFiles = readdirSync(DESIGN_DIR)
    .filter((name) => name.endsWith(".css"))
    .sort();
  for (const name of designFiles) {
    emit(join(DESIGN_OUT, name), readOrFail(join(DESIGN_DIR, name), "设计令牌 CSS"), `设计令牌 ${name}`);
  }

  // ⚠️ 对照专用样式（`public/settings-test.css`）是**手写并入库**的
  // （与批一的 `public/pilot-test.css` 同口径）⇒ **不由本脚本生成**。

  console.log("抽取完成。");
}

try {
  await main();
} catch (error) {
  console.error(`抽取失败：${error.message}`);
  process.exit(1);
}
