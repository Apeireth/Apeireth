// 泊位收纳面纯逻辑（organize-store.ts，真实模块）——收纳分区与快捷位证明。
//
// 契约要点：
//   1. 分区/快捷位 CRUD + 跨组移动（0 装：空名/超限/冲突/不存在一律抛错）；
//   2. 切片边界如实：目标是文字目标（真实文件映射待原生面）。
import assert from 'node:assert/strict';
import {
  createGroup,
  addShortcut,
  removeShortcut,
  moveShortcut,
} from '../src/lib/starport/apps/organize-store.ts';

console.log('--- Starting Starport Organize Check ---');

// 1. CRUD 与跨组移动
{
  let groups = [createGroup('g1', '工作'), createGroup('g2', '生活')];
  groups = addShortcut(groups, 'g1', {id: 's1', name: '项目目录', target: 'C:\\projects'});
  groups = addShortcut(groups, 'g1', {id: 's2', name: '文档目录', target: 'C:\\docs'});
  assert.equal(groups[0].shortcuts.length, 2);
  assert.equal(groups[1].shortcuts.length, 0);

  groups = moveShortcut(groups, 's1', 'g1', 'g2');
  assert.equal(groups[0].shortcuts.length, 1);
  assert.equal(groups[1].shortcuts[0].id, 's1');

  groups = removeShortcut(groups, 'g2', 's1');
  assert.equal(groups[1].shortcuts.length, 0);
  console.log('  [1] CRUD + move OK');
}

// 2. 0 装边界
{
  const groups = [createGroup('g1', '工作')];
  assert.throws(() => createGroup('g2', '  '), /名称不能为空/);
  assert.throws(() => createGroup('g2', 'x'.repeat(121)), /超限/);
  assert.throws(() => addShortcut(groups, 'g1', {id: 's1', name: 'a', target: ''}), /目标不能为空/);
  assert.throws(() => addShortcut(groups, 'g1', {id: 's1', name: 'a', target: 'x'.repeat(2001)}), /超限/);
  assert.throws(() => addShortcut(groups, 'nope', {id: 's1', name: 'a', target: 't'}), /分区不存在/);

  const one = addShortcut(groups, 'g1', {id: 's1', name: 'a', target: 't'});
  assert.throws(() => addShortcut(one, 'g1', {id: 's1', name: 'b', target: 't'}), /id 冲突/);
  assert.throws(() => removeShortcut(one, 'g1', 'ghost'), /快捷位不存在/);
  assert.throws(() => moveShortcut(one, 's1', 'g1', 'ghost'), /目标分区不存在/);
  assert.throws(() => moveShortcut(one, 'ghost', 'g1', 'g1'), /不在源分区/);
  console.log('  [2] zero-fabrication boundaries OK');
}

console.log('--- Starport Organize Check PASSED ---');
