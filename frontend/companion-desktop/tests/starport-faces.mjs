// 星港内置面清单（faces.ts，真实模块）——交付台账的诚实口径证明。
//
// 契约要点：
//   1. 三态口径唯一：live / slice / not_assembled，全表无第四态；
//   2. 每面必带 note（0 装：slice 说边界、not_assembled 说缺件，禁止空话）；
//   3. id 唯一且小写连字符；codename 唯一（命名即承诺）；
//   4. 全部 17 面在册（完工汇报单一真源）。
import assert from 'node:assert/strict';
import {
  STARPORT_FACES,
  listFacesByStatus,
  getFace,
} from '../src/lib/starport/faces.ts';

console.log('--- Starting Starport Faces Check ---');

// ---------------------------------------------------------------------------
// 1. 全表在册 + id/codename 唯一
// ---------------------------------------------------------------------------
{
  assert.equal(STARPORT_FACES.length, 17, '全部 17 面在册');
  const ids = new Set();
  const codenames = new Set();
  for (const face of STARPORT_FACES) {
    assert.match(face.id, /^[a-z][a-z0-9-]{1,63}$/, `id 非法：${face.id}`);
    assert.ok(!ids.has(face.id), `id 重复：${face.id}`);
    assert.ok(!codenames.has(face.codename), `代号重复：${face.codename}`);
    ids.add(face.id);
    codenames.add(face.codename);
    assert.ok(face.title, `缺标题：${face.id}`);
  }
  console.log('  [1] registry completeness OK');
}

// ---------------------------------------------------------------------------
// 2. 三态口径 + note 必填（诚实口径钉死）
// ---------------------------------------------------------------------------
{
  const allowed = new Set(['live', 'slice', 'not_assembled']);
  for (const face of STARPORT_FACES) {
    assert.ok(allowed.has(face.status), `未知状态：${face.status}`);
    assert.ok(face.note && face.note.length >= 8, `note 太短/缺失：${face.id}`);
    if (face.status === 'slice') {
      assert.match(face.note, /MVP|切片/, `slice 面必须写明切片边界：${face.id}`);
    }
    if (face.status === 'not_assembled') {
      assert.match(face.note, /缺/, `未组装面必须写明缺件：${face.id}`);
    }
    if (face.status === 'live') {
      assert.ok(face.note.length >= 8);
    }
  }
  console.log('  [2] honest status vocabulary OK');
}

// ---------------------------------------------------------------------------
// 3. 查询面
// ---------------------------------------------------------------------------
{
  assert.equal(getFace('host')?.codename, '星港');
  assert.equal(getFace('no-such-face'), null);
  const assembled = listFacesByStatus('live').map((f) => f.id);
  assert.ok(assembled.includes('host') && assembled.includes('agent-status'));
  const pending = listFacesByStatus('not_assembled').map((f) => f.codename);
  for (const codename of ['星语', '深流', '舵室', '信使', '矿脉', '星槎']) {
    assert.ok(pending.includes(codename), `未组装面缺登记：${codename}`);
  }
  console.log('  [3] query surface OK');
}

console.log('--- Starport Faces Check PASSED ---');
