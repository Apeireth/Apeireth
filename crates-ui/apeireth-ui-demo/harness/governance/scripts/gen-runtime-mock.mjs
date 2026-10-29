#!/usr/bin/env node
/**
 * 生成 `runtime` 模块的**对照用替身**（mock）。
 *
 * ## 为什么必须有 mock
 *
 * 生产的治理卷宗面四个 Tab 在 `onMount` 里拉数据（`fetchGrants` /
 * `fetchCanonicalApprovals` / `fetchAuditLogs` / `fetchGuardStatus` …）并做
 * **能力门控**（`capabilityAvailable` / `capabilityUnavailableReason`）⇒ 参考侧
 * 要在浏览器里喂**固定数据**，就必须把 `../../runtime` 这个模块替换掉。
 *
 * 替换由 vite 插件在 `resolveId` 阶段完成（按 **importer 路径**判断 ⇒ 只拦
 * 治理卷宗面四个 Tab 的导入）⇒ **不修改生产任何文件**。
 *
 * ## 纪律
 *
 * - **不手抄导出面**：本脚本读生产 `runtime.ts` 的导出面 ⇒ 自动产出 mock
 *   （生产导出面变了 ⇒ 重跑本脚本即可，`--check` 模式可复核漂移）；
 * - **不修改生产任何文件**（只读打开）；
 * - 用到的几个给**真实现**（喂固定 fixture，见 `runtime-mock-real.js`），
 *   其余给**显式报错的桩**：调用即抛错 / 读取即抛错 ⇒ **不会静默给出假数据**；
 * - **不碰数据面**：mock 只喂固定数据 ⇒ 引擎不碰 HTTP / SSE 契约
 *   （设计文档 §1.3 第 6 条）。
 *
 * ## 用法
 *
 * ```bash
 * node scripts/gen-runtime-mock.mjs          # 写入
 * node scripts/gen-runtime-mock.mjs --check  # 只校验（漂移 ⇒ 退出码 1）
 * ```
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const HARNESS = resolve(HERE, "..");
const REPO = resolve(HARNESS, "../../../..");

/** 生产前端根（**只读**）。 */
const FRONTEND = join(REPO, "frontend/companion-desktop");
/** 生产 `runtime` 模块（**只读**）。 */
const RUNTIME_TS = join(FRONTEND, "src/lib/runtime.ts");
/** 生成物（mock 模块本体）。 */
const MOCK_JS = join(HARNESS, "mock/runtime-mock.js");

const checkOnly = process.argv.includes("--check");

/**
 * **已接线**的导出（真实现，见 `runtime-mock-real.js`）。
 *
 * 判定口径：只有「治理卷宗面四个 Tab 真正调用到」的导出才给真实现；其余一律
 * 显式报错桩。
 */
const WIRED = [
  "capabilityAvailable",
  "capabilityUnavailableReason",
  "findCapability",
  "friendlyErrorMessage",
  "fetchBackendSessions",
  "fetchCanonicalApprovals",
  "fetchGrants",
  "fetchTraceList",
  "fetchTraceDetail",
  "fetchAuditLogs",
  "fetchGuardStatus",
  "fetchGuardEvents",
  "evaluateGuard",
];

/** 导出语句的匹配式（行首 `export` + 种类 + 名字）。 */
const EXPORT_RE =
  /^export\s+(async\s+function|function|const|class|interface|type|let|var)\s+([A-Za-z0-9_]+)/gm;

/** 编译期擦除的种类（type / interface ⇒ 运行期不存在 ⇒ 不必出现在 mock 里）。 */
const TYPE_KINDS = new Set(["interface", "type"]);

/** 读文件（不存在 ⇒ 明确报错，不静默）。 */
function readOrFail(path, what) {
  if (!existsSync(path)) {
    throw new Error(`${what} 不存在：${path}`);
  }
  return readFileSync(path, "utf8");
}

/**
 * 解析生产 `runtime.ts` 的导出面。
 *
 * @returns {{kinds: Map<string,string>, statements: number, duplicates: string[]}}
 */
function readExportSurface(source) {
  const kinds = new Map();
  const duplicates = [];
  let statements = 0;
  EXPORT_RE.lastIndex = 0;
  let match = EXPORT_RE.exec(source);
  while (match !== null) {
    statements += 1;
    const kind = match[1].replace(/\s+/g, " ");
    const name = match[2];
    if (kinds.has(name)) {
      duplicates.push(name);
    } else {
      kinds.set(name, kind);
    }
    match = EXPORT_RE.exec(source);
  }
  return { kinds, statements, duplicates };
}

/** 桩的报错文案（统一 ⇒ 一眼看出是 mock 而不是生产）。 */
function stubMessage(name) {
  return `runtime mock：导出「${name}」未接线（对照 harness 只喂固定 fixture）`;
}

/** 生成一个导出名 ⇒ 桩。 */
function renderStub(name, kind) {
  if (kind === "async function") {
    return [
      `export async function ${name}() {`,
      `  throw new Error(${JSON.stringify(stubMessage(name))});`,
      `}`,
    ].join("\n");
  }
  if (kind === "function") {
    return [
      `export function ${name}() {`,
      `  throw new Error(${JSON.stringify(stubMessage(name))});`,
      `}`,
    ].join("\n");
  }
  if (kind === "class") {
    return [
      `export class ${name} extends Error {`,
      `  constructor(message) {`,
      `    super(${JSON.stringify(stubMessage(name))});`,
      `    this.name = ${JSON.stringify(name)};`,
      `  }`,
      `}`,
    ].join("\n");
  }
  return `export const ${name} = notWiredValue(${JSON.stringify(name)});`;
}

/** 渲染整个 mock 文件。 */
function renderMock(surface) {
  const names = [...surface.kinds.keys()];
  const runtimeNames = names.filter((name) => !TYPE_KINDS.has(surface.kinds.get(name)));
  const typeNames = names.filter((name) => TYPE_KINDS.has(surface.kinds.get(name)));
  const missing = WIRED.filter((name) => !surface.kinds.has(name));
  if (missing.length > 0) {
    throw new Error(
      `接线名单里的导出在生产 runtime 里不存在：${missing.join(" / ")} ⇒ 请更新生成脚本`,
    );
  }
  const wiredSet = new Set(WIRED);

  const lines = [];
  lines.push("// `runtime` 模块的**对照用替身**（**自动生成** —— 勿手改）。");
  lines.push("//");
  lines.push(
    "// 生成器：`crates-ui/apeireth-ui-demo/harness/governance/scripts/gen-runtime-mock.mjs`",
  );
  lines.push("// 来源：`frontend/companion-desktop/src/lib/runtime.ts`（**只读**）");
  lines.push("//");
  lines.push("// ## 这个文件是什么");
  lines.push("//");
  lines.push("// 参考侧要**只读引入**生产的治理卷宗面四个 Tab ⇒ 它们在 `onMount` 里拉数据、");
  lines.push("// 做能力门控 ⇒ 必须把 `../../runtime` 换成喂固定数据的替身（由 vite 插件在");
  lines.push("// `resolveId` 阶段拦截 ⇒ **生产文件零改动**）。");
  lines.push("//");
  lines.push("// ## 出口清单（由生产导出面**自动派生** ⇒ 不手抄）");
  lines.push("//");
  lines.push(`// - 生产导出语句：**${surface.statements}** 条（去重后 **${names.length}** 个导出名）`);
  lines.push(
    `// - 运行期导出（值）：**${runtimeNames.length}** 个 ⇒ 其中 **${WIRED.length}** 个已接线（真实现），`,
  );
  lines.push(
    `//   **${runtimeNames.length - WIRED.length}** 个是显式报错桩；`,
  );
  lines.push(`// - 编译期擦除（type / interface）：**${typeNames.length}** 个 ⇒ 运行期不存在。`);
  if (surface.duplicates.length > 0) {
    lines.push(
      `// - 重载同名导出：${[...new Set(surface.duplicates)].join(" / ")}（按名字去重）。`,
    );
  }
  lines.push("//");
  lines.push("// ## 纪律");
  lines.push("//");
  lines.push("// - 已接线的导出喂**固定 fixture**（两侧同源）⇒ 不碰 HTTP / SSE 契约；");
  lines.push("// - **变更调用不接线**（`resolveCanonicalApproval` / `revokeGrant` 是桩）");
  lines.push("//   ⇒ 对照不覆盖「点签批 / 点撤销」之后的服务端状态；");
  lines.push("// - 其余导出**调用即抛错**（const 是**读取即抛错**）⇒ 不静默给假数据。");
  lines.push("");
  lines.push("/**");
  lines.push(" * 未接线的常量值（**读取即抛错** ⇒ 不静默给假数据）。");
  lines.push(" *");
  lines.push(" * @param {string} name 导出名（报错文案里点名）。");
  lines.push(" * @returns {unknown} 读取即抛错的替身。");
  lines.push(" */");
  lines.push("function notWiredValue(name) {");
  lines.push(
    "  const text = `runtime mock：导出「${name}」未接线（对照 harness 只喂固定 fixture）`;",
  );
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
  lines.push(
    `export {\n${WIRED.map((name) => `  ${name},`).join("\n")}\n} from "./runtime-mock-real.js";`,
  );
  lines.push("");
  lines.push("// ── 未接线（显式报错桩） ───────────────────────────────────────────────");
  const stubs = runtimeNames.filter((name) => !wiredSet.has(name));
  for (const name of stubs) {
    lines.push(renderStub(name, surface.kinds.get(name)));
    lines.push("");
  }
  return `${lines.join("\n").replace(/\n+$/, "")}\n`;
}

function main() {
  console.log(`仓库根：${REPO}`);
  console.log(`生产 runtime（只读）：${RUNTIME_TS}`);
  const surface = readExportSurface(readOrFail(RUNTIME_TS, "生产 runtime 模块"));
  const content = renderMock(surface);

  const current = existsSync(MOCK_JS) ? readFileSync(MOCK_JS, "utf8") : null;
  if (current === content) {
    console.log("  ✓ runtime mock 未漂移");
    return;
  }
  if (checkOnly) {
    throw new Error(
      `runtime mock 已漂移：${MOCK_JS}（请重跑 node scripts/gen-runtime-mock.mjs）`,
    );
  }
  mkdirSync(dirname(MOCK_JS), { recursive: true });
  writeFileSync(MOCK_JS, content, "utf8");
  const runtimeCount = [...surface.kinds.values()].filter(
    (kind) => !TYPE_KINDS.has(kind),
  ).length;
  console.log(
    `  → runtime mock 已写入 ${MOCK_JS}（导出语句 ${surface.statements} 条 / 运行期 ${runtimeCount} 个 / 接线 ${WIRED.length} 个）`,
  );
}

try {
  main();
} catch (error) {
  console.error(`生成失败：${error.message}`);
  process.exit(1);
}
