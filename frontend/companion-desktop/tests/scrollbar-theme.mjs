// 滚动条主题令牌单测（显白病灶修复；源码镜像口径）。
//
// 病灶：自定义滚动条只覆盖 4 个选择器且硬编码骨白；其余滚动容器落回
// Chromium 默认白滚动条 —— 深色主题下即"侧边异常白色色块"。
// 盯死三组：
//   ① 令牌覆盖矩阵：:root 深底基准 + 三个浅色主题必须覆写为墨色低透
//      （浅底用骨白 = 发白），星舰冷白覆写；任何滚动条令牌禁止取白；
//   ② 全局规则：伪元素规则必须是全局选择器 + 令牌引用（所有滚动容器统一）；
//   ③ 无残留硬编码：shell.css 滚动条规则里不许再出现字面色值。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const tokensSrc = readFileSync(join(here, '../src/lib/design/tokens.css'), 'utf8');
const shellSrc = readFileSync(join(here, '../src/lib/design/shell.css'), 'utf8');
// ②③ 在去注释后的源码上断言（注释里引用旧硬编码值是合法的历史记录）。
const shell = shellSrc.replace(/\/\*[\s\S]*?\*\//g, '');

/** 取 `selector { ... }` 块体（该文件主题块内无嵌套花括号，首个 } 即块尾）。 */
function blockOf(src, selector) {
  const at = src.indexOf(selector);
  assert.ok(at >= 0, `缺少块: ${selector}`);
  const end = src.indexOf('}', at);
  return src.slice(at, end);
}

// ---- ① 令牌覆盖矩阵 ----
{
  const rootBlock = blockOf(tokensSrc, ':root {');
  assert.ok(rootBlock.includes('--ap-scrollbar-thumb:'), ':root 必须定义滚动条基准（深底主题继承）');
  assert.ok(rootBlock.includes('--ap-scrollbar-thumb-hover:'), ':root 必须定义 hover 令牌');
  assert.ok(rootBlock.includes('--ap-scrollbar-track: transparent'), '轨道透明（不显色块）');

  for (const theme of ['origin', 'essence', 'day', 'paper']) {
    const block = blockOf(tokensSrc, `html[data-theme='${theme}'] {`);
    assert.ok(
      /--ap-scrollbar-thumb: rgba\((28, 27, 31|31, 30, 34|31, 35, 41), 0\.(22|2)\d*\)/.test(block),
      `${theme} 浅底主题必须把滑块覆写为墨色低透（骨白在浅底发白）`,
    );
  }
  const starship = blockOf(tokensSrc, "html[data-theme='starship'] {");
  assert.ok(/--ap-scrollbar-thumb: rgba\((228, 237, 245)/.test(starship), '星舰覆写冷白骨色（与 --ap-bone 同族）');

  // 任何滚动条令牌都不许取白（显白病灶的机器断言）。
  const tokenLines = tokensSrc.split('\n').filter((l) => l.includes('--ap-scrollbar-'));
  for (const line of tokenLines) {
    assert.ok(
      !/#fff|#ffffff|255, 255, 255/i.test(line),
      `滚动条令牌禁止取白: ${line.trim()}`,
    );
  }
  console.log('  ok  ① 令牌矩阵（:root 基准 + 浅底墨色覆写 + 星舰冷白 + 零取白）');
}

// ---- ② 全局规则 + 令牌引用 ----
{
  assert.ok(/(^|\n)::-webkit-scrollbar \{/.test(shell), '全局伪元素规则（覆盖所有滚动容器）');
  assert.ok(
    /::-webkit-scrollbar-thumb \{\s*background: var\(--ap-scrollbar-thumb\);/.test(shell),
    '滑块颜色走令牌',
  );
  assert.ok(
    /::-webkit-scrollbar-thumb:hover \{\s*background: var\(--ap-scrollbar-thumb-hover\);/.test(shell),
    'hover 走令牌',
  );
  assert.ok(
    /::-webkit-scrollbar-track \{\s*background: var\(--ap-scrollbar-track\);/.test(shell),
    '轨道走令牌',
  );
  assert.ok(/scrollbar-color: var\(--ap-scrollbar-thumb\)/.test(shell), '标准属性兜底在案');
  console.log('  ok  ② 全局规则（伪元素 + 标准属性兜底，全部令牌引用）');
}

// ---- ③ 无残留硬编码色 ----
{
  const scrollbarRules = shell.split('}').filter((chunk) => chunk.includes('::-webkit-scrollbar'));
  assert.ok(scrollbarRules.length >= 4, '滚动条规则在案（scrollbar/track/thumb/thumb:hover）');
  for (const chunk of scrollbarRules) {
    assert.ok(!/rgba?\(/.test(chunk), `滚动条规则不许硬编码色值: ${chunk.trim().slice(0, 60)}…`);
    assert.ok(!/#([0-9a-f]{3,6})\b/i.test(chunk), `滚动条规则不许硬编码十六进制色: ${chunk.trim().slice(0, 60)}…`);
  }
  assert.ok(!/::-webkit-scrollbar[^}]*rgba\(232, 224, 204/.test(shell), '旧硬编码骨白滑块已移除');
  console.log('  ok  ③ 零硬编码（滚动条规则全令牌化）');
}

console.log('--- All Scrollbar Theme (white-block fix) Checks PASSED! ---');
