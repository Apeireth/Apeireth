// 设置页大类归类契约（两级导航）：五组映射、14 分区恰好各归一组、
// 深链 id 全保留（契约不破坏）、渲染接线在案。纯逻辑 + 源码镜像。
// 简化批契约：侧栏只列大类（展开才见分区页），描述词落位类别分级下的页面。
// 多开批契约：大类展开态独立（多开多收、再点即收、不自动收放），全开总高放得下。
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

// ② 14 个分区恰好各归一组（owner 清单缺口修复：cognition 归组 2；用户中心新增；
//    用户印象 = AI 智能体独立设置栏新增）
const allIds = SETTINGS_NAV_GROUPS.flatMap((g) => [...g.sectionIds]);
assert.equal(allIds.length, 14, '14 个分区全覆盖');
assert.equal(new Set(allIds).size, 14, '无重复归组');
assert.ok(allIds.includes('cognition'), '记忆与认知已归组（清单缺口修复）');
assert.equal(SETTINGS_SECTION_GROUP.cognition, 'character', 'cognition 归组 2 它的性格');
assert.equal(SETTINGS_SECTION_GROUP.impression, 'character', '用户印象归组 2 它的性格（它怎么看你）');
assert.equal(SETTINGS_SECTION_GROUP.security, 'capability', '安全与治理归组 3 能力与安全');
assert.equal(SETTINGS_SECTION_GROUP.models, 'account', '模型与提供商归组 1');
assert.equal(SETTINGS_SECTION_GROUP.user, 'account', '用户中心归组 1 账户与服务商');
assert.equal(SETTINGS_NAV_GROUPS[0].sectionIds[0], 'user', '用户中心 = 设置栏目第一页');
assert.equal(SETTINGS_SECTION_GROUP.appearance, 'appearance', '外观归组 4');
assert.equal(SETTINGS_SECTION_GROUP.developer, 'data', '开发者选项归组 5');

// ③ 深链契约：14 个 id 全部原样通过、未知回 null
for (const id of allIds) {
  assert.equal(resolveSettingsSection(id, allIds), id, `深链 ${id} 原样通过`);
}
assert.equal(resolveSettingsSection('nope', allIds), null, '未知 id 不猜');
assert.equal(resolveSettingsSection(null, allIds), null, '空值不猜');

// ④ 渲染接线镜像：大类即导航 + 多开多收（独立展开态）+ 描述词挪至类别分级下的页面
const view = readFileSync('src/lib/views/SettingsView.svelte', 'utf8');
assert.ok(view.includes('SETTINGS_NAV_GROUPS'), '视图导入五组映射');
assert.ok(view.includes('navGroups'), '视图派生分组导航');
assert.ok(view.includes('subnav-group-btn'), '侧栏只列大类（大类行是导航主件）');
assert.ok(view.includes('subnav-group-pages'), '大类展开才见其下分区页');
assert.ok(view.includes('toggleGroup('), '大类行可展开也可收起（toggle）');
assert.ok(view.includes('expandedGroups = [...expandedGroups, group.id]'), '开 = 加进展开集（不顶替别的大类）');
assert.ok(view.includes('expandedGroups = expandedGroups.filter((id) => id !== group.id)'), '收 = 从展开集移除（再点即收）');
assert.ok(!view.includes('enterGroup('), '旧「开了关不掉」交互退役');
assert.ok(view.includes('selectSection('), '分区页切换收口（runtime 落区语义保留）');
assert.ok(view.includes('subnav-group-name'), '大类标题上屏');
assert.ok(!view.includes('subnav-group-blurb'), '描述词不再堆在导航列');
assert.ok(view.includes('category-context'), '描述词挪至内容列（类别分级下的页面）');
assert.ok(view.includes('category-context-blurb'), '大类说明在页面顶部上屏');
assert.ok(!/{#each sections as sec}/.test(view), '旧平铺导航已退役');

// ⑤ 用户中心（设置栏目第一页）渲染接线镜像：分区存在、用户信息走本地存取缝
assert.ok(view.includes("label: '用户中心'"), '用户中心分区上导航');
assert.ok(view.includes("activeSection === 'user'"), '用户中心分区渲染分支存在');
assert.ok(view.includes("from '../user-profile'"), '用户信息走本地用户资料模块（暂只随应用存本地）');
assert.ok(view.includes('saveUserProfile('), '用户信息失焦/回车即存本地');
assert.ok(view.includes("label: '用户印象'"), '用户印象（AI 智能体独立设置栏）分区上导航');

// ⑥ 多开总高预算镜像：子导航不自滚是滚轮契约（wheel-scroll.mjs 钉 overflow: hidden），
// 5 大行 + 14 分区页全开必须原生放得下（980×640 实测可用 500px）——行高压缩在案。
assert.ok(/\.subnav-btn[\s\S]{0,400}?padding: 3px 12px;[\s\S]{0,80}?line-height: 16px/.test(view), '分区页 22px 行高在案');
assert.ok(/\.subnav-group-btn[\s\S]{0,400}?padding: 5px 12px; line-height: 16px/.test(view), '大类行 26px 行高在案');

console.log('--- All settings nav group checks passed ---');
