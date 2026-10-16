// 待机微动作与作息节律（idle-behavior.ts，真实模块）——衣服的质感纪律。
//
// 契约要点：
//   1. 幅度封顶是防线：微摆/微倾 ≤ 标称幅度，质感不许冒充信号；
//   2. 确定性可复现（禁随机）：同 clock+seed 同值；
//   3. 呼吸增益有界 [0.8,1.2]；伸展是周期窗内三角波，窗外恒 0；
//   4. 入定档从真实 dreaming 时长爬坡（不发明假事件）。
import assert from 'node:assert/strict';
import {deriveIdleMotion, IDLE} from '../src/lib/pet/idle-behavior.ts';

console.log('--- Starting Idle Behavior Check ---');

// ---------------------------------------------------------------------------
// 1. 幅度封顶 + 确定性
// ---------------------------------------------------------------------------
{
  assert.equal(deriveIdleMotion(3.21, 7, 0).swayRad, deriveIdleMotion(3.21, 7, 0).swayRad, '确定性');
  assert.notEqual(deriveIdleMotion(3.21, 7, 0).swayRad, deriveIdleMotion(3.21, 8, 0).swayRad, 'seed 相位不同');
  let maxSway = 0;
  let maxTilt = 0;
  for (let t = 0; t < 120; t += 0.13) {
    const m = deriveIdleMotion(t, 7, 0);
    maxSway = Math.max(maxSway, Math.abs(m.swayRad));
    maxTilt = Math.max(maxTilt, Math.abs(m.headTiltRad));
    assert.ok(Math.abs(m.swayRad) <= IDLE.swayAmplitude + 1e-9, '微摆越界');
    assert.ok(Math.abs(m.headTiltRad) <= IDLE.headTiltAmplitude + 1e-9, '微倾越界');
  }
  assert.ok(maxSway > 0.005 && maxTilt > 0.005, '质感要真动（幅度非退化）');
  console.log(`  -> PASS: 幅度封顶（sway≤${IDLE.swayAmplitude} tilt≤${IDLE.headTiltAmplitude}）+ 确定性`);
}

// ---------------------------------------------------------------------------
// 2. 呼吸增益有界 + 伸展周期窗
// ---------------------------------------------------------------------------
{
  for (let t = 0; t < 60; t += 0.7) {
    const m = deriveIdleMotion(t, 7, 0);
    assert.ok(m.breathGain >= 0.8 && m.breathGain <= 1.2, '呼吸增益越界');
  }
  // 伸展：一个完整周期内恰有一段连续窗（相位随 seed 偏移，不钉窗口位置，
  // 钉行为：窗内真伸、窗外恒 0、窗连续、窗宽≈时长、峰值到顶）
  const samples = [];
  for (let t = 0; t < IDLE.stretchPeriodSec; t += 0.05) {
    const s = deriveIdleMotion(t, 7, 0).stretch;
    assert.ok(s >= 0 && s <= 1, '伸展越界');
    samples.push(s);
  }
  const nz = samples.map((s, i) => [s, i]).filter(([s]) => s > 0);
  assert.ok(nz.length > 0, '伸展窗内必须真伸');
  for (let i = 1; i < nz.length; i++) {
    assert.equal(nz[i][1], nz[i - 1][1] + 1, '伸展窗必须连续（非随机闪现）');
  }
  assert.ok(
    Math.abs(nz.length * 0.05 - IDLE.stretchDurationSec) < 0.2,
    `窗宽须≈${IDLE.stretchDurationSec}s`,
  );
  assert.ok(nz.some(([s]) => s > 0.9), '窗内峰值到顶');
  console.log(`  -> PASS: 呼吸增益 [0.8,1.2] + 伸展周期窗 ${IDLE.stretchPeriodSec}s/${IDLE.stretchDurationSec}s`);
}

// ---------------------------------------------------------------------------
// 3. 入定档：真实 dreaming 时长爬坡（不发明假事件）
// ---------------------------------------------------------------------------
{
  assert.equal(deriveIdleMotion(0, 7, 0).doze, 0, '没睡就是没睡');
  assert.equal(deriveIdleMotion(0, 7, IDLE.dozeRampSec / 2).doze, 0.5, '线性爬坡');
  assert.equal(deriveIdleMotion(0, 7, IDLE.dozeRampSec).doze, 1, '满档');
  assert.equal(deriveIdleMotion(0, 7, IDLE.dozeRampSec * 9).doze, 1, '封顶不溢出');
  assert.equal(deriveIdleMotion(0, 7, -5).doze, 0, '负时长钳 0');
  console.log('  -> PASS: 入定档随真实 dreaming 时长爬坡（0→1 线性封顶）');
}

console.log('--- All Idle Behavior Checks PASSED! ---');
