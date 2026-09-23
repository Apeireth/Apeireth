// 主题系统纯逻辑单测（规范 §8 + 2026-09-22 增补段）
// 直接 import 真实实现 ../src/lib/theme.ts（Node ≥23.6 类型擦除；模块 DOM-free：
// applyDocumentTheme 内部有 typeof document 守门，导入无副作用）。
//
// 锁定三条主人拍板纪律：
//   ① 默认主题 = heritage-void（静态星空山脉图），黑洞实时场景降级为可选；
//   ② ?theme= URL 覆写纪律不破（query 优先于 config）；
//   ③ 静态背景主题判定准确（场景层隐藏 + 渲染循环暂停的依据）；
//   ④ 空心主题不得出现在目录（2026-09-23 主人指示：day/ocean/forest/paper
//      无实现即删，补全列入 backlog，0 装是底线）。
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
// 1. 默认回落 = heritage-void（§8 增补②③：静态图默认，黑洞场景降为可选）
// ---------------------------------------------------------------------------
{
  assert.equal(resolveTheme(undefined, null), 'heritage-void', '无配置无 query 必须回落 heritage-void');
  assert.equal(resolveTheme('night', null), 'night', 'config 合法主题原样保留');
  assert.equal(resolveTheme('essence', null), 'essence');
  // @ts-expect-error 故意传非法值
  assert.equal(resolveTheme('dark', null), 'heritage-void', '非法 config 主题回落 heritage-void');
}

// ---------------------------------------------------------------------------
// 2. ?theme= URL 覆写纪律（开发/截图链路依赖）：query 合法时压过 config
// ---------------------------------------------------------------------------
{
  assert.equal(resolveTheme('heritage-void', 'night'), 'night', 'query 优先于 config');
  assert.equal(resolveTheme('night', 'essence'), 'essence');
  assert.equal(resolveTheme(undefined, 'heritage-void'), 'heritage-void');
  assert.equal(resolveTheme('night', 'bogus'), 'night', '非法 query 不覆写，回落 config');
  assert.equal(resolveTheme(undefined, 'bogus'), 'heritage-void', '全非法时回落默认');
}

// ---------------------------------------------------------------------------
// 3. 静态背景主题判定：heritage-void/essence 隐藏场景并暂停渲染；night 保留实时场景
// ---------------------------------------------------------------------------
{
  assert.equal(isStaticBgTheme('heritage-void'), true);
  assert.equal(isStaticBgTheme('essence'), true);
  assert.equal(isStaticBgTheme('night'), false, 'night = 黑洞实时场景（可选主题），不暂停');
  // 2026-09-23：空心主题已删（主人指示），出现即回落默认，目录里不得有
  for (const hollow of ['day', 'ocean', 'forest', 'paper']) {
    assert.equal(VALID_THEMES.includes(hollow), false, `空心主题 ${hollow} 不得留在 VALID_THEMES`);
    assert.equal(resolveTheme(hollow), 'heritage-void', `config 里的 ${hollow} 必须回落默认`);
    assert.ok(!THEME_CATALOG.find((t) => t.id === hollow), `空心主题 ${hollow} 不得留在 THEME_CATALOG`);
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
  assert.equal(THEME_CATALOG[0].id, 'heritage-void', '目录首项应为默认主题');
  assert.equal(themeLabel('heritage-void'), '遗产星空');
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
