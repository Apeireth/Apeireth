import type {Accent, Theme} from './types';

export const VALID_THEMES: Theme[] = ['origin', 'noir', 'heritage-void', 'essence', 'night', 'day', 'ocean', 'forest', 'paper', 'starship'];

/** 静态背景主题（无 WebGL 场景层）：场景层隐藏并暂停渲染循环。 */
export const STATIC_BG_THEMES: readonly Theme[] = ['origin', 'noir', 'heritage-void', 'essence', 'day', 'ocean', 'forest', 'paper', 'starship'];

export type ThemeOption = {
  id: Theme;
  label: string;
  desc: string;
  /** CSS background for preview swatch */
  swatch: string;
};

export const THEME_CATALOG: ThemeOption[] = [
  {
    id: 'origin',
    label: '原初',
    // 2026-10-03 主人拍板：新增"最初的主题"并设为默认（DeepSeek 原生界面风，
    // 白底/品牌蓝/零动画纯色层；不改组件结构、功能按钮与配置）。
    desc: '默认 · DeepSeek 原生界面风 · 白底蓝标 · 零动画简洁高效',
    swatch:
      'radial-gradient(circle at 78% 22%, rgba(77, 107, 254, 0.9) 0%, rgba(77, 107, 254, 0) 34%), linear-gradient(180deg, #ffffff 0%, #f2f3f5 100%)',
  },
  {
    id: 'noir',
    label: '纯黑',
    // 2026-10-03 主人拍板「再加一个黑色的」：真黑 AMOLED 档，零装饰零动画。
    desc: '纯黑底 · 零装饰 · 高对比简洁',
    swatch: 'radial-gradient(ellipse 120% 90% at 50% -10%, rgba(255, 255, 255, 0.1) 0%, rgba(255, 255, 255, 0) 55%), #000000',
  },
  {
    id: 'heritage-void',
    label: '遗产星空',
    desc: '静态星空山脉（旧前端遗图，主人供）',
    swatch: 'url(/assets/themes/heritage-void-bg.png) center/cover',
  },
  {
    id: 'essence',
    label: 'Essence',
    desc: '星空山脉 · 浅色雾面',
    // 渐变层必须半透明——曾用不透明渐变把底图完全盖住，卡片看起来是空白的
    swatch:
      'linear-gradient(rgba(247, 245, 241, 0.45), rgba(247, 245, 241, 0.45)), url(/assets/themes/essence-bg.png) center/cover',
  },
  {
    id: 'night',
    label: '深空舰桥',
    desc: '实时场景 · 金色存在',
    // 黑洞 + 金色吸积环的存在金预览（该主题实景即含金，预览合法）
    swatch:
      'radial-gradient(circle at 62% 55%, rgba(255, 210, 122, 0.85) 0%, rgba(255, 210, 122, 0.25) 7%, rgba(255, 210, 122, 0) 13%), radial-gradient(circle at 62% 55%, #000000 0%, #000000 10%, rgba(0, 0, 0, 0) 11%), radial-gradient(ellipse at 50% 30%, #1a1520 0%, #07070c 70%)',
  },
  {
    id: 'starship',
    label: '星舰',
    desc: '科幻 HUD 档 · 深空黑底 · 发光数据',
    // 网格 + 青蓝辉光的 HUD 预览（网格/辉光即该主题的识别特征）
    swatch:
      'radial-gradient(ellipse 90% 60% at 50% 118%, rgba(24, 84, 128, 0.5) 0%, rgba(4, 7, 13, 0) 62%), repeating-linear-gradient(0deg, rgba(87, 214, 255, 0.12) 0 1px, transparent 1px 14px), repeating-linear-gradient(90deg, rgba(87, 214, 255, 0.12) 0 1px, transparent 1px 14px), linear-gradient(180deg, #060d18 0%, #02060c 100%)',
  },
  {
    id: 'day',
    label: '日光',
    desc: '明亮纸面 · 档案调',
    // 高位暖阳——与纸面/essence 的冷白拉开区分
    swatch:
      'radial-gradient(circle at 78% 16%, rgba(255, 236, 200, 0.95) 0%, rgba(255, 236, 200, 0) 34%), linear-gradient(180deg, #f9f7f2 0%, #ede8dc 100%)',
  },
  {
    id: 'paper',
    label: '纸面',
    desc: '注册表档案 · 低饱和',
    // 注册表横线纹理——纸面调的识别特征
    swatch:
      'repeating-linear-gradient(180deg, rgba(20, 19, 22, 0.05) 0 1px, transparent 1px 9px), linear-gradient(180deg, #efede8 0%, #e2e0db 100%)',
  },
  {
    id: 'ocean',
    label: '深海',
    desc: '深舱蓝调 · 工程感',
    // 顶侧冷光柱——工程舱照明特征
    swatch:
      'linear-gradient(205deg, rgba(110, 160, 195, 0.4) 0%, rgba(110, 160, 195, 0) 42%), linear-gradient(180deg, #1d2a33 0%, #050a0f 100%)',
  },
  {
    id: 'forest',
    label: '林海',
    desc: '沉稳绿调 · 专注',
    // 林冠漏光——顶部绿晕
    swatch:
      'radial-gradient(circle at 30% 0%, rgba(127, 184, 148, 0.35) 0%, rgba(127, 184, 148, 0) 46%), linear-gradient(180deg, #1a2420 0%, #0a100e 100%)',
  },
];

export function resolveTheme(configTheme?: Theme, query?: string | null): Theme {
  if (query && VALID_THEMES.includes(query as Theme)) return query as Theme;
  if (configTheme && VALID_THEMES.includes(configTheme)) return configTheme;
  // 2026-10-03 主人拍板：默认主题 = 原初（origin，DeepSeek 原生界面风）——
  // 取代 2026-09-22 的 heritage-void 默认（该主题保留在目录，随时可选）。
  // 只影响未显式配置主题的用户；已保存的主题选择原样保留（不改现有配置）。
  return 'origin';
}

export function applyDocumentTheme(theme: Theme): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  if (theme === 'night') {
    root.removeAttribute('data-theme');
  } else {
    root.setAttribute('data-theme', theme);
  }
}

/** 该主题是否以静态图为背景（WebGL 场景层应隐藏并暂停渲染循环）。 */
export function isStaticBgTheme(theme: Theme): boolean {
  return STATIC_BG_THEMES.includes(theme);
}

export function themeLabel(theme: Theme): string {
  return THEME_CATALOG.find((t) => t.id === theme)?.label ?? theme;
}

/* ============================================================================
   UI 配色方案（规范 §8 增补⑤，2026-09-22 主人拍板）
   配色只染 UI 高亮（导航激活/工作台开关这类界面 chrome），经 --ap-accent-ui
   覆写落地；金色纪律不可破——存在金（#ffd27a 系）是他的，可选方案里没有金色系，
   isPresenceGoldFamily 是这条纪律的机器断言（tests/theme-system.mjs 锁定）。
   ============================================================================ */

export const VALID_ACCENTS: Accent[] = ['presence-gold', 'deep-space', 'sage', 'bone'];

export type AccentOption = {
  id: Accent;
  label: string;
  desc: string;
  /** accent 主色与其上文字色（设置预览真渲染与 tokens.css 的 data-accent 覆写共用同一份值） */
  accent: string;
  ink: string;
};

export const ACCENT_CATALOG: AccentOption[] = [
  {
    id: 'presence-gold',
    label: '存在金（默认）',
    desc: 'UI 高亮与他同色——金色纪律的默认态',
    accent: '#ffd27a',
    ink: '#1b1409',
  },
  {
    id: 'deep-space',
    label: '深空蓝',
    desc: '舷窗远星色调 · 克制中性',
    accent: '#7d9cc0',
    ink: '#0b0d12',
  },
  {
    id: 'sage',
    label: '雾原绿',
    desc: '清晨苔原色调 · 安静收敛',
    accent: '#7fb894',
    ink: '#0b0d12',
  },
  {
    id: 'bone',
    label: '骨白',
    desc: '无彩色档案调 · 几乎隐形',
    accent: '#e6e2da',
    ink: '#1c1b1f',
  },
];

export function resolveAccent(configAccent?: Accent): Accent {
  return configAccent && VALID_ACCENTS.includes(configAccent) ? configAccent : 'presence-gold';
}

export function applyDocumentAccent(accent: Accent): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  if (accent === 'presence-gold') {
    root.removeAttribute('data-accent');
  } else {
    root.setAttribute('data-accent', accent);
  }
}

/** 金色纪律守卫（纯函数）：存在金家族 = 色相 32°–52° 且高饱和高亮
 *  （#ffd27a / #e8a33d / #fff2d1 系都在此区间）。非默认配色必须返回 false。 */
export function isPresenceGoldFamily(hex: string): boolean {
  const m = /^#([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return false;
  const n = parseInt(m[1], 16);
  const r = ((n >> 16) & 255) / 255;
  const g = ((n >> 8) & 255) / 255;
  const b = (n & 255) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  const d = max - min;
  const s = d === 0 ? 0 : d / (1 - Math.abs(2 * l - 1));
  let h = 0;
  if (d !== 0) {
    if (max === r) h = 60 * (((g - b) / d) % 6);
    else if (max === g) h = 60 * ((b - r) / d + 2);
    else h = 60 * ((r - g) / d + 4);
  }
  if (h < 0) h += 360;
  return h >= 32 && h <= 52 && s > 0.35 && l > 0.45;
}
