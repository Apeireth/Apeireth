// 桌宠常驻投影 — 形象层注册表（双形态可插拔的插口实现）
//
// 契约见 pet-contract.ts。这里只做三件事：
//   ① 内置形象层注册（光态哨兵 / 微光简模）；
//   ② 主题包形象清单的校验与注册（parseThemePackManifest）；
//   ③ 按 id 解析形象层（解析不到回落微光简模，绝不让桌宠空窗）。
//
// 渲染器本体是 Svelte 组件（模块形状 `{default: Component}`），注册表只存
// 「懒装载函数」，不 import 任何组件 —— 本文件零运行时依赖（不 import
// svelte / DOM），Node 可直接 import 测试（tests/pet-avatar-registry.mjs）。
//
// 主题包形象层装载口（plug point）：
//   主题包目录携带 avatar-manifest.json（形状见 parseThemePackManifest），
//   校验通过后调用 registerThemePackAvatar 注册；装载器返回的模块形状与
//   内置层完全一致（{default: 组件}，组件吃 PetFrame 契约）——换形象不换契约。
//   真实动态 import 只发生在 Svelte 侧的 AvatarHost（本文件不碰网络/文件）。

import {
  DEFAULT_AVATAR_LAYER_ID,
  FALLBACK_AVATAR_LAYER_ID,
  type AvatarLayerDescriptor,
} from './pet-contract.ts';

/** 形象层模块形状：默认导出一个消费 PetFrame 的组件。 */
export interface AvatarLayerModule {
  default: unknown;
}

/** 懒装载器：Svelte 侧解析为组件；测试里可解析为哑对象。 */
export type AvatarLayerLoader = () => Promise<AvatarLayerModule>;

interface RegisteredAvatarLayer {
  descriptor: AvatarLayerDescriptor;
  load: AvatarLayerLoader;
}

const registry = new Map<string, RegisteredAvatarLayer>();

/** 注册一个形象层（同 id 覆盖 = 热换形象，契约不动）。 */
export function registerAvatarLayer(
  descriptor: AvatarLayerDescriptor,
  load: AvatarLayerLoader,
): void {
  registry.set(descriptor.id, {descriptor, load});
}

/** 按 id 解析；不存在返回 null（调用方决定回落，不猜）。 */
export function resolveAvatarLayer(id: string): RegisteredAvatarLayer | null {
  return registry.get(id) ?? null;
}

/** 全部已注册形象层声明（设置面/右键菜单列选项用）。 */
export function listAvatarLayers(): AvatarLayerDescriptor[] {
  return [...registry.values()].map((entry) => entry.descriptor);
}

/** 形象层解析的最终回退：认不出的 id → 微光简模（桌宠绝不空窗）。 */
export function resolveAvatarLayerOrFallback(id: string): RegisteredAvatarLayer {
  return (
    resolveAvatarLayer(id) ??
    resolveAvatarLayer(FALLBACK_AVATAR_LAYER_ID) ??
    // 微光简模必已内置注册（registerBuiltInAvatarLayers）；真到这一步说明
    // 宿主忘了注册，宁可显式抛错也不静默画空气。
    (() => {
      throw new Error('avatar registry empty: registerBuiltInAvatarLayers not called');
    })()
  );
}

// ============================================================
// 内置形象层
// ============================================================

/**
 * 内置形象层注册（Svelte 宿主启动时调用一次，传入各内置组件的装载器）。
 * 光态哨兵是默认形象层（00-PHILOSOPHY §5 微光在场）；微光简模是 T0 廉价档
 * 与渲染失败回落；角色形象层（开放 3D 人形格式渲染，可选装载器）是"华服"——
 * 三个实现共用同一 PetFrame 契约，插口的活证据。
 */
export function registerBuiltInAvatarLayers(loaders: {
  lightSentinel: AvatarLayerLoader;
  emberMinimal: AvatarLayerLoader;
  /** 角色形象层（tier0Safe=false：T0 梯队不装载，老机器不付这笔钱） */
  characterForm?: AvatarLayerLoader;
}): void {
  registerAvatarLayer(
    {
      id: DEFAULT_AVATAR_LAYER_ID,
      displayName: '光态哨兵',
      kind: 'builtin',
      tier0Safe: true,
    },
    loaders.lightSentinel,
  );
  registerAvatarLayer(
    {
      id: FALLBACK_AVATAR_LAYER_ID,
      displayName: '微光简模',
      kind: 'builtin',
      tier0Safe: true,
    },
    loaders.emberMinimal,
  );
  if (loaders.characterForm) {
    registerAvatarLayer(
      {
        id: CHARACTER_AVATAR_LAYER_ID,
        displayName: '角色形象',
        kind: 'builtin',
        tier0Safe: false,
      },
      loaders.characterForm,
    );
  }
}

// ============================================================
// 主题包形象清单（theme-pack 形象层的校验口）
// ============================================================

/** 主题包形象清单（avatar-manifest.json）的合法形状。 */
export interface ThemePackAvatarManifest {
  /** 清单版本（前向兼容；未知大版本拒绝） */
  schemaVersion: number;
  /** 形象层 id（不与内置 id 冲突——内置 id 保留字） */
  id: string;
  displayName: string;
  /** 形象层模块相对路径（相对清单所在主题包目录，禁止绝对路径/上级逃逸） */
  module: string;
}

const BUILTIN_RESERVED_IDS: ReadonlySet<string> = new Set([
  DEFAULT_AVATAR_LAYER_ID,
  FALLBACK_AVATAR_LAYER_ID,
]);

/**
 * 校验主题包形象清单。严格但不脆弱：字段缺一不可，未知字段容忍（前向兼容）；
 * id 撞内置保留字 / module 路径逃逸 / schemaVersion 不认识 → null（不猜不修）。
 */
export function parseThemePackManifest(json: string): ThemePackAvatarManifest | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch {
    return null;
  }
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) return null;
  const m = parsed as Record<string, unknown>;
  const schemaVersion = typeof m.schemaVersion === 'number' ? m.schemaVersion : null;
  if (schemaVersion !== 1) return null;
  const id = typeof m.id === 'string' ? m.id.trim() : '';
  const displayName = typeof m.displayName === 'string' ? m.displayName.trim() : '';
  const module = typeof m.module === 'string' ? m.module.trim() : '';
  if (!id || !displayName || !module) return null;
  if (BUILTIN_RESERVED_IDS.has(id)) return null;
  if (!isSafeRelativeModulePath(module)) return null;
  return {schemaVersion, id, displayName, module};
}

/** 相对模块路径安全检查：非绝对、无上级逃逸、必须是 .js/.mjs。 */
function isSafeRelativeModulePath(path: string): boolean {
  if (path.startsWith('/') || path.startsWith('\\')) return false;
  if (/^[a-zA-Z]:/.test(path)) return false;
  if (path.split(/[\\/]/).includes('..')) return false;
  return /\.(js|mjs)$/.test(path);
}

/**
 * 注册主题包形象层（清单校验 + 装载器注入）。装载器由 Svelte 宿主提供
 * （动态 import 主题包模块），本函数只负责登记，与内置层走同一张表。
 */
export function registerThemePackAvatar(
  manifest: ThemePackAvatarManifest,
  load: AvatarLayerLoader,
): void {
  registerAvatarLayer(
    {
      id: manifest.id,
      displayName: manifest.displayName,
      kind: 'theme-pack',
      tier0Safe: false,
    },
    load,
  );
}

// ============================================================
// 形象包清单 v2 —— 许可红线进代码（avatar-pack-admission.md §3/§6）
//
// v2 在 v1 之上要求携带许可声明与能力声明。许可校验是**红线**：
// spdx 不在白名单 / 署名档缺署名文案 → 一律拒绝注册（不猜不修）。
// 渠道纪律（准入规范 §1b）：用户自带形象由用户自担许可责任，本校验只管
// 平台侧注册（内置/官方/测试渠道）；用户渠道另走格式合法性校验口。
// ============================================================

/** 许可档：public-domain 无义务 / attribution 署名义务（准入规范 §3 白名单）。 */
export type AvatarLicenseTier = 'public-domain' | 'attribution';

/** 白名单 SPDX 表（准入规范 §3 拍板口径）；表外一律拒绝。 */
export const AVATAR_LICENSE_WHITELIST: Readonly<Record<string, AvatarLicenseTier>> = {
  'CC0-1.0': 'public-domain',
  Unlicense: 'public-domain',
  MIT: 'attribution',
  'Apache-2.0': 'attribution',
  'BSD-2-Clause': 'attribution',
  'BSD-3-Clause': 'attribution',
  'CC-BY-4.0': 'attribution',
};

export interface AvatarPackLicense {
  /** SPDX 标识，必须在白名单内 */
  spdx: string;
  /** 署名文案（attribution 档必填非空；public-domain 档可为空串） */
  attribution: string;
  /** 原始发布页 URL（http/https） */
  source: string;
  /** 修改记录（无修改写「无」，不许留空） */
  modifications: string;
}

export interface AvatarPackCapabilities {
  /** 姿态 id 必须是契约四值内（PET_POSTURE_BY_STANCE 的值域） */
  postures: string[];
  expressions: string[];
  /** 口型音素/开合档位 */
  visemes: string[];
  gaze: boolean;
  blink: boolean;
}

export interface AvatarPackManifestV2 {
  schemaVersion: 2;
  id: string;
  displayName: string;
  module: string;
  license: AvatarPackLicense;
  capabilities: AvatarPackCapabilities;
  /** 形象包一律 false（T0 梯队只走内置光态/简模） */
  tier0Safe: false;
}

/** 契约姿态值域（v2 姿态声明只许在此之内，防清单与契约漂移）。 */
const KNOWN_POSTURES: ReadonlySet<string> = new Set([
  'corner_quiet',
  'head_up',
  'curled_asleep',
  'leaning_close',
]);

function isHttpUrl(value: string): boolean {
  return /^https?:\/\/\S+$/.test(value);
}

/**
 * 校验形象包清单 v2。严格但不脆弱：未知附加字段容忍（前向兼容），
 * 主键/许可/能力缺一不可；任何红线越界（白名单外许可、署名缺失、
 * 路径逃逸、保留字 id、未知姿态）→ null。
 */
export function parseAvatarPackManifestV2(json: string): AvatarPackManifestV2 | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch {
    return null;
  }
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) return null;
  const m = parsed as Record<string, unknown>;
  if (m.schemaVersion !== 2) return null;
  const id = typeof m.id === 'string' ? m.id.trim() : '';
  const displayName = typeof m.displayName === 'string' ? m.displayName.trim() : '';
  const module = typeof m.module === 'string' ? m.module.trim() : '';
  if (!id || !displayName || !module) return null;
  if (BUILTIN_RESERVED_IDS.has(id)) return null;
  if (!isSafeRelativeModulePath(module)) return null;

  // —— 许可红线 ——
  const lic = m.license as Record<string, unknown> | null;
  if (lic === null || typeof lic !== 'object' || Array.isArray(lic)) return null;
  const spdx = typeof lic.spdx === 'string' ? lic.spdx.trim() : '';
  const tier = AVATAR_LICENSE_WHITELIST[spdx];
  if (!tier) return null; // 白名单外一律拒绝（NC/ND/禁二销/不明全在这条线上死）
  const attribution = typeof lic.attribution === 'string' ? lic.attribution.trim() : '';
  if (tier === 'attribution' && !attribution) return null; // 署名档缺署名 = 拒
  const source = typeof lic.source === 'string' ? lic.source.trim() : '';
  if (!isHttpUrl(source)) return null; // 无原始来源不收（准入流程第 2 步的字面要求）
  const modifications = typeof lic.modifications === 'string' ? lic.modifications.trim() : '';
  if (!modifications) return null;

  // —— 能力声明 ——
  const caps = m.capabilities as Record<string, unknown> | null;
  if (caps === null || typeof caps !== 'object' || Array.isArray(caps)) return null;
  const postures = caps.postures;
  if (!Array.isArray(postures) || postures.length === 0) return null;
  for (const posture of postures) {
    if (typeof posture !== 'string' || !KNOWN_POSTURES.has(posture)) return null;
  }
  const expressions = caps.expressions;
  const visemes = caps.visemes;
  if (!Array.isArray(expressions) || !Array.isArray(visemes)) return null;
  if (typeof caps.gaze !== 'boolean' || typeof caps.blink !== 'boolean') return null;
  if (m.tier0Safe !== false) return null;

  return {
    schemaVersion: 2,
    id,
    displayName,
    module,
    license: {spdx, attribution, source, modifications},
    capabilities: {
      postures: postures as string[],
      expressions: expressions as string[],
      visemes: visemes as string[],
      gaze: caps.gaze,
      blink: caps.blink,
    },
    tier0Safe: false,
  };
}

/**
 * 注册 v2 形象包（许可红线已过）。许可声明随注册留存于清单对象，
 * 准入台账（ATTRIBUTION/NOTICE）以它为唯一事实源，不另抄一份。
 */
export function registerAvatarPackV2(
  manifest: AvatarPackManifestV2,
  load: AvatarLayerLoader,
): void {
  registerAvatarLayer(
    {
      id: manifest.id,
      displayName: manifest.displayName,
      kind: 'theme-pack',
      tier0Safe: false,
    },
    load,
  );
}
