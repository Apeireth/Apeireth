// 设置页大类归类（两级导航）：把 14 个设置分区归入五个大类——
// 侧栏只列大类，展开的大类才见其下的分区页；各分区渲染体一字不迁
// （源码镜像测试零破坏）。
// 组织哲学：每一组回答一个问题（它靠谁供给智能/它是谁/它能做什么/
// 它住的房间怎么布置/它的记忆与数据落在哪）。
// blurb（描述词）不进导航列——渲染落位在类别分级下的页面顶部。

export type SettingsGroupId = 'account' | 'character' | 'capability' | 'appearance' | 'data';

export interface SettingsNavGroup {
  id: SettingsGroupId;
  title: string;
  /** 一句话人话说明：这组管什么。落位在类别分级下的页面顶部，不进导航列。 */
  blurb: string;
  sectionIds: readonly string[];
}

export const SETTINGS_NAV_GROUPS: readonly SettingsNavGroup[] = [
  {
    id: 'account',
    title: '账户与服务商',
    blurb: '你是谁、它靠谁供给智能：用户资料、服务商、密钥、模型与连接。',
    sectionIds: ['user', 'models'],
  },
  {
    id: 'character',
    title: '它的性格',
    blurb: '它是谁、它怎么看你：人设、用户印象、记忆倾向、性情旋钮与成长日志。',
    sectionIds: ['personality', 'impression', 'cognition', 'disposition'],
  },
  {
    id: 'capability',
    title: '能力与安全',
    blurb: '它能做什么、要经谁允许：能力开关、预算与治理。',
    sectionIds: ['tools', 'budget', 'governance', 'security'],
  },
  {
    id: 'appearance',
    title: '外观',
    blurb: '它住的房间怎么布置：主题、背景与动效。',
    sectionIds: ['appearance'],
  },
  {
    id: 'data',
    title: '数据与工作区',
    blurb: '记忆与数据落在哪：工作区、存储、诊断与开发者项。',
    sectionIds: ['data', 'runtime', 'developer'],
  },
];

/** 分区 → 组（导航高亮与深链落组用）。 */
export const SETTINGS_SECTION_GROUP: Readonly<Record<string, SettingsGroupId>> = Object.freeze(
  Object.fromEntries(
    SETTINGS_NAV_GROUPS.flatMap((group) => group.sectionIds.map((id) => [id, group.id])),
  ) as Record<string, SettingsGroupId>,
);

/** 深链/命令面板落区：合法分区原样通过（14 个 id 全保留，深链契约不变），未知回 null。 */
export function resolveSettingsSection(
  raw: string | null | undefined,
  known: readonly string[],
): string | null {
  if (!raw) return null;
  return known.includes(raw) ? raw : null;
}
