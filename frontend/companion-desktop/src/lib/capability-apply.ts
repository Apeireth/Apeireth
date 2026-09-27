// 能力开关的「即点即生效」映射件（纯函数，无副作用）。
//
// 设置页的开关是「拨动即生效」的视觉语言：拨动后立即走与「保存设置」同一条
// apply 路径（配置持久化 + env 注入 + 侧车重启/热应用），而不是把状态留在
// 本地等远处的保存按钮。本模块把「拨动一个开关」与「拼出待推送配置」拆成
// 纯函数，供 SettingsView 与映射测试共用同一份实现。

import type {ApeirethConfig, CapabilityToggles} from './types';

/** 能力开关（boolean 槽位）的键；数值/字符串旋钮不是开关，仍走批量保存。 */
export type CapabilityToggleKey = {
  [K in keyof CapabilityToggles]-?: CapabilityToggles[K] extends boolean ? K : never;
}[keyof CapabilityToggles];

/** 拨动一个开关后的完整能力集（不可变更新；只动目标键）。 */
export function toggledCapability(
  toggles: CapabilityToggles,
  key: CapabilityToggleKey,
  checked: boolean,
): CapabilityToggles {
  return {...toggles, [key]: checked === true};
}

/** 用最新能力集拼出待保存/待推送的配置（叠加不覆盖其余字段）。 */
export function configWithCapabilities(
  config: ApeirethConfig,
  toggles: CapabilityToggles,
): ApeirethConfig {
  return {...config, capabilities: toggles};
}
