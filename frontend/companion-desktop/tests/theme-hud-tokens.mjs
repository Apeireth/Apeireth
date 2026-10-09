// 星舰主题层（科幻 HUD 档）变量层源码镜像断言：
//   ① 令牌覆盖完整：核心 --ap-* / 旧 Pattern 令牌 / 档案调 / HUD 数据光令牌
//      全部在星舰块里落值——不留「换档后还是旧色」的漏网变量；
//   ② 无硬编码漏网色值：星舰块只许自定义属性带色值，hud.css 零色值
//      （颜色只经 var() 引用，换色只改令牌一处）；
//   ③ reduced-motion 生效：氛围动效只在 no-preference 下启用，令牌层保留
//      reduce 全局降级兜底；
//   ④ 默认零回归：hud.css 顶层规则全部以 html[data-theme='starship'] 为域，
//      既有默认值字节不动（哨兵值比对）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const testsDir = dirname(fileURLToPath(import.meta.url));
const designDir = join(testsDir, '..', 'src', 'lib', 'design');
const tokensCss = readFileSync(join(designDir, 'tokens.css'), 'utf8');
const baseCss = readFileSync(join(designDir, 'base.css'), 'utf8');
const hudCss = readFileSync(join(designDir, 'hud.css'), 'utf8');
const stylesCss = readFileSync(join(testsDir, '..', 'src', 'styles.css'), 'utf8');

console.log('--- Starting Theme HUD-Layer Tokens Check ---');

/** 去注释（注释里的示例色值不算硬编码）。 */
function stripComments(css) {
  return css.replace(/\/\*[\s\S]*?\*\//g, '');
}

/** 色值字面量：#hex / rgb() / hsl()（id 选择器如 #presence 不会命中——首字符非 hex）。 */
const COLOR_LITERAL = /#[0-9a-fA-F]{3,8}\b|\b(?:rgb|rgba|hsl|hsla)\s*\(/;

/** 声明表：prop -> value（body 内顶层按 ; 切分）。 */
function declarations(body) {
  const out = [];
  for (const part of body.split(';')) {
    const colon = part.indexOf(':');
    if (colon === -1) continue;
    const prop = part.slice(0, colon).trim();
    const value = part.slice(colon + 1).trim();
    if (prop) out.push([prop, value]);
  }
  return out;
}

/** 以 prelude 含 needle 的规则为域，返回 {prelude, body} 列表（花括号配对）。 */
function ruleBodies(css, needle) {
  const out = [];
  const seen = new Set();
  let from = 0;
  while (true) {
    const idx = css.indexOf(needle, from);
    if (idx === -1) break;
    from = idx + needle.length;
    const open = css.indexOf('{', idx);
    if (open === -1) break;
    if (seen.has(open)) continue;
    seen.add(open);
    let depth = 0;
    let i = open;
    for (; i < css.length; i++) {
      if (css[i] === '{') depth += 1;
      else if (css[i] === '}') {
        depth -= 1;
        if (depth === 0) break;
      }
    }
    const delim = Math.max(css.lastIndexOf('}', open), css.lastIndexOf(';', open), css.lastIndexOf('{', open - 1));
    out.push({prelude: css.slice(delim + 1, open).trim(), body: css.slice(open + 1, i)});
  }
  return out;
}

/** 顶层（depth 0）规则的 prelude 列表。 */
function topLevelPreludes(css) {
  const out = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < css.length; i++) {
    if (css[i] === '{') {
      if (depth === 0) {
        const delim = Math.max(css.lastIndexOf('}', i), css.lastIndexOf(';', i), css.lastIndexOf('{', i - 1));
        out.push(css.slice(delim + 1, i).trim());
      }
      depth += 1;
    } else if (css[i] === '}') {
      depth -= 1;
    }
  }
  return out;
}

const tokens = stripComments(tokensCss);
const base = stripComments(baseCss);
const hud = stripComments(hudCss);
const starshipBlocks = [
  ...ruleBodies(tokens, "html[data-theme='starship']"),
  ...ruleBodies(base, "html[data-theme='starship']"),
];

// ---------------------------------------------------------------------------
// 1. 令牌覆盖完整：核心 --ap-* / 旧 Pattern / 档案调 / HUD 令牌全部落值
// ---------------------------------------------------------------------------
{
  const defined = new Set();
  for (const block of starshipBlocks) {
    for (const [prop] of declarations(block.body)) defined.add(prop);
  }
  const REQUIRED_AP = [
    '--ap-space-void', '--ap-space-horizon',
    '--ap-bone', '--ap-bone-68', '--ap-bone-42', '--ap-bone-30', '--ap-ink',
    '--ap-panel', '--ap-panel-solid', '--ap-shell-chip', '--ap-shell-elevated',
    '--ap-card', '--ap-card-dark', '--ap-line', '--ap-stamp',
    '--ap-semantic-success', '--ap-semantic-warning', '--ap-semantic-danger', '--ap-semantic-info',
  ];
  const REQUIRED_ARCHIVE = [
    '--ap-register-archive-paper', '--ap-register-archive-paper-lo', '--ap-register-archive-paper-hi',
    '--ap-register-archive-ink', '--ap-register-archive-ink-60', '--ap-register-archive-ink-55',
    '--ap-register-archive-ink-40', '--ap-register-archive-ink-35', '--ap-register-archive-ink-26',
    '--ap-register-archive-ink-14', '--ap-register-archive-ghost-number', '--ap-register-archive-icon-gray',
    '--ap-register-archive-card-dark', '--ap-register-archive-card-deep', '--ap-register-archive-ink-gold',
    '--ap-register-archive-surface-2', '--ap-register-archive-surface-3',
  ];
  const REQUIRED_LEGACY = [
    '--bg', '--surface', '--surface-2', '--surface-3',
    '--line', '--line-strong', '--text', '--muted', '--faint',
    '--amber', '--amber-hi', '--amber-wash', '--amber-line',
    '--green', '--green-wash', '--blue', '--blue-wash', '--danger', '--shadow', '--bg-input',
  ];
  const REQUIRED_HUD = [
    '--ap-hud-cyan', '--ap-hud-cyan-soft', '--ap-hud-cyan-line',
    '--ap-hud-amber', '--ap-hud-amber-soft',
    '--ap-hud-title-ink', '--ap-hud-title-glow', '--ap-hud-title-glow-hi',
    '--ap-hud-data-glow', '--ap-hud-amber-glow', '--ap-hud-amber-glow-hi', '--ap-hud-danger-glow',
    '--ap-hud-corner', '--ap-hud-rule', '--ap-hud-track', '--ap-hud-bar-fill',
    '--ap-hud-sky', '--ap-hud-veil', '--ap-hud-scan',
  ];
  for (const prop of [...REQUIRED_AP, ...REQUIRED_ARCHIVE, ...REQUIRED_LEGACY, ...REQUIRED_HUD]) {
    assert.ok(defined.has(prop), `星舰令牌层缺 ${prop}（变量层覆盖必须完整）`);
  }
  // 存在金家族不许在新档重染（金色是他的存在色，纪律机器断言）
  for (const prop of defined) {
    assert.ok(!prop.startsWith('--ap-gold'), `星舰令牌层重染了存在金 ${prop}——金色纪律被破`);
  }
  // HUD 数据光令牌不得泄漏到 :root（默认主题零变化）
  const rootBlock = ruleBodies(tokens, ':root')[0];
  assert.ok(rootBlock, 'tokens.css 仍有 :root 基准块');
  assert.ok(!rootBlock.body.includes('--ap-hud-'), 'HUD 令牌不许进 :root（默认主题零变化）');
  console.log('  -> PASS: 令牌覆盖完整（核心/档案/旧 Pattern/HUD 全落值，存在金不动）');
}

// ---------------------------------------------------------------------------
// 2. 无硬编码漏网色值：星舰块只许自定义属性带色值；hud.css 零色值
// ---------------------------------------------------------------------------
{
  for (const block of starshipBlocks) {
    for (const [prop, value] of declarations(block.body)) {
      if (prop.startsWith('--')) continue; // 令牌声明处是色值唯一落点
      assert.ok(!COLOR_LITERAL.test(value), `星舰块 ${prop} 硬编码色值漏网：${value}`);
    }
  }
  assert.ok(!COLOR_LITERAL.test(hud), 'hud.css 必须零色值（颜色只经 var() 引用令牌）');
  assert.ok(hud.includes('var(--ap-hud-'), 'hud.css 必须经 HUD 令牌取色');
  console.log('  -> PASS: 无硬编码漏网色值（色值只在令牌声明处）');
}

// ---------------------------------------------------------------------------
// 3. reduced-motion 生效：动效只在 no-preference 下；reduce 全局降级兜底
// ---------------------------------------------------------------------------
{
  const noPrefSpans = [];
  for (const m of hud.matchAll(/@media \(prefers-reduced-motion: no-preference\)/g)) {
    const open = hud.indexOf('{', m.index);
    let depth = 0;
    let i = open;
    for (; i < hud.length; i++) {
      if (hud[i] === '{') depth += 1;
      else if (hud[i] === '}') {
        depth -= 1;
        if (depth === 0) break;
      }
    }
    noPrefSpans.push([open, i]);
  }
  assert.ok(noPrefSpans.length > 0, 'hud.css 应有 no-preference 动效块');
  for (const m of hud.matchAll(/animation\s*:/g)) {
    assert.ok(
      noPrefSpans.some(([a, b]) => m.index > a && m.index < b),
      `animation 声明必须包在 prefers-reduced-motion: no-preference 内（偏移 ${m.index}）`,
    );
  }
  // 令牌层的全局降级开关仍在（reduce 时 animation/transition 全停）
  assert.ok(
    /@media \(prefers-reduced-motion: reduce\)[\s\S]*?animation: none !important/.test(tokens),
    'tokens.css 必须保留 prefers-reduced-motion: reduce 的全局降级',
  );
  console.log('  -> PASS: reduced-motion 生效（动效守卫 + reduce 兜底）');
}

// ---------------------------------------------------------------------------
// 4. 默认零回归：hud.css 全域限定 + 既有默认值字节不动
// ---------------------------------------------------------------------------
{
  for (const prelude of topLevelPreludes(hud)) {
    if (prelude.startsWith('@media')) {
      assert.ok(prelude.includes('prefers-reduced-motion: no-preference'), `hud.css 顶层 @media 只许动效守卫：${prelude}`);
    } else if (prelude.startsWith('@keyframes')) {
      // keyframes 名称带 hud 前缀，不与既有动画冲突
      assert.ok(/@keyframes ap-hud-/.test(prelude), `keyframes 必须用 ap-hud- 前缀：${prelude}`);
    } else {
      assert.ok(
        prelude.includes("html[data-theme='starship']"),
        `hud.css 顶层规则必须以星舰主题为域：${prelude}`,
      );
    }
  }
  assert.ok(stylesCss.includes("@import './lib/design/hud.css';"), 'styles.css 必须接入 hud.css 主题层');
  // 哨兵值：既有默认/浅色档的字节零回归（改动这些值即失败）
  for (const sentinel of [
    '--ap-gold: #ffd27a;',
    '--ap-panel: rgba(11, 13, 18, 0.82);',
    '--ap-card: #ece7da;',
    '--ap-line: rgba(232, 224, 204, 0.14);',
    '--ap-space-void: #e6e2da;',
  ]) {
    assert.ok(tokensCss.includes(sentinel), `tokens.css 既有值被改动：${sentinel}`);
  }
  for (const sentinel of ['--bg: #0e0d10;', '--text: #ebe8e2;', '--surface: #151419;']) {
    assert.ok(baseCss.includes(sentinel), `base.css 既有值被改动：${sentinel}`);
  }
  console.log('  -> PASS: 默认零回归（主题为域 + 哨兵值字节不动）');
}

console.log('--- All Theme HUD-Layer Tokens Checks PASSED! ---');
