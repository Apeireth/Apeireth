// 角色形象参数映射（character-mapping.ts，真实模块）——华服从契约显影身体语言。
//
// 契约要点：
//   1. 口型只跟本地真实 speaking 信号动（不模拟音节，不发明动画源）；
//   2. 抬头一瞥的视线是显式语义（proactive 引擎的 glance），微漂移是生理质感
//      （幅度 ≤0.08 不构成"看向"语义），两者不混；
//   3. 眨眼是确定性伪随机节律（可复现可单测，禁 Math.random）；
//   4. 姿态→身体倾角映射固定四值，与 00-PHILOSOPHY §5 姿态语义一致。
import assert from 'node:assert/strict';
import {
  deriveCharacterParams,
  blinkAt,
  deterministicUnit,
  POSTURE_BODY_BIAS,
  BLINK,
  GAZE_DRIFT,
  GAZE_GLANCE_Y,
} from '../src/lib/pet/character-mapping.ts';
import {derivePetFrame} from '../src/lib/pet/pet-contract.ts';

console.log('--- Starting Character Mapping Check ---');

const baseFrame = derivePetFrame(
  {
    stance: 'attentive_presence',
    mode: 'quiet',
    breath: {periodSecs: 4, amplitude: 0.5},
    glow: 0.3,
    simulated: false,
    connected: true,
  },
  {restGlow: 0.14},
);

// ---------------------------------------------------------------------------
// 1. 口型纪律：只有真实 speaking 信号才动嘴
// ---------------------------------------------------------------------------
{
  const quiet = deriveCharacterParams({...baseFrame, activity: 'quiet'}, 1.23, 7);
  assert.ok(quiet.visemeOpen < 0.1, '安静时口型近闭合（静息一丝）');

  const speaking = deriveCharacterParams({...baseFrame, activity: 'speaking'}, 1.23, 7);
  assert.ok(speaking.visemeOpen > 0.3, 'speaking 真实信号下口型显影');
  const speakingLater = deriveCharacterParams({...baseFrame, activity: 'speaking'}, 1.5, 7);
  assert.notEqual(speaking.visemeOpen, speakingLater.visemeOpen, '说话波形随时间动');
  console.log('  -> PASS: 口型只跟真实 speaking 信号');
}

// ---------------------------------------------------------------------------
// 2. 视线纪律：glance 是语义抬眼，微漂移是生理质感（幅度受限）
// ---------------------------------------------------------------------------
{
  const glance = deriveCharacterParams({...baseFrame, glance: true}, 2.0, 7);
  assert.equal(glance.gazeY, GAZE_GLANCE_Y, 'glance 抬眼到位');
  assert.equal(glance.gazeX, 0, 'glance 时不横瞟');

  let maxDrift = 0;
  for (let t = 0; t < 40; t += 0.7) {
    const calm = deriveCharacterParams({...baseFrame, glance: false}, t, 7);
    maxDrift = Math.max(maxDrift, Math.abs(calm.gazeX), Math.abs(calm.gazeY));
  }
  assert.ok(maxDrift <= GAZE_DRIFT + 1e-9, `微漂移幅度必须 ≤${GAZE_DRIFT}`);
  assert.ok(maxDrift > 0, '微漂移存在（活着的质感）');
  console.log('  -> PASS: glance 语义与生理微漂移分离');
}

// ---------------------------------------------------------------------------
// 3. 眨眼节律：确定性可复现、有界、周期在生理区间
// ---------------------------------------------------------------------------
{
  assert.equal(blinkAt(0.5, 7), blinkAt(0.5, 7), '同 seed 同时刻同值（确定性）');
  // 不同 seed 节律不同：比较首个眨眼时刻（t=0.5s 处两者都还没到眨眼点，
  // 直接比较瞬时值比了个寂寞——用例要钉行为不钉字面）
  const firstBlinkTime = (seed) => {
    for (let t = 0; t < 30; t += 0.01) {
      if (blinkAt(t, seed) > 0) return Math.round(t * 100);
    }
    return -1;
  };
  assert.notEqual(firstBlinkTime(7), firstBlinkTime(8), '不同 seed 眨眼时刻不同（非机械同步）');
  assert.ok(firstBlinkTime(7) > 0 && firstBlinkTime(8) > 0, '两个 seed 都真眨了眼');
  let maxBlink = 0;
  let blinks = 0;
  let prev = 0;
  for (let t = 0; t < 60; t += 0.01) {
    const v = blinkAt(t, 7);
    assert.ok(v >= 0 && v <= 1, '眨眼闭合度越界');
    maxBlink = Math.max(maxBlink, v);
    if (prev === 0 && v > 0) blinks += 1;
    prev = v;
  }
  assert.ok(maxBlink > 0.5, '眨眼闭合要闭到位');
  const perMinute = blinks;
  assert.ok(
    perMinute >= 4 && perMinute <= 40,
    `眨眼频率须在生理区间（实测 ${perMinute} 次/分）`,
  );
  assert.ok(BLINK.basePeriodSec > 2 && BLINK.basePeriodSec < 8);
  console.log(`  -> PASS: 眨眼确定性节律（${perMinute} 次/分，生理区间）`);
}

// ---------------------------------------------------------------------------
// 4. 确定性伪随机：有界、非退化（禁 Math.random 由镜像测试钉）
// ---------------------------------------------------------------------------
{
  for (let i = 0; i < 50; i++) {
    const v = deterministicUnit(i);
    assert.ok(v >= 0 && v < 1, '伪随机越界');
  }
  const a = new Set(Array.from({length: 20}, (_, i) => deterministicUnit(i)));
  assert.ok(a.size > 15, '伪随机不许退化成常数');
  console.log('  -> PASS: 确定性伪随机有界不退化');
}

// ---------------------------------------------------------------------------
// 5. 姿态→身体倾角：四值固定且语义方向正确（§5）
// ---------------------------------------------------------------------------
{
  assert.equal(Object.keys(POSTURE_BODY_BIAS).length, 4);
  assert.ok(POSTURE_BODY_BIAS.leaning_close.leanDeg > 0, '靠近=前倾');
  assert.ok(POSTURE_BODY_BIAS.head_up.headUpDeg > 0, '抬头=头抬');
  assert.ok(POSTURE_BODY_BIAS.curled_asleep.curl > 0.8, '蜷睡=收拢');
  assert.ok(POSTURE_BODY_BIAS.curled_asleep.headUpDeg < 0, '蜷睡=垂头');
  assert.ok(POSTURE_BODY_BIAS.corner_quiet.scale < 1, '角落安静=收小');
  console.log('  -> PASS: 姿态身体语言方向正确');
}

// ---------------------------------------------------------------------------
// 6. 呼吸相位：周期来自契约 breath，相位有界
// ---------------------------------------------------------------------------
{
  const p = deriveCharacterParams(baseFrame, 2.0, 7);
  assert.ok(p.breathPhase >= 0 && p.breathPhase < 1, '呼吸相位越界');
  assert.equal(p.breathAmplitude, 0.5, '呼吸振幅直通契约');
  const p2 = deriveCharacterParams(baseFrame, 2.0 + 4.0, 7);
  assert.ok(Math.abs(p2.breathPhase - p.breathPhase) < 1e-9, '一个周期后相位回归');
  console.log('  -> PASS: 呼吸相位契约直通');
}

console.log('--- All Character Mapping Checks PASSED! ---');
