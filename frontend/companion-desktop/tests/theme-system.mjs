// 主题系统纯逻辑单测（规范 §8 + 2026-09-22 增补段）
// 直接 import 真实实现 ../src/lib/theme.ts（Node ≥23.6 类型擦除；模块 DOM-free：
// applyDocumentTheme 内部有 typeof document 守门，导入无副作用）。
//
// 锁定主人拍板纪律：
//   ① 默认主题 = origin 原初（2026-10-03 拍板，DeepSeek 原生界面风：白底/品牌蓝/
//      零动画纯色层；取代 2026-09-22 的 heritage-void 默认，该主题保留在目录）；
//   ② ?theme= URL 覆写纪律不破（query 优先于 config）；
//   ③ 静态背景主题判定准确（场景层隐藏 + 渲染循环暂停的依据）；
//   ④ 补全主题必须真实现（2026-09-23 主人指示「最后我们都是要做的」）：
//      day/ocean/forest/paper 当日曾作为空心项删除，现在令牌+背景补全回归现役。
// 不依赖后端进程。
import assert from 'node:assert/strict';

import {
  VALID_THEMES,
  THEME_CATALOG,
  resolveTheme,
  isStaticBgTheme,
  themeLabel,
  VALID_ACCENTS,
  ACCENT_CATALOG,
  resolveAccent,
  isPresenceGoldFamily,
} from '../src/lib/theme.ts';

console.log('--- Starting Theme System Check ---');

// ---------------------------------------------------------------------------
// 1. 默认回落 = origin 原初（2026-10-03 拍板；已保存的 config 主题原样保留 = 不改现有配置）
// ---------------------------------------------------------------------------
{
  assert.equal(resolveTheme(undefined, null), 'origin', '无配置无 query 必须回落 origin');
  assert.equal(resolveTheme('night', null), 'night', 'config 合法主题原样保留');
  assert.equal(resolveTheme('essence', null), 'essence');
  assert.equal(resolveTheme('heritage-void', null), 'heritage-void', '旧默认主题仍可选、可保留');
  // @ts-expect-error 故意传非法值
  assert.equal(resolveTheme('dark', null), 'origin', '非法 config 主题回落 origin');
}

// ---------------------------------------------------------------------------
// 2. ?theme= URL 覆写纪律（开发/截图链路依赖）：query 合法时压过 config
// ---------------------------------------------------------------------------
{
  assert.equal(resolveTheme('heritage-void', 'night'), 'night', 'query 优先于 config');
  assert.equal(resolveTheme('night', 'essence'), 'essence');
  assert.equal(resolveTheme(undefined, 'heritage-void'), 'heritage-void');
  assert.equal(resolveTheme(undefined, 'origin'), 'origin');
  assert.equal(resolveTheme('night', 'bogus'), 'night', '非法 query 不覆写，回落 config');
  assert.equal(resolveTheme(undefined, 'bogus'), 'origin', '全非法时回落默认');
}

// ---------------------------------------------------------------------------
// 3. 静态背景主题判定：静态主题隐藏场景并暂停渲染；night 保留实时场景
// ---------------------------------------------------------------------------
{
  assert.equal(isStaticBgTheme('origin'), true, 'origin = 静态纯色背景（零动画，无 WebGL 场景层）');
  assert.equal(isStaticBgTheme('heritage-void'), true);
  assert.equal(isStaticBgTheme('essence'), true);
  assert.equal(isStaticBgTheme('night'), false, 'night = 黑洞实时场景（可选主题），不暂停');
  // 2026-09-23 四主题补全（主人指示「最后我们都是要做的」）：day/ocean/forest/paper
  // 从空心回归现役——有真实现（tokens/base/shell 令牌+背景），必须合法、静态、有目录条目。
  // 2026-10-03 noir 纯黑档（主人拍板）同纪律入列。
  for (const id of ['day', 'ocean', 'forest', 'paper', 'noir']) {
    assert.equal(VALID_THEMES.includes(id), true, `补全主题 ${id} 必须在 VALID_THEMES`);
    assert.equal(isStaticBgTheme(id), true, `补全主题 ${id} 为静态背景（无 WebGL 场景层）`);
    assert.equal(resolveTheme(id), id, `config 里的 ${id} 原样保留`);
    assert.ok(THEME_CATALOG.find((t) => t.id === id), `补全主题 ${id} 必须有 THEME_CATALOG 条目`);
  }
}

// ---------------------------------------------------------------------------
// 4. 目录完整性：每个合法主题都有目录条目；heritage-void 指向入库资产而非 artifacts
// ---------------------------------------------------------------------------
{
  for (const id of VALID_THEMES) {
    const entry = THEME_CATALOG.find((t) => t.id === id);
    assert.ok(entry, `主题 ${id} 缺少 THEME_CATALOG 条目`);
    assert.ok(entry.label && entry.desc && entry.swatch, `主题 ${id} 条目字段不全`);
  }
  const heritage = THEME_CATALOG.find((t) => t.id === 'heritage-void');
  assert.ok(heritage.swatch.includes('/assets/themes/heritage-void-bg.png'), 'heritage-void 预览图必须指向 public 入库资产');
  assert.ok(!heritage.swatch.includes('artifacts/'), '禁止引用 artifacts 路径（主人供图已复制入库）');
  // 主题目录首项 = 默认主题，设置面板里它排在最前
  // 2026-10-03 拍板：origin 原初取默认位（旧"新档不许插队"默认位锁定由主人改判；
  // 其后新增档位仍不得插队）。
  assert.equal(THEME_CATALOG[0].id, 'origin', '目录首项应为默认主题 origin');
  assert.equal(themeLabel('origin'), '原初');
  assert.equal(themeLabel('heritage-void'), '遗产星空');
}

// ---------------------------------------------------------------------------
// 4b. 简洁高效档零动画纯色层（源码镜像）：主人要求"不要求即时演算的动画，只要求
//     简洁高效"——origin/noir 主题块只许声明颜色令牌，不得引入动画/过渡/关键帧。
// ---------------------------------------------------------------------------
{
  const {readFileSync} = await import('node:fs');
  const {dirname, join} = await import('node:path');
  const {fileURLToPath} = await import('node:url');
  const here = dirname(fileURLToPath(import.meta.url));
  const tokensSrc = readFileSync(join(here, '../src/lib/design/tokens.css'), 'utf8');
  const shellSrc = readFileSync(join(here, '../src/lib/design/shell.css'), 'utf8');
  for (const id of ['origin', 'noir']) {
    const at = tokensSrc.indexOf(`html[data-theme='${id}'] {`);
    assert.ok(at >= 0, `tokens.css 有 ${id} 主题块`);
    const block = tokensSrc.slice(at, tokensSrc.indexOf('}', at));
    assert.ok(!/animation|transition|@keyframes/i.test(block), `${id} 令牌块零动画（纯色层）`);
    for (const m of shellSrc.matchAll(new RegExp(`\\.app-root\\.theme-${id}[^{]*\\{[^}]*\\}`, 'g'))) {
      assert.ok(!/animation|transition|@keyframes/i.test(m[0]), `${id} 规则零动画: ${m[0].slice(0, 60)}…`);
      assert.ok(!/url\(\/assets/i.test(m[0]), `${id} 背景零照片资产（纯静态渐变，简洁高效）`);
    }
    assert.ok(new RegExp(`\\.app-root\\.theme-${id} \\.static-bg`).test(shellSrc), `${id} 静态背景规则在案`);
  }
  // origin 档全档去 backdrop-filter（2026-10-03）：平白底模糊=视觉空操作，
  // 关掉省 GPU（勘误注：先前归因光标伪影有误，规则理由以 shell.css 注释为准）。
  assert.ok(
    /html\[data-theme='origin'\] \*[\s\S]{0,300}?backdrop-filter: none !important/.test(shellSrc),
    'origin 必须全档关闭 backdrop-filter（省 GPU 合成开销）',
  );
  // 浅底四主题的文本光标（2026-10-03 主人反馈②）：白色指针方案的 I-beam 在
  // 白底"几乎不可见"——输入面必须换深色 I-beam（内联 SVG，白描边）。
  const essenceSrc = readFileSync(join(here, '../src/lib/design/essence.css'), 'utf8');
  for (const id of ['origin', 'essence', 'day', 'paper']) {
    assert.ok(
      new RegExp(`html\\[data-theme='${id}'\\] :is\\(input:not\\(\\[type\\]\\)[\\s\\S]{0,1200}?cursor: url\\("data:image/svg\\+xml`).test(essenceSrc),
      `${id} 输入面必须挂深色 I-beam 文本光标（白色系统光标在浅底不可见）`,
    );
  }
  // 深底主题保留系统光标（白色 I-beam 在深底正常可见，不越界改）。
  assert.ok(!/html\[data-theme='noir'\] :is\(input/.test(essenceSrc), 'noir 等深底主题不挂自定义文本光标');
}

// ---------------------------------------------------------------------------
// 5. UI 配色方案（§8 增补⑤）：resolveAccent 回落纪律与非法值守卫
// ---------------------------------------------------------------------------
{
  assert.equal(resolveAccent(undefined), 'presence-gold', '未配置必须回落存在金（默认态）');
  assert.equal(resolveAccent('deep-space'), 'deep-space', '合法 accent 原样保留');
  assert.equal(resolveAccent('sage'), 'sage');
  assert.equal(resolveAccent('bone'), 'bone');
  // @ts-expect-error 故意传非法值
  assert.equal(resolveAccent('gold'), 'presence-gold', '非法 accent 回落存在金');
  // @ts-expect-error 故意传非法值
  assert.equal(resolveAccent('#ffd27a'), 'presence-gold', 'hex 字符串不是合法 accent id');
}

// ---------------------------------------------------------------------------
// 6. 金色纪律机器断言：存在金家族判定 + 可选配色里绝无金色系
//    存在金是他的；accent 只染 UI chrome，可选方案不许偷渡金色。
// ---------------------------------------------------------------------------
{
  // sanity：他的金必须落在家族区间内
  assert.equal(isPresenceGoldFamily('#ffd27a'), true, '存在金 #ffd27a 必须在金色家族内');
  assert.equal(isPresenceGoldFamily('#e8a33d'), true, '琥珀金 #e8a33d 必须在金色家族内');
  // 纪律：每个非默认可选配色都必须被家族判定排除
  for (const opt of ACCENT_CATALOG) {
    if (opt.id === 'presence-gold') continue;
    assert.equal(
      isPresenceGoldFamily(opt.accent),
      false,
      `可选配色 ${opt.id}（${opt.accent}）闯入金色家族——金色纪律被破`,
    );
  }
  // 默认色本身当然在家族内（否则目录自相矛盾）
  const gold = ACCENT_CATALOG.find((a) => a.id === 'presence-gold');
  assert.equal(isPresenceGoldFamily(gold.accent), true, 'presence-gold 目录色必须属于金色家族');
}

// ---------------------------------------------------------------------------
// 7. accent 目录完整性：每个合法 id 有条目；accent/ink 均为合法 #rrggbb hex
//    （tokens.css 的 data-accent 覆写与设置预览真渲染共用这份值）
// ---------------------------------------------------------------------------
{
  for (const id of VALID_ACCENTS) {
    const entry = ACCENT_CATALOG.find((a) => a.id === id);
    assert.ok(entry, `accent ${id} 缺少 ACCENT_CATALOG 条目`);
    assert.ok(entry.label && entry.desc, `accent ${id} 条目字段不全`);
    assert.match(entry.accent, /^#[0-9a-f]{6}$/i, `accent ${id} 主色须为 #rrggbb`);
    assert.match(entry.ink, /^#[0-9a-f]{6}$/i, `accent ${id} ink 须为 #rrggbb`);
  }
  assert.equal(ACCENT_CATALOG[0].id, 'presence-gold', 'accent 目录首项应为默认配色');
}

console.log('Theme system check passed.');
