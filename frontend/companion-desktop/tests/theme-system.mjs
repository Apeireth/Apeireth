// 主题系统纯逻辑单测（规范 §8 + 2026-09-22 增补段）
// 直接 import 真实实现 ../src/lib/theme.ts（Node ≥23.6 类型擦除；模块 DOM-free：
// applyDocumentTheme 内部有 typeof document 守门，导入无副作用）。
//
// 锁定三条主人拍板纪律：
//   ① 默认主题 = heritage-void（静态星空山脉图），黑洞实时场景降级为可选；
//   ② ?theme= URL 覆写纪律不破（query 优先于 config）；
//   ③ 静态背景主题判定准确（场景层隐藏 + 渲染循环暂停的依据）。
// 不依赖后端进程。
import assert from 'node:assert/strict';

import {
  VALID_THEMES,
  THEME_CATALOG,
  resolveTheme,
  isStaticBgTheme,
  themeLabel,
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
  assert.equal(isStaticBgTheme('day'), false);
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

console.log('Theme system check passed.');
