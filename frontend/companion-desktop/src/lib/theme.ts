import type {Theme} from './types';

export const VALID_THEMES: Theme[] = ['heritage-void', 'night', 'day', 'ocean', 'forest', 'paper', 'essence'];

/** 静态背景主题（无 WebGL 场景层）：场景层隐藏并暂停渲染循环。 */
export const STATIC_BG_THEMES: readonly Theme[] = ['heritage-void', 'essence'];

export type ThemeOption = {
  id: Theme;
  label: string;
  desc: string;
  /** CSS background for preview swatch */
  swatch: string;
};

export const THEME_CATALOG: ThemeOption[] = [
  {
    id: 'heritage-void',
    label: '遗产星空',
    desc: '默认 · 静态星空山脉（旧前端遗图，主人供）',
    swatch: 'url(/assets/themes/heritage-void-bg.png) center/cover',
  },
  {
    id: 'essence',
    label: 'Essence',
    desc: '星空山脉 · 浅色雾面',
    swatch:
      'linear-gradient(160deg, #f7f5f1 0%, #e8e4dc 45%, #c9d4e8 100%), url(/assets/themes/essence-bg.png) center/cover',
  },
  {
    id: 'night',
    label: '深空舰桥',
    desc: '实时场景 · 金色存在',
    swatch: 'radial-gradient(ellipse at 50% 30%, #1a1520 0%, #07070c 70%)',
  },
  {
    id: 'day',
    label: '日光',
    desc: '明亮纸面 · 档案调',
    swatch: 'linear-gradient(180deg, #f7f6f3 0%, #e8e7e4 100%)',
  },
  {
    id: 'paper',
    label: '纸面',
    desc: '注册表档案 · 低饱和',
    swatch: 'linear-gradient(180deg, #eceae5 0%, #e2e1df 100%)',
  },
  {
    id: 'ocean',
    label: '深海',
    desc: '深舱蓝调 · 工程感',
    swatch: 'linear-gradient(180deg, #1d262c 0%, #050a0f 100%)',
  },
  {
    id: 'forest',
    label: '林海',
    desc: '沉稳绿调 · 专注',
    swatch: 'linear-gradient(180deg, #1a2420 0%, #0a100e 100%)',
  },
];

export function resolveTheme(configTheme?: Theme, query?: string | null): Theme {
  if (query && VALID_THEMES.includes(query as Theme)) return query as Theme;
  if (configTheme && VALID_THEMES.includes(configTheme)) return configTheme;
  // 2026-09-22 主人拍板（规范 §8 增补）：默认背景 = 静态星空山脉图，
  // 黑洞实时场景降级为可选主题（night）。
  return 'heritage-void';
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
