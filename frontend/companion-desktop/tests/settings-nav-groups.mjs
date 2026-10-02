// 设置页五组归类契约（导航壳层）：五组映射、12 分区恰好各归一组、
// 深链 id 全保留（契约不破坏）、渲染接线在案。纯逻辑 + 源码镜像。
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  SETTINGS_NAV_GROUPS,
  SETTINGS_SECTION_GROUP,
  resolveSettingsSection,
} from '../src/lib/settings-nav-groups.ts';

console.log('--- Starting settings nav groups check ---');

// ① 五组齐备、组头说明非空
assert.equal(SETTINGS_NAV_GROUPS.length, 5, '五组归类');
for (const group of SETTINGS_NAV_GROUPS) {
  assert.ok(group.title.length > 0, `组 ${group.id} 有标题`);
  assert.ok(group.blurb.length > 0, `组 ${group.id} 有人话说明`);
}

// ② 12 个分区恰好各归一组（owner 清单缺口修复：cognition 归组 2）
const allIds = SETTINGS_NAV_GROUPS.flatMap((g) => [...g.sectionIds]);
assert.equal(allIds.length, 12, '12 个分区全覆盖');
assert.equal(new Set(allIds).size, 12, '无重复归组');
assert.ok(allIds.includes('cognition'), '记忆与认知已归组（清单缺口修复）');
assert.equal(SETTINGS_SECTION_GROUP.cognition, 'character', 'cognition 归组 2 它的性格');
assert.equal(SETTINGS_SECTION_GROUP.security, 'capability', '安全与治理归组 3 能力与安全');
assert.equal(SETTINGS_SECTION_GROUP.models, 'account', '模型与提供商归组 1');
assert.equal(SETTINGS_SECTION_GROUP.appearance, 'appearance', '外观归组 4');
assert.equal(SETTINGS_SECTION_GROUP.developer, 'data', '开发者选项归组 5');

// ③ 深链契约：12 个 id 全部原样通过、未知回 null
for (const id of allIds) {
  assert.equal(resolveSettingsSection(id, allIds), id, `深链 ${id} 原样通过`);
}
assert.equal(resolveSettingsSection('nope', allIds), null, '未知 id 不猜');
assert.equal(resolveSettingsSection(null, allIds), null, '空值不猜');

// ④ 渲染接线镜像：导航按组渲染 + 组头文案上屏
const view = readFileSync('src/lib/views/SettingsView.svelte', 'utf8');
assert.ok(view.includes('SETTINGS_NAV_GROUPS'), '视图导入五组映射');
assert.ok(view.includes('navGroups'), '视图派生分组导航');
assert.ok(view.includes('subnav-group-title'), '组头标题上屏');
assert.ok(view.includes('subnav-group-blurb'), '组头说明上屏');
assert.ok(!/{#each sections as sec}/.test(view), '旧平铺导航已退役');

console.log('--- All settings nav group checks passed ---');
