// idle-behavior.ts — 待机微动作与作息节律（衣服的质感，不是状态信号）
//
// 纪律（character-mapping.ts 同源注记）：本模块的动作是**形象自身的生理质感**
// ——微摆/微倾/呼吸游移/偶发伸展/入定——不携带任何 presence 语义、不进入
// 显影分级；幅度封顶在"活着"的范围内（≤0.03 弧度当量）。"不发明高频动画源"
// 纪律管的是状态信号的显影，不管衣服质感；但质感也不许冒充信号（幅度上限
// 就是防线：大动作必须来自真实姿态帧）。
//
// 作息节律**从真实姿态时长推导**：dreaming_consolidation 持续越久入定越深
// （doze ↑），不发明"他去睡觉了"的假事件——他睡没睡是 presence 说了算，
// 我们只把"睡深了"演得更像。
//
// 零运行时依赖，确定性纯函数（禁 Math.random），Node 可测
// （tests/pet-idle-behavior.mjs）。

export interface IdleMotion {
  /** 躯干微摆（弧度，|值| ≤ swayAmplitude） */
  swayRad: number;
  /** 头部微倾（弧度，|值| ≤ headTiltAmplitude） */
  headTiltRad: number;
  /** 呼吸增益 ∈ [0.8, 1.2]（慢游移，让呼吸不死板） */
  breathGain: number;
  /** 伸展瞬时 ∈ [0,1]（确定性偶发窗口，非随机） */
  stretch: number;
  /** 入定档 ∈ [0,1]（0=活跃，1=深定；随 dreaming 持续时长爬坡） */
  doze: number;
}

export const IDLE = {
  /** 躯干微摆：周期/幅度（弧度） */
  swayPeriodSec: 7.3,
  swayAmplitude: 0.018,
  /** 头部微倾 */
  headTiltPeriodSec: 11.7,
  headTiltAmplitude: 0.026,
  /** 呼吸游移 */
  breathWanderPeriodSec: 13.1,
  breathWanderGain: 0.15,
  /** 偶发伸展：周期窗口（秒）与窗内时长 */
  stretchPeriodSec: 96,
  stretchDurationSec: 3.2,
  /** 入定爬坡：dreaming 持续多少秒到满档 */
  dozeRampSec: 300,
} as const;

/** 确定性相位：同 seed 同相位（复用 character-mapping 的正弦折叠式）。 */
function phaseOf(seed: number, salt: number): number {
  const x = Math.sin(seed * 12.9898 + salt * 78.233) * 43758.5453;
  return (x - Math.floor(x)) * Math.PI * 2;
}

/**
 * 待机微动作 + 入定档（纯函数）。
 * @param clockSec  形象层动画时钟（秒，单调）
 * @param seed      会话级确定性种子
 * @param dreamHoldSec  当前 dreaming 姿态已持续秒数（调用方按真实姿态时长累计）
 */
export function deriveIdleMotion(
  clockSec: number,
  seed: number,
  dreamHoldSec: number,
): IdleMotion {
  const sway =
    Math.sin((clockSec / IDLE.swayPeriodSec) * Math.PI * 2 + phaseOf(seed, 1)) *
    IDLE.swayAmplitude;
  const headTilt =
    Math.sin((clockSec / IDLE.headTiltPeriodSec) * Math.PI * 2 + phaseOf(seed, 2)) *
    IDLE.headTiltAmplitude;
  const breathGain =
    1 +
    IDLE.breathWanderGain *
      Math.sin((clockSec / IDLE.breathWanderPeriodSec) * Math.PI * 2 + phaseOf(seed, 3));

  // 伸展：固定周期窗 + 相位错开；窗内三角波（非随机，可复现）
  const stretchPhase = ((clockSec + (phaseOf(seed, 4) / (Math.PI * 2)) * IDLE.stretchPeriodSec) % IDLE.stretchPeriodSec);
  const stretch =
    stretchPhase < IDLE.stretchDurationSec
      ? 1 - Math.abs((stretchPhase / IDLE.stretchDurationSec) * 2 - 1)
      : 0;

  const doze = Math.min(1, Math.max(0, dreamHoldSec / IDLE.dozeRampSec));

  return {
    swayRad: sway,
    headTiltRad: headTilt,
    breathGain: Math.min(1.2, Math.max(0.8, breathGain)),
    stretch,
    doze,
  };
}
