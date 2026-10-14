// 桌宠形象抽象契约（pet-contract.ts，真实模块）——姿态映射 / 光参数镜像 /
// 静息回落 / 色温换算。
//
// 契约要点：
//   1. 四姿态与 presence 姿态枚举一一对应（00-PHILOSOPHY §5）；
//   2. 光参数表与 ember_hud_driver.rs::synthesize_uniforms 逐值镜像
//      （四个投影显影同一份状态）；
//   3. 无真实帧 = 静息姿态 + 静息光晕 + 基线呼吸，绝不编造状态；
//   4. glance 只透传 proactive 引擎的瞬时信号，形象层不自造动作。
import assert from 'node:assert/strict';
import {
  derivePetFrame,
  kelvinToRgb,
  PET_LIGHT_BY_STANCE,
  PET_POSTURE_BY_STANCE,
  PET_REST_POSTURE,
} from '../src/lib/pet/pet-contract.ts';

console.log('--- Starting Pet Avatar Contract Check ---');

// ---------------------------------------------------------------------------
// 1. 姿态映射：四姿态一一对应（语义见 00-PHILOSOPHY §5）
// ---------------------------------------------------------------------------
{
  assert.equal(PET_POSTURE_BY_STANCE.deep_coding_focus, 'corner_quiet');
  assert.equal(PET_POSTURE_BY_STANCE.attentive_presence, 'head_up');
  assert.equal(PET_POSTURE_BY_STANCE.dreaming_consolidation, 'curled_asleep');
  assert.equal(PET_POSTURE_BY_STANCE.empathetic_care, 'leaning_close');
  assert.equal(PET_REST_POSTURE, 'corner_quiet');
  console.log('  -> PASS: 四姿态映射与静息姿态符合契约');
}

// ---------------------------------------------------------------------------
// 2. 光参数表 = ember_hud_driver.rs 逐值镜像（两处同表，改数即改口径）
// ---------------------------------------------------------------------------
{
  const mirror = {
    deep_coding_focus: {baseIntensity: 0.02, breathAmplitude: 0.03, colorTempK: 3200},
    attentive_presence: {baseIntensity: 0.15, breathAmplitude: 0.2, colorTempK: 4500},
    dreaming_consolidation: {baseIntensity: 0.08, breathAmplitude: 0.12, colorTempK: 6000},
    empathetic_care: {baseIntensity: 0.25, breathAmplitude: 0.35, colorTempK: 2700},
  };
  for (const [stance, params] of Object.entries(mirror)) {
    assert.deepEqual(PET_LIGHT_BY_STANCE[stance], params, `光参数镜像漂移: ${stance}`);
  }
  console.log('  -> PASS: 四姿态光参数与 ember_hud_driver.rs 同表');
}

// ---------------------------------------------------------------------------
// 3. 静息回落：无真实帧 = 静息姿态 + 静息光晕 + 基线呼吸（不编造状态）
// ---------------------------------------------------------------------------
{
  const frame = derivePetFrame(
    {
      stance: null,
      mode: 'quiet',
      breath: null,
      glow: 0.42, // 无帧时这个值不许被采用
      simulated: false,
      connected: false,
    },
    {glance: false, restGlow: 0.14},
  );
  assert.equal(frame.posture, 'corner_quiet');
  assert.equal(frame.glow, 0.14);
  assert.deepEqual(frame.breath, {periodSecs: 4.0, amplitude: 0.65});
  assert.equal(frame.glance, false);
  assert.equal(frame.simulated, false);
  console.log('  -> PASS: 无帧回落静息口径（姿态/光晕/呼吸全不编造）');
}

// ---------------------------------------------------------------------------
// 4. 有帧推导：姿态/光/呼吸/glance/SIM 全部如实透传
// ---------------------------------------------------------------------------
{
  const frame = derivePetFrame(
    {
      stance: 'dreaming_consolidation',
      mode: 'thinking',
      breath: {periodSecs: 6.5, amplitude: 0.3},
      glow: 0.5,
      simulated: true,
      connected: false,
    },
    {glance: true, restGlow: 0.14},
  );
  assert.equal(frame.posture, 'curled_asleep');
  assert.equal(frame.light.colorTempK, 6000);
  assert.deepEqual(frame.breath, {periodSecs: 6.5, amplitude: 0.3});
  assert.equal(frame.glow, 0.5);
  assert.equal(frame.activity, 'thinking');
  assert.equal(frame.glance, true);
  assert.equal(frame.simulated, true);
  console.log('  -> PASS: 有帧推导如实透传（含 glance 与 SIM 标注）');
}

// ---------------------------------------------------------------------------
// 5. 色温换算：镜像 ember_hud_driver.rs::kelvin_to_rgb（三通道 ∈ [0,1]）
// ---------------------------------------------------------------------------
{
  for (const kelvin of [2700, 3200, 4500, 6000]) {
    const [r, g, b] = kelvinToRgb(kelvin);
    for (const channel of [r, g, b]) {
      assert.ok(channel >= 0 && channel <= 1, `色温 ${kelvin}K 通道越界: ${channel}`);
    }
  }
  const cool = kelvinToRgb(6000);
  const warm = kelvinToRgb(2700);
  assert.ok(
    warm[0] - warm[2] > cool[0] - cool[2],
    '暖色温红蓝差应高于冷色温（镜像式中红通道 ≤6600K 恒 1，看红蓝差）',
  );
  assert.ok(cool[2] > warm[2], '冷色温蓝通道应高于暖色温');
  console.log('  -> PASS: 色温换算保界且冷暖方向正确');
}

console.log('--- All Pet Avatar Contract Checks PASSED! ---');
