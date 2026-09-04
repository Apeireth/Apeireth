import type {Theme} from './types';

export const VALID_THEMES: Theme[] = ['night', 'day', 'ocean', 'forest', 'paper', 'essence'];

export type ThemeOption = {
  id: Theme;
  label: string;
  desc: string;
  /** CSS background for preview swatch */
  swatch: string;
};

export const THEME_CATALOG: ThemeOption[] = [
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
    desc: '默认夜景 · 金色存在',
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
  return 'essence';
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

export function themeLabel(theme: Theme): string {
  return THEME_CATALOG.find((t) => t.id === theme)?.label ?? theme;
}
