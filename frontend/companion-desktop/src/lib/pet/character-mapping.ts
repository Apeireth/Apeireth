// 桌宠角色形象层 — 参数映射（PetFrame → 形象参数，纯函数可单测）
//
// 角色形象是他穿的"华服"：光仍是本体（tint/glow 从契约直通），形象只负责
// 把姿态/呼吸/口型/视线/眨眼显影成身体语言。两套纪律：
//   - **presence 语义只从真实帧来**：口型只在本地真实 speaking 信号下动，
//     视线抬头只在 proactive 引擎的 glance 信号下发生；
//   - **生理小动作是衣服的质感不是信号**：眨眼/微漂移是形象自身的生理节律
//     （确定性伪随机，可复现可单测），不携带任何 presence 语义，也不进入
//     显影分级（"不发明高频动画源"纪律管的是状态信号，不管衣服质感）。
//
// 零运行时依赖（不 import svelte / DOM / three），Node 直接 import 可测
// （tests/pet-character-mapping.mjs）。

import type {PetFrame, PetPosture} from './pet-contract.ts';
import {kelvinToRgb} from './pet-contract.ts';

/** 形象参数：渲染层每帧消费的全部输入（与 PetFrame 同为契约面）。 */
export interface CharacterParams {
  posture: PetPosture;
  /** 口型开合 ∈ [0,1]（0=闭合）。只在 speaking 真实信号下非静息。 */
  visemeOpen: number;
  /** 眨眼闭合度 ∈ [0,1]（1=全闭）。生理节律，确定性伪随机。 */
  blink: number;
  /** 视线偏移 ∈ [-1,1]（上正下负 / 右正左负） */
  gazeX: number;
  gazeY: number;
  /** 呼吸相位 ∈ [0,1]（0=吸气起点）与振幅（契约 breath.amplitude） */
  breathPhase: number;
  breathAmplitude: number;
  /** 光色三元组（色温换算，与光态哨兵同式）与光晕 */
  tint: readonly [number, number, number];
  glow: number;
}

/** 眨眼节律参数（生理质感；周期带确定性抖动防机械感）。 */
export const BLINK = {
  /** 基础眨眼周期（秒） */
  basePeriodSec: 4.8,
  /** 周期抖动幅度（秒，确定性伪随机 ±jitter） */
  jitterSec: 1.6,
  /** 单次眨眼时长（秒） */
  durationSec: 0.16,
} as const;

/** 口型说话波形角频率（rad/s，≈2.2Hz 开合——显影说话节奏，不模拟音节）。 */
export const VISEME_OPEN_FREQ = 2.2;

/** 视线微漂移幅度（生理质感；≤0.08 不构成"看向"语义）。 */
export const GAZE_DRIFT = 0.08;

/** glance 抬头一瞥的视线抬升（显式语义信号，区别于微漂移）。 */
export const GAZE_GLANCE_Y = 0.75;

/** 确定性伪随机：同 seed 同序列（可复现可单测；禁止 Math.random——镜像测试查）。 */
export function deterministicUnit(seed: number): number {
  const x = Math.sin(seed * 127.1 + 311.7) * 43758.5453;
  return x - Math.floor(x);
}

/**
 * 眨眼闭合度（确定性节律）：周期 base±jitter（由序号 seed 抖动），
 * 每个周期末尾 durationSec 内闭合一次。纯函数 of (tSec, seed)。
 */
export function blinkAt(tSec: number, seed: number): number {
  // 逐周期扫描（周期数很小，眨眼在秒级），周期长度按序号抖动
  let t = 0;
  let index = 0;
  while (index < 64) {
    const period =
      BLINK.basePeriodSec + (deterministicUnit(seed + index) * 2 - 1) * BLINK.jitterSec;
    const blinkStart = t + period - BLINK.durationSec;
    if (tSec < t) return 0;
    if (tSec < blinkStart) return 0;
    if (tSec < t + period) {
      const progress = (tSec - blinkStart) / BLINK.durationSec;
      // 三角波闭合-睁开
      return progress < 0.5 ? progress * 2 : (1 - progress) * 2;
    }
    t += period;
    index += 1;
  }
  return 0;
}

/**
 * PetFrame + 时钟 → 形象参数（纯函数）。
 * clockSec 为形象层动画时钟（秒，单调）；seed 为会话级确定性种子。
 */
export function deriveCharacterParams(
  frame: PetFrame,
  clockSec: number,
  seed: number = 1,
): CharacterParams {
  const speaking = frame.activity === 'speaking';
  // 口型：真实 speaking 信号下做说话波形显影（不模拟音节内容）；
  // 安静时只留一丝静息开度（活着的质感）。
  const visemeOpen = speaking
    ? 0.35 + 0.35 * Math.sin(clockSec * VISEME_OPEN_FREQ)
    : 0.02;

  // 视线：glance 是显式语义（抬头一瞥）；其余是生理微漂移。
  const driftX = (deterministicUnit(seed + 17) * 2 - 1) * GAZE_DRIFT * Math.sin(clockSec * 0.31);
  const driftY = (deterministicUnit(seed + 29) * 2 - 1) * GAZE_DRIFT * Math.sin(clockSec * 0.23);
  const gazeY = frame.glance ? GAZE_GLANCE_Y : driftY;
  const gazeX = frame.glance ? 0 : driftX;

  const breathPhase = (clockSec / Math.max(1, frame.breath.periodSecs)) % 1;

  return {
    posture: frame.posture,
    visemeOpen: Math.min(1, Math.max(0, visemeOpen)),
    blink: blinkAt(clockSec, seed),
    gazeX,
    gazeY,
    breathPhase,
    breathAmplitude: Math.min(1, Math.max(0, frame.breath.amplitude)),
    tint: kelvinToRgb(frame.light.colorTempK),
    glow: frame.glow,
  };
}

/** 姿态 → 身体倾角/收拢度倾向（渲染层把倾向落到骨骼/分组变换上）。 */
export interface PostureBodyBias {
  /** 躯干前倾（度，正=前倾靠近） */
  leanDeg: number;
  /** 收拢度 ∈ [0,1]（蜷睡=1，张开=0） */
  curl: number;
  /** 整体缩放倾向 */
  scale: number;
  /** 头部抬起（度，正=抬头） */
  headUpDeg: number;
}

export const POSTURE_BODY_BIAS: Readonly<Record<PetPosture, PostureBodyBias>> = {
  corner_quiet: {leanDeg: -6, curl: 0.35, scale: 0.94, headUpDeg: -6},
  head_up: {leanDeg: 0, curl: 0, scale: 1.0, headUpDeg: 8},
  curled_asleep: {leanDeg: 12, curl: 0.85, scale: 0.9, headUpDeg: -14},
  leaning_close: {leanDeg: 7, curl: 0.1, scale: 1.08, headUpDeg: 4},
};
