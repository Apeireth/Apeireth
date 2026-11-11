<!--
  参考侧：生产组件的**只读消费者**。

  ## 纪律

  - 生产组件（`SettingsView.svelte` / `FirstRunWizard.svelte` /
    `ThemeSettingsPanel.svelte`）**只读引入**（构建期由 vite 编译 ⇒ 不复制、
    不改、不手抄）；
  - 生产的数据面模块（`runtime` / `tauri-bridge` / `desktop-bridge` /
    `user-profile` / `user-impression` / `bg-store`）由 **vite 插件拦到
    对照用替身**（喂固定 fixture）⇒ 生产文件**零改动**；
  - 数据 = `fixtures/*.json`（与引擎侧**同一份**）。

  ## 两种挂载形态

  | 面板 kind | 挂什么 | 怎么定位分区 |
  |---|---|---|
  | `settings` | 生产 `SettingsView` | `initialSection` prop（生产自带 `$effect` 会切到它并展开对应大类） |
  | `component` | 生产 `FirstRunWizard` / `ThemeSettingsPanel` | 组件自身的步骤态 |

  ## 占位说明

  **本文件是骨架**：`onSave` / `onClearSessionData` 目前是**空实现**（只记录
  调用）⇒ 迁移线开工时把它们换成「把变更写进 fixture 快照」或「记录到
  `window.__SETTINGS_CALLS__`」以便断言提交语义。
-->
<script>
  import {onMount} from 'svelte';
  import {PANELS, panelById} from '../shared/panels.js';
  import {createSettingsApi} from '../shared/settings-api.js';

  import SettingsView from '../../../../../../frontend/companion-desktop/src/lib/views/SettingsView.svelte';
  import FirstRunWizard from '../../../../../../frontend/companion-desktop/src/lib/components/FirstRunWizard.svelte';
  import ThemeSettingsPanel from '../../../../../../frontend/companion-desktop/src/lib/components/ThemeSettingsPanel.svelte';

  import navFixture from '../../fixtures/nav-groups.json';
  import capabilityFixture from '../../fixtures/capabilities.json';
  import providerFixture from '../../fixtures/provider.json';
  import characterFixture from '../../fixtures/character.json';
  import budgetFixture from '../../fixtures/budget.json';
  import dataFixture from '../../fixtures/data.json';
  import themeFixture from '../../fixtures/theme.json';
  import wizardFixture from '../../fixtures/wizard.json';

  /** 每个面的固定数据（与引擎侧同一份 JSON）。 */
  const FIXTURES = {
    'nav-groups.json': navFixture,
    'capabilities.json': capabilityFixture,
    'provider.json': providerFixture,
    'character.json': characterFixture,
    'budget.json': budgetFixture,
    'data.json': dataFixture,
    'theme.json': themeFixture,
    'wizard.json': wizardFixture,
  };

  /** 当前面（由 URL 路径决定：`/reference/<id>/index.html`）。 */
  const panelId = (() => {
    const parts = location.pathname.split('/').filter(Boolean);
    return parts.length >= 2 ? parts[parts.length - 2] : 'frame';
  })();
  const panel = panelById(panelId) ?? PANELS[0];
  const fixture = FIXTURES[panel.fixture] ?? {};

  /** 生产的配置对象（形状正确即可 ⇒ 真数据在替身里）。 */
  const config = {
    baseUrl: 'http://127.0.0.1:8080',
    apiKey: '',
    model: 'gpt-4o',
    theme: themeFixture.state?.activeTheme ?? 'origin',
    accent: themeFixture.state?.activeAccent ?? 'presence-gold',
    customBg: themeFixture.state?.customBg ?? false,
    provider: {
      protocol: providerFixture.state?.activeProtocol ?? 'openai',
      preset: providerFixture.state?.activePreset ?? 'openai',
      baseUrl: providerFixture.state?.providerBaseUrl ?? '',
      apiKey: '',
      model: providerFixture.state?.providerModel ?? 'gpt-4o',
      anthropicVersion: providerFixture.state?.anthropicVersion ?? '2023-06-01',
    },
    capabilities: capabilityFixture.capabilities ?? {},
    personas: characterFixture.personas ?? [],
    activePersonaId: characterFixture.activePersonaId ?? '',
  };

  /** 当前分区（`settings` 形态：由 `initialSection` 驱动）。 */
  let sectionId = $state(panel.reference?.initialSection ?? panel.sections[0]);

  /** 调用记录（迁移线用来断言提交语义：哪条缝被调了、带了什么）。 */
  const calls = [];

  function onSave(next) {
    calls.push({kind: 'onSave', at: calls.length});
    return Promise.resolve();
  }

  onMount(() => {
    if (typeof window !== 'undefined') {
      window.__SETTINGS_CALLS__ = calls;
      window.__SETTINGS_FIXTURE__ = fixture;
      window.__SETTINGS__ = createSettingsApi({
        panel: panel.id,
        sections: panel.sections,
        fixture,
        // 参考侧自己的裁剪选择器（按当前分区取）。
        rootSelector: () => {
          const row = panel.sections.find((item) => item.id === sectionId);
          return (row ?? panel.sections[0]).ref;
        },
        currentSection: () => sectionId,
        selectSection: (id) => {
          sectionId = id;
        },
        ready: () => true,
      });
    }
  });
</script>

<!--
  `settings` 形态：整页生产容器（只读引入）。
  ⚠️ `initialSection` 是生产 prop ⇒ 换分区 = 改这个值 ⇒ 生产自带 `$effect`
  会把 `activeSection` 切过去并展开对应大类 ⇒ **不点导航也能落到目标分区**。
-->
{#if panel.reference.kind === 'settings'}
  <SettingsView
    {config}
    {onSave}
    initialSection={sectionId}
    sessionUsage={budgetFixture.sessionUsage ?? null}
  />
{:else if panel.reference.component === 'FirstRunWizard'}
  <FirstRunWizard
    onComplete={() => {}}
    onSkip={() => {}}
  />
{:else if panel.reference.component === 'ThemeSettingsPanel'}
  <ThemeSettingsPanel {config} {onSave} />
{/if}
