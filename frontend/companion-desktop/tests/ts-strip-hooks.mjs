// Node 模块加载钩子：用**项目自带的** TypeScript 编译器擦除类型。
//
// 为什么需要它（2026-11-07 实测）：`pnpm test` 的多数套件要 `import` `.ts` 源码，
// 原本靠 Node 内置的类型擦除（`--experimental-strip-types`）。但**发行版构建的 Node
// 可能没有内置 TS 支持** —— 实测本机 `/usr/bin/node` v24.21.0 的
// `process.features.typescript === false` ⇒ 报
// `ERR_NO_TYPESCRIPT: Node.js is not compiled with TypeScript support`
// ⇒ ⇒ 83 个套件里 66 个**根本加载不了**（不是断言失败，是模块加载就死）。
//
// 本钩子只做一件事：把 `.ts` / `.mts` 转成等价的 ESM（**纯语法层面**，不改变语义）。
// 用的是 `package.json` 里**已有**的 `typescript` devDependency
// ⇒ ⇒ **0 新外部依赖**。
//
// ⚠️ 只在 Node 缺少内置 TS 支持时启用（见 `tests/run-all.mjs` 的自动探测）
// ⇒ Node 有内置支持时走原生路径，**行为不变**（CI 不受影响）。
import {readFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';

import ts from 'typescript';

const TS_EXTENSIONS = ['.ts', '.mts', '.cts'];

export async function load(url, context, nextLoad) {
  const path = url.startsWith('file:') ? fileURLToPath(url) : '';
  const isTypeScript = TS_EXTENSIONS.some((extension) => path.endsWith(extension));
  if (!isTypeScript) return nextLoad(url, context);

  const source = await readFile(path, 'utf8');
  const {outputText} = ts.transpileModule(source, {
    compilerOptions: {
      // 与 Node 内置擦除的取向一致：**只去掉类型**，不改模块形态。
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ESNext,
      // `isolatedModules` 语义 = 逐文件转译（无跨文件类型信息），这正是加载钩子能做的。
      isolatedModules: true,
      sourceMap: false,
      inlineSourceMap: false,
      // 保留 JSX 原样（本包无 JSX 源码；命中时也报出而不是静默改语义）。
      jsx: ts.JsxEmit.Preserve,
    },
    fileName: path,
    reportDiagnostics: false,
  });

  return {format: 'module', source: outputText, shortCircuit: true};
}
