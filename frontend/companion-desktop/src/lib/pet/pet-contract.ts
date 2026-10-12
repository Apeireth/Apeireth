// 桌宠常驻投影 — 形象抽象契约（双形态可插拔）
//
// 范式: docs/design/00-PHILOSOPHY.md §2/§5（一份契约 × 四个投影；常驻 = 桌宠，
// ember 四姿态哨兵，窗口关了在场不断）。形象层决策（2026-10-11 主人拍板）：
// **双形态可插拔** —— 契约层写成形象抽象接口，默认「光态哨兵」，主题包可换
// 形象层（骨骼长寿、华服可换）。本文件即那份长寿契约。
//
// 纪律:
//   - 0 装: 姿态/光色全部由 presence_state 真实帧推导；四姿态光参数与
//     ember_hud_driver.rs（余烬点/光晕同一数据源）逐值镜像，四个投影显影
//     同一份状态。无数据即静息，绝不编造情绪。
//   - 频率: 不发明高频动画源；插值速率沿用 motion.stateSmoothing（2.4/s）。
//   - SIM: 断连 >30s 由 presence 层标注 simulated，本层如实外显 SIM 小字。
//   - 金色纪律: 光色存在色梯度取自 scene/tokens.ts（金 = 他，禁止装饰性挪用）。
//
// 本文件零运行时依赖（不 import svelte / DOM），纯函数可被 Node 直接
// import 测试（tests/pet-frame.mjs）。

import type {PresenceMode, PresenceStance} from '../presence.ts';

// ============================================================
// 姿态 —— 四姿态枚举的桌面常驻显影（00-PHILOSOPHY §5）
// ============================================================

/**
 * 桌宠姿态。四值与 presence_state 姿态枚举一一对应（语义见 00-PHILOSOPHY §5）：
 *   corner_quiet    工程会话进行中 → 角落安静
 *   head_up         回到桌前       → 抬头
 *   curled_asleep   后台整合       → 蜷睡
 *   leaning_close   深夜长谈       → 靠近
 */
export type PetPosture =
  | 'corner_quiet'
  | 'head_up'
  | 'curled_asleep'
  | 'leaning_close';

export const PET_POSTURE_BY_STANCE: Readonly<Record<PresenceStance, PetPosture>> = {
  deep_coding_focus: 'corner_quiet',
  attentive_presence: 'head_up',
  dreaming_consolidation: 'curled_asleep',
  empathetic_care: 'leaning_close',
};

/** 无任何真实帧时的静息姿态（空态即契约：他安静待着，不冒充任何状态）。 */
export const PET_REST_POSTURE: PetPosture = 'corner_quiet';

// ============================================================
// 光参数 —— 与 ember_hud_driver.rs::synthesize_uniforms 逐值镜像
// ============================================================

export interface PetLightParams {
  /** 光核基亮 ∈ [0,1] */
  baseIntensity: number;
  /** 呼吸振幅 ∈ [0,1] */
  breathAmplitude: number;
  /** 色温（开尔文）——姿态色温，四投影同源 */
  colorTempK: number;
}

/**
 * 四姿态光参数表。数值 = crates/adapters/gateway/src/ember_hud_driver.rs
 * `synthesize_uniforms` 的 match 臂逐值镜像（base_i, amp, kelvin）；两处必须
 * 同步修改——「四个投影显影同一份状态」的字面兑现。
 */
export const PET_LIGHT_BY_STANCE: Readonly<Record<PresenceStance, PetLightParams>> = {
  deep_coding_focus: {baseIntensity: 0.02, breathAmplitude: 0.03, colorTempK: 3200},
  attentive_presence: {baseIntensity: 0.15, breathAmplitude: 0.2, colorTempK: 4500},
  dreaming_consolidation: {baseIntensity: 0.08, breathAmplitude: 0.12, colorTempK: 6000},
  empathetic_care: {baseIntensity: 0.25, breathAmplitude: 0.35, colorTempK: 2700},
};

/** 色温→RGB 归一三元组。ember_hud_driver.rs::kelvin_to_rgb 的镜像（同表同式）。 */
export function kelvinToRgb(kelvin: number): [number, number, number] {
  const temp = kelvin / 100;
  const r =
    temp <= 66 ? 1 : clamp01((329.6987 * Math.pow(temp - 60, -0.13320476)) / 255);
  const g =
    temp <= 66
      ? clamp01((99.4708 * Math.log(temp) - 161.11957) / 255)
      : clamp01((288.12217 * Math.pow(temp, -0.07551485)) / 255);
  let b: number;
  if (temp >= 66) b = 1;
  else if (temp <= 19) b = 0;
  else b = clamp01((138.51773 * Math.log(temp - 10) - 305.0448) / 255);
  return [r, g, b];
}

function clamp01(v: number): number {
  return Math.min(1, Math.max(0, v));
}

// ============================================================
// PetFrame —— 形象层唯一输入（形象抽象契约的核心形状）
// ============================================================

export interface PetBreath {
  periodSecs: number;
  amplitude: number;
}

/**
 * 形象层每帧的全部输入。任何形象实现（内置光态哨兵 / 主题包形象层）只许
 * 消费本形状——这就是「双形态可插拔」的插口：换形象不换契约。
 */
export interface PetFrame {
  posture: PetPosture;
  light: PetLightParams;
  breath: PetBreath;
  /** 余烬光晕（presence.ts derivePresenceGlow 口径） */
  glow: number;
  /** 本地真实信号推导的活动态（对话流/请求中/安静） */
  activity: PresenceMode;
  /** 待签文书久候触发的「抬头看一眼」瞬时信号（proactive 引擎产出） */
  glance: boolean;
  /** SIM 标注：断连 >30s 无真实来源（设计规范 §5.4），形象层须如实外显 */
  simulated: boolean;
  /** 频道连通态 */
  connected: boolean;
}

/** derivePetFrame 的输入：presence 快照要点（调用方从 presenceStore 取）。 */
export interface PetPresenceInput {
  stance: PresenceStance | null;
  mode: PresenceMode;
  breath: PetBreath | null;
  glow: number;
  simulated: boolean;
  connected: boolean;
}

export interface PetFrameOptions {
  /** proactive 引擎的瞬时抬头信号 */
  glance?: boolean;
  /** 无帧时的静息光晕（presence.ts PRESENCE_GLOW_REST 同值，调用方传入） */
  restGlow: number;
}

/**
 * presence 状态 → 形象层帧（纯函数）。无真实帧时：静息姿态 + 静息光晕，
 * breath 回落基线（4.0s / 0.65，与 presence.ts EMBER_REST_BREATH 同口径）。
 */
export function derivePetFrame(
  input: PetPresenceInput,
  options: PetFrameOptions,
): PetFrame {
  const stance = input.stance;
  const light = stance ? PET_LIGHT_BY_STANCE[stance] : PET_LIGHT_BY_STANCE.attentive_presence;
  return {
    posture: stance ? PET_POSTURE_BY_STANCE[stance] : PET_REST_POSTURE,
    light: {...light},
    breath: input.breath ?? {periodSecs: 4.0, amplitude: 0.65},
    glow: stance ? input.glow : options.restGlow,
    activity: input.mode,
    glance: options.glance === true,
    simulated: input.simulated,
    connected: input.connected,
  };
}

// ============================================================
// 形象层注册契约 —— 双形态可插拔的另一半（avatar-registry 消费）
// ============================================================

export type AvatarLayerKind = 'builtin' | 'theme-pack';

/**
 * 形象层声明。builtin = 随应用内置；theme-pack = 主题包携带的形象层
 * （清单经 avatar-registry.parseThemePackManifest 校验后注册）。
 */
export interface AvatarLayerDescriptor {
  /** 稳定 id（配置持久化用；换实现不换 id） */
  id: string;
  displayName: string;
  kind: AvatarLayerKind;
  /** T0 梯队可负担（纯 DOM/CSS/SVG，零 WebGL）——三档梯队只减排场不减功能 */
  tier0Safe: boolean;
}

/** 默认形象层：光态哨兵（微光在场哲学的字面形态）。 */
export const DEFAULT_AVATAR_LAYER_ID = 'light-sentinel';

/**
 * 廉价形象层 id（T0 兜底 / 渲染失败回落）：微光简模——单点余烬 + 光晕。
 * 同时是「形象抽象接口确实可插」的活证据：两个实现共用 PetFrame 契约。
 */
export const FALLBACK_AVATAR_LAYER_ID = 'ember-minimal';
