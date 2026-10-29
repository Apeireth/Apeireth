#!/usr/bin/env node
/**
 * 生成设置页对照用的**替身模块**（`mock/<模块名>.js`）。
 *
 * ## 为什么必须有替身
 *
 * 生产的设置组件在**挂载时**就会碰数据面（模型列表 / 工作区 / 桌面判�� /
 * 本地存储 / IndexedDB）⇒ 参考侧要喂**固定数据**，就必须把这些模块替换掉。
 *
 * ## 纪律
 *
 * - **不手抄导出面**：本脚本读生产的导出面 ⇒ 自动产出替身
 *   （生产导出面变了 ⇒ 重跑本脚本即可；`--check` 模式可复核漂移）；
 * - **不修改生产任何文件**（只读打开）；
 * - 已接线的导出（`mock/real/<模块名>.js`）喂**固定 fixture**；
 *   其余导出给**显式报错的桩**：调用即抛错 / 读取即抛错
 *   ⇒ **不会静默给出假数据**；
 * - **不碰数据面**：替身只喂固定数据 ⇒ 引擎不碰 HTTP / SSE 契约
 *   （设计文档 §1.3 第 6 条）与宿主能力（第 7 条）。
 *
 * ## 为什么「导出面」必须**齐**
 *
 * ES 模块的具名导入在**链接期**就要能解析 ⇒ 生产里任何一个被导入的名字在
 * 替身里缺席 ⇒ 整个模块图加载失败 ⇒ 参考侧**整页白屏** ⇒ 对照无从谈起。
 *
 * ⇒ 本脚本保证「生产导出面 ⊆ 替身导出面」。
 *
 * ## 用法
 *
 * ```bash
 * node scripts/gen-settings-mock.mjs          # 写入
 * node scripts/gen-settings-mock.mjs --check  # 只校验（漂移 ⇒ 退出码 1）
 * ```
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const SETTINGS = resolve(HERE, "..");
const REPO = resolve(SETTINGS, "../../../..");
const FRONTEND = join(REPO, "frontend/companion-desktop");
const LIB = join(FRONTEND, "src/lib");
const MOCK_DIR = join(SETTINGS, "mock");
const REAL_DIR = join(MOCK_DIR, "real");

const checkOnly = process.argv.includes("--check");

/**
 * 被替身的生产模块（**basename**，不含扩展名）。
 *
 * ⚠️ 这张表与 `mock/settings-mock-plugin.mjs` 的 `MOCKED_MODULES` **必须一致**
 *   ⇒ 抽取器/生成器会互相校验（见 `main()` 的自检）。
 */
const MODULES = [
  "runtime",
  "tauri-bridge",
  "desktop-bridge",
  "user-profile",
  "user-impression",
  "bg-store",
];

/** 读文件（不存在 ⇒ 明确报错，不静默）。 */
function readOrFail(path, what) {
  if (!existsSync(path)) {
    throw new Error(`${what} 不存在：${path}`);
  }
  return readFileSync(path, "utf8");
}

/**
 * 解析生产模块的导出面。
 *
 * @param {string} source 模块源码。
 * @returns {{kinds: Map<string,string>, statements: number}}
 *   `kinds` = 导出名 ⇒ 种类（去重后）。
 */
function readExportSurface(source) {
  const kinds = new Map();
  let statements = 0;
  const exportRe =
    /^export\s+(async\s+function|function|const|class|interface|type|let|var)\s+([A-Za-z0-9_]+)/gm;
  exportRe.lastIndex = 0;
  let match = exportRe.exec(source);
  while (match !== null) {
    statements += 1;
    kinds.set(match[2], match[1].replace(/\s+/g, " "));
    match = exportRe.exec(source);
  }
  // `export { a, b } from './x'` 与 `export { a, b }` 两种形式。
  const listRe = /^export\s*\{([^}]*)\}/gm;
  listRe.lastIndex = 0;
  match = listRe.exec(source);
  while (match !== null) {
    for (const part of match[1].split(",")) {
      const name = part.trim().split(/\s+as\s+/).pop();
      if (name && /^[A-Za-z0-9_]+$/.test(name) && !kinds.has(name)) {
        kinds.set(name, "reexport");
      }
    }
    match = listRe.exec(source);
  }
  return {kinds, statements};
}

/** 桩的报错文案（统一 ⇒ 一眼看出是替身而不是生产）。 */
function stubMessage(moduleName, name) {
  return `${moduleName} 替身：导出「${name}」未接线（对照 harness 只喂固定 fixture）`;
}

/** 生成一个导出名 ⇒ 桩。 */
function renderStub(moduleName, name, kind) {
  const text = JSON.stringify(stubMessage(moduleName, name));
  if (kind === "async function") {
    return [`export async function ${name}() {`, `  throw new Error(${text});`, `}`];
  }
  if (kind === "function") {
    return [`export function ${name}() {`, `  throw new Error(${text});`, `}`];
  }
  if (kind === "class") {
    return [
      `export class ${name} extends Error {`,
      `  constructor() {`,
      `    super(${text});`,
      `    this.name = ${JSON.stringify(name)};`,
      `  }`,
      `}`,
    ];
  }
  // ⚠️ **必须返回数组**（并行面 M5 实测发现，2026-10-24）：本函数原本**前三条分支返回数组、
  // 末条返回字符串** ⇒ 调用方**无法用同一种 push 方式**接住两者：
  // · `push(数组)` ⇒ 数组被当**一个元素** ⇒ join 出 `export function x() {, …,}` ⇒ 语法错误
  // · `push(...字符串)` ⇒ 字符串被**逐字符展开** ⇒ 每个字符一行 ⇒ 同样是语法错误
  // ⇒ 统一：**永远返回行数组**。
  return [`export const ${name} = notWiredValue(${JSON.stringify(name)}, ${text});`];
}

/** 渲染替身模块。 */
function renderMock(moduleName, surface, wired) {
  const names = [...surface.kinds.keys()];
  const typeNames = names.filter((name) => surface.kinds.get(name) === "interface" || surface.kinds.get(name) === "type");
  const valueNames = names.filter((name) => !typeNames.includes(name));
  const stubs = valueNames.filter((name) => !wired.includes(name));

  const lines = [];
  lines.push(`// \`${moduleName}\` 的**对照用替身**（**自动生成** —— 勿手改）。`);
  lines.push("//");
  lines.push("// 生成器：`crates-ui/apeireth-ui-demo/harness/settings/scripts/gen-settings-mock.mjs`");
  lines.push(`// 来源：\`frontend/companion-desktop/src/lib/${moduleName}.ts\`（**只读**）`);
  lines.push("//");
  lines.push("// ## 出口清单（由生产导出面**自动派生** ⇒ 不手抄）");
  lines.push("//");
  lines.push(`// - 生产导出语句：**${surface.statements}** 条（去重后 **${names.length}** 个导出名）`);
  lines.push(`// - 运行期导出（值）：**${valueNames.length}** 个 ⇒ 其中 **${wired.length}** 个已接线，**${stubs.length}** 个是显式报错桩；`);
  lines.push(`// - 编译期擦除（type / interface）：**${typeNames.length}** 个。`);
  lines.push("//");
  lines.push("// ## 纪律");
  lines.push("//");
  lines.push("// - 已接线的导出喂**固定 fixture**（两侧同源）⇒ 不碰 HTTP / SSE 契约；");
  lines.push("// - 其余导出**调用即抛错**（const 是**读取即抛错**）⇒ 不静默给假数据。");
  lines.push("");
  lines.push("/**");
  lines.push(" * 未接线的常量值（**读取即抛错** ⇒ 不静默给假数据）。");
  lines.push(" *");
  lines.push(" * @param {string} name 导出名（报错文案里点名）。");
  lines.push(" * @param {string} text 报错文案。");
  lines.push(" * @returns {unknown} 读取即抛错的替身。");
  lines.push(" */");
  lines.push("function notWiredValue(name, text) {");
  lines.push("  const target = function stub() {};");
  lines.push("  return new Proxy(target, {");
  lines.push("    get(_target, prop) {");
  lines.push("      if (typeof prop === 'symbol' || prop === 'then' || prop === 'toJSON') {");
  lines.push("        return undefined;");
  lines.push("      }");
  lines.push("      throw new Error(text);");
  lines.push("    },");
  lines.push("    apply() {");
  lines.push("      throw new Error(text);");
  lines.push("    },");
  lines.push("    construct() {");
  lines.push("      throw new Error(text);");
  lines.push("    },");
  lines.push("  });");
  lines.push("}");
  lines.push("");
  lines.push("// ── 已接线（真实现；喂固定 fixture） ─────────────────────────────────────");
  lines.push(`export {\n${wired.map((name) => `  ${name},`).join("\n")}\n} from "./real/${moduleName}.js";`);
  lines.push("");
  lines.push("// ── 未接线（显式报错桩） ───────────────────────────────────────────────");
  for (const name of stubs) {
    // ⚠️ `renderStub` 返回**行数组** ⇒ 必须**展开** push（并行面 M5 实测发现，2026-10-24）：
    // 直接 `push(数组)` 会把数组当**一个元素** ⇒ `join("\n")` 产出
    // `export function x() {,  throw new Error(…);,}` ⇒ **语法错误**
    // ⇒ 替身**一旦被真正解析**，vite 构建直接失败 ⇒ 这条路径**从未跑通过**。
    lines.push(...renderStub(moduleName, name, surface.kinds.get(name)));
    lines.push("");
  }
  return `${lines.join("\n").replace(/\n+$/, "")}\n`;
}

/** 读「已接线」名单（`mock/real/<模块>.js` 里的导出）。 */
function readWired(moduleName) {
  const path = join(REAL_DIR, `${moduleName}.js`);
  const source = readOrFail(path, `替身真实现 ${moduleName}.js`);
  const names = new Set();
  const re = /^export\s+(?:async\s+)?(?:function|const|class|let|var)\s+([A-Za-z0-9_]+)/gm;
  re.lastIndex = 0;
  let match = re.exec(source);
  while (match !== null) {
    names.add(match[1]);
    match = re.exec(source);
  }
  const listRe = /^export\s*\{([^}]*)\}/gm;
  listRe.lastIndex = 0;
  match = listRe.exec(source);
  while (match !== null) {
    for (const part of match[1].split(",")) {
      const name = part.trim().split(/\s+as\s+/)[0];
      if (name && /^[A-Za-z0-9_]+$/.test(name)) {
        names.add(name);
      }
    }
    match = listRe.exec(source);
  }
  return [...names].sort();
}

function main() {
  console.log(`仓库根：${REPO}`);
  console.log(`生产前端（只读）：${FRONTEND}`);
  for (const moduleName of MODULES) {
    const surface = readExportSurface(readOrFail(join(LIB, `${moduleName}.ts`), `生产模块 ${moduleName}.ts`));
    const wired = readWired(moduleName);
    const content = renderMock(moduleName, surface, wired);
    const path = join(MOCK_DIR, `${moduleName}.js`);
    const current = existsSync(path) ? readFileSync(path, "utf8") : null;
    if (current === content) {
      console.log(`  ✓ ${moduleName} 替身未漂移`);
      continue;
    }
    if (checkOnly) {
      throw new Error(`${moduleName} 替身已漂移：${path}（请重跑 node scripts/gen-settings-mock.mjs）`);
    }
    mkdirSync(dirname(path), {recursive: true});
    writeFileSync(path, content, "utf8");
    console.log(`  → ${moduleName} 替身已写入 ${path}（已接线 ${wired.length} 个）`);
  }
  console.log("替身生成完成。");
}

try {
  main();
} catch (error) {
  console.error(`生成失败：${error.message}`);
  process.exit(1);
}
