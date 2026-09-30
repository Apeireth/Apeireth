// 用户气泡对比度哨兵（内测实锤修复②防复发）：
//   病灶：星舰主题下用户气泡内文字黑字不可读。真凶 = tokens.css 星舰块内
//   `--ap-ink` 被同名后置声明覆盖（块内第二个声明赢），把为深空卡面准备的
//   冷白墨盖成深色；子树里自带墨色的两片叶子（.user-md .md-code/.md-inline）
//   在深空卡面同样不可读。
//   哨兵断言：
//   ① 星舰档用户气泡 fg/bg 对比度 ≥ 4.5:1（含子树叶子墨色，背景按叠底合成）；
//   ② 同名令牌后置覆盖即失败（星舰块 --ap-ink 只许声明一次）；
//   ③ 默认主题气泡零回归（默认令牌对与叶子墨色字节不动，且同样 ≥ 4.5:1）；
//   ④ 全主题矩阵：每个档位解析出的（--ap-ink, --ap-card）对都 ≥ 4.5:1。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const testsDir = dirname(fileURLToPath(import.meta.url));
const designDir = join(testsDir, '..', 'src', 'lib', 'design');
const tokensCss = readFileSync(join(designDir, 'tokens.css'), 'utf8');
const shellCss = readFileSync(join(designDir, 'shell.css'), 'utf8');
const hudCss = readFileSync(join(designDir, 'hud.css'), 'utf8');
const proseCss = readFileSync(join(designDir, 'prose.css'), 'utf8');

console.log('--- Starting Theme Bubble Contrast Sentinel ---');

// --------------------------- CSS 小解析器 ---------------------------
function stripComments(css) {
  return css.replace(/\/\*[\s\S]*?\*\//g, '');
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
    out.push({prelude: css.slice(idx, open).trim(), body: css.slice(open + 1, i)});
  }
  return out;
}

/** 声明表（保持出现顺序，同名后声明赢）→ Map。 */
function declarationList(body) {
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

function effectiveTokens(cssBlocks) {
  const map = new Map();
  for (const block of cssBlocks) {
    for (const [prop, value] of declarationList(block.body)) map.set(prop, value);
  }
  return map;
}

// --------------------------- 颜色与对比度 ---------------------------
function parseColor(value) {
  const hex = value.trim();
  const m6 = /^#([0-9a-fA-F]{6})$/.exec(hex);
  if (m6) {
    const n = parseInt(m6[1], 16);
    return {rgb: [(n >> 16) & 255, (n >> 8) & 255, n & 255], alpha: 1};
  }
  const m3 = /^#([0-9a-fA-F]{3})$/.exec(hex);
  if (m3) {
    const [r, g, b] = m3[1].split('').map((c) => parseInt(c + c, 16));
    return {rgb: [r, g, b], alpha: 1};
  }
  const rgba = /^rgba?\(([^)]+)\)$/i.exec(hex);
  if (rgba) {
    const parts = rgba[1].split(',').map((s) => Number.parseFloat(s.trim()));
    return {rgb: [parts[0], parts[1], parts[2]], alpha: parts.length > 3 ? parts[3] : 1};
  }
  throw new Error(`无法解析的色值: ${value}`);
}

function linear(channel) {
  const c = channel / 255;
  return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

function luminance([r, g, b]) {
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
}

/** WCAG 对比度（1 ~ 21）。 */
function contrastRatio(fg, bg) {
  const l1 = luminance(fg);
  const l2 = luminance(bg);
  const [hi, lo] = l1 > l2 ? [l1, l2] : [l2, l1];
  return (hi + 0.05) / (lo + 0.05);
}

/** 前景色按 alpha 叠在底色上（叶子背景 rgba(0,0,0,α) 盖在卡面上）。 */
function composite(top, bottom) {
  const a = top.alpha;
  return top.rgb.map((c, i) => Math.round(a * c + (1 - a) * bottom.rgb[i]));
}

function pairRatio(fgValue, bgValue) {
  const fg = parseColor(fgValue);
  const bg = parseColor(bgValue);
  return contrastRatio(fg.rgb, bg.rgb);
}

// --------------------------- 解析令牌 ---------------------------
const tokens = stripComments(tokensCss);
const rootBlock = ruleBodies(tokens, ':root')[0];
assert.ok(rootBlock, 'tokens.css 仍有 :root 基准块');
const rootTokens = effectiveTokens([rootBlock]);

const THEME_NAMES = ['essence', 'day', 'paper', 'ocean', 'forest', 'starship'];
const themeTokens = new Map([['(default)', rootTokens]]);
for (const name of THEME_NAMES) {
  const blocks = ruleBodies(tokens, `html[data-theme='${name}']`);
  const merged = new Map(rootTokens);
  for (const [prop, value] of blocks.flatMap((b) => declarationList(b.body))) merged.set(prop, value);
  themeTokens.set(name, merged);
}

// --------------------------- ② 星舰档哨兵 ---------------------------
{
  const starshipBlocks = ruleBodies(tokens, "html[data-theme='starship']");
  assert.ok(starshipBlocks.length >= 1, 'tokens.css 有星舰主题块');

  // 同名后置覆盖 = 实机黑字的真凶路径：块内同名令牌只许声明一次。
  const inkDecls = starshipBlocks
    .flatMap((b) => declarationList(b.body))
    .filter(([prop]) => prop === '--ap-ink');
  assert.equal(
    inkDecls.length,
    1,
    `星舰块 --ap-ink 只许声明一次（后置声明会覆盖先声明值）：实际 ${inkDecls.length} 处 ${JSON.stringify(inkDecls)}`,
  );

  const starship = themeTokens.get('starship');
  const fg = starship.get('--ap-ink');
  const bg = starship.get('--ap-card');
  const ratio = pairRatio(fg, bg);
  assert.ok(
    ratio >= 4.5,
    `星舰档用户气泡对比度不足: --ap-ink ${fg} 对 --ap-card ${bg} = ${ratio.toFixed(2)}:1（要求 ≥ 4.5:1）`,
  );

  // 子树叶子墨色（行内代码/代码块）：星舰档必须经 hud.css 令牌换冷白墨。
  const hud = stripComments(hudCss);
  assert.ok(
    /html\[data-theme='starship'\]\s*\.user-md\s*\.md-code[\s\S]{0,120}?color:\s*var\(--ap-hud-/.test(hud),
    '星舰档 .user-md .md-code 墨色必须走 HUD 令牌（prose 浅底墨值在深空卡面不可读）',
  );
  assert.ok(
    /html\[data-theme='starship'\]\s*\.user-md\s*\.md-inline[\s\S]{0,120}?color:\s*var\(--ap-hud-/.test(hud) ||
      /html\[data-theme='starship'\]\s*\.user-md\s*\.md-code,\s*\n?\s*html\[data-theme='starship'\]\s*\.user-md\s*\.md-inline\s*\{[\s\S]{0,120}?color:\s*var\(--ap-hud-/.test(hud),
    '星舰档 .user-md .md-inline 墨色必须走 HUD 令牌',
  );

  const leafInk = parseColor(starship.get('--ap-hud-title-ink')).rgb;
  for (const [label, wash] of [
    ['md-code', 0.08],
    ['md-inline', 0.06],
  ]) {
    const card = parseColor(bg);
    const leafBg = composite({rgb: [0, 0, 0], alpha: wash}, card);
    const leafRatio = contrastRatio(leafInk, leafBg);
    assert.ok(
      leafRatio >= 4.5,
      `星舰档用户气泡 ${label} 叶子对比度不足: ${leafRatio.toFixed(2)}:1（要求 ≥ 4.5:1）`,
    );
    console.log(`  -> 星舰 ${label} 叶子对比度 ${leafRatio.toFixed(2)}:1`);
  }
  console.log(`  -> PASS: 星舰用户气泡对比度 ${ratio.toFixed(2)}:1（单声明 + 叶子换墨守卫）`);
}

// --------------------------- ③ 默认主题零回归 ---------------------------
{
  // 字节哨兵：默认令牌对不动（浅底墨字路径零回归）。
  assert.ok(tokensCss.includes('--ap-ink: #26262a;'), '默认 --ap-ink 被改动');
  assert.ok(tokensCss.includes('--ap-card: #ece7da;'), '默认 --ap-card 被改动');

  const ratio = pairRatio('#26262a', '#ece7da');
  assert.ok(ratio >= 4.5, `默认主题气泡对比度不足: ${ratio.toFixed(2)}:1`);

  // 叶子墨色浅色主题原值不动（零回归哨兵）且保持可读。
  assert.ok(proseCss.includes('color: #1a1a1e;'), '.user-md .md-code 浅色档墨值被改动');
  assert.ok(proseCss.includes('color: #a8381e;'), '.user-md .md-inline 浅色档墨值被改动');
  for (const [label, ink, wash] of [
    ['md-code', '#1a1a1e', 0.08],
    ['md-inline', '#a8381e', 0.06],
  ]) {
    const leafBg = composite({rgb: [0, 0, 0], alpha: wash}, parseColor('#ece7da'));
    const leafRatio = contrastRatio(parseColor(ink).rgb, leafBg);
    assert.ok(leafRatio >= 4.5, `默认主题 ${label} 叶子对比度不足: ${leafRatio.toFixed(2)}:1`);
    console.log(`  -> 默认 ${label} 叶子对比度 ${leafRatio.toFixed(2)}:1`);
  }

  // 气泡取色链守卫：.user-card 走令牌对、文字层逐级 inherit（子树最终 color
  // 就是 --ap-ink；任何一层再引深色字面量都会绕过哨兵）。
  assert.ok(
    /\.user-card\s*\{[^}]*background:\s*var\(--ap-card\);[^}]*color:\s*var\(--ap-ink\);/.test(stripComments(shellCss)),
    '.user-card 必须走 --ap-card/--ap-ink 令牌对',
  );
  assert.ok(
    /\.user-card\s*:global\(\.user-text\)\s*\{[^}]*color:\s*inherit;/.test(stripComments(shellCss)),
    '.user-text 必须继承气泡墨色（不得自带 color）',
  );
  assert.ok(
    /\.user-md\s*\{[^}]*color:\s*inherit;/.test(stripComments(proseCss)),
    '.user-md 必须继承气泡墨色（不得自带 color）',
  );
  console.log(`  -> PASS: 默认主题气泡零回归（对比度 ${ratio.toFixed(2)}:1 + 字节哨兵 + 取色链）`);
}

// --------------------------- ④ 全主题矩阵 ---------------------------
{
  for (const [name, tokens] of themeTokens) {
    const fg = tokens.get('--ap-ink');
    const bg = tokens.get('--ap-card');
    assert.ok(fg && bg, `${name} 档缺 --ap-ink/--ap-card`);
    const ratio = pairRatio(fg, bg);
    assert.ok(
      ratio >= 4.5,
      `${name} 档用户气泡对比度不足: --ap-ink ${fg} 对 --ap-card ${bg} = ${ratio.toFixed(2)}:1`,
    );
    console.log(`  -> ${name}: ${ratio.toFixed(2)}:1`);
  }
  console.log(`  -> PASS: 全主题用户气泡对比度矩阵 ≥ 4.5:1（${themeTokens.size} 档）`);
}

console.log('--- All Theme Bubble Contrast Sentinel Checks PASSED! ---');
