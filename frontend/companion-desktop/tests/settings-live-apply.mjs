// 设置面「三级即效」语义一致性（开关/滑杆/预设即效 · 文本失焦/回车提交 ·
// 危险动作二次确认）。import 真实 types.ts / capability-apply.ts /
// settings-live-apply.ts / desktop-bridge.ts，并对 SettingsView / App /
// ThemeSettingsPanel 的即效接线做源码级镜像校验。
//
// 契约（每类控件只有一条生效时机，页面没有「保存」按钮）：
//   1. 开关 / 滑杆 / 预设：点（或松手）即生效，pending 转圈，失败回填 + 横幅；
//   2. 文本输入：失焦或回车提交（服务商组整组提交，避免半截配置），
//      字段旁 ✓ 瞬间确认，未提交显「未保存」微提示，失败回填旧值 + 横幅；
//   3. 危险动作（删数据/清记忆/断开连接类）：即点 + 二次确认弹层，确认后即效。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const testsDir = dirname(fileURLToPath(import.meta.url));
const srcDir = join(testsDir, '..', 'src');

const {DEFAULT_CAPABILITY_TOGGLES} = await import('../src/lib/types.ts');
const {configWithCapabilities, toggledCapability} = await import('../src/lib/capability-apply.ts');
const {
  DANGER_ACTION_CONFIRMATIONS,
  configWithGatewayUrl,
  configWithPersonas,
  configWithProviderGroup,
  focusLeavesGroup,
  isSameAsCommitted,
  isTextCommitKey,
  personasSnapshot,
  providerGroupSnapshot,
  requiresDangerConfirm,
  textCommitFlag,
} = await import('../src/lib/settings-live-apply.ts');
const {capabilityEnvFromConfig} = await import('../src/lib/desktop-bridge.ts');

const settingsSrc = readFileSync(join(srcDir, 'lib/views/SettingsView.svelte'), 'utf8');
const appSrc = readFileSync(join(srcDir, 'App.svelte'), 'utf8');
const themePanelSrc = readFileSync(join(srcDir, 'lib/components/ThemeSettingsPanel.svelte'), 'utf8');

console.log('--- Starting Settings Live-Apply (three-tier) Check ---');

// ---------------------------------------------------------------------------
// 1. 开关即效接线：能力开关群全部拨动即走 apply（onSave），不等任何按钮
// ---------------------------------------------------------------------------
{
  const toggleRegion = /function handleCapabilityToggle[\s\S]*?\n  \}/.exec(settingsSrc);
  assert.ok(toggleRegion, 'SettingsView 必须有 handleCapabilityToggle');
  assert.ok(
    toggleRegion[0].includes('applyCapabilityNow('),
    'handleCapabilityToggle 拨动后必须立即走 apply（不允许只改本地状态）',
  );

  const applyRegion = /async function applyCapabilityNow[\s\S]*?\n  \}/.exec(settingsSrc);
  assert.ok(applyRegion, 'SettingsView 必须有 applyCapabilityNow（即点即生效 apply 缝）');
  assert.ok(
    applyRegion[0].includes('await onSave(configWithCapabilities(config, attempted))'),
    'applyCapabilityNow 必须走 onSave apply 路径',
  );
  assert.ok(applyRegion[0].includes('liveApplyPendingKey = key'), '开关即效必须带 pending 态');
  assert.ok(applyRegion[0].includes('[key]: previous[key]'), '推送失败必须把开关拨回原值（回填）');
  assert.ok(applyRegion[0].includes('liveApplyError = errorBannerFrom(err)'), '推送失败必须亮错误横幅');

  // 能力开关群（注册表行 / 自学习 / 沙箱子开关）都汇到同一条即效路径。
  assert.ok(settingsSrc.includes('onclick={() => toggleCap(def)}'), '能力注册表行必须拨动即生效（toggleCap）');
  assert.ok(
    /function toggleCap\(def: CapDef\)[\s\S]*?handleCapabilityToggle\(/.test(settingsSrc),
    'toggleCap 必须汇到 handleCapabilityToggle',
  );
  assert.ok(
    /function setSelfTuning\(on: boolean\)[\s\S]*?handleCapabilityToggle\('selfTuning'/.test(settingsSrc),
    '「从使用中学习」开关必须同走开关即效路径',
  );
  assert.ok(
    settingsSrc.includes("handleCapabilityToggle('shellSandbox'"),
    '沙箱子开关必须同走开关即效路径',
  );

  // 纯映射：拨动后待推送配置携带新值，其余字段不被覆盖。
  const toggled = toggledCapability(DEFAULT_CAPABILITY_TOGGLES, 'memoryInjection', false);
  const cfg = configWithCapabilities({baseUrl: 'http://127.0.0.1:1', model: 'm'}, toggled);
  assert.equal(cfg.capabilities.memoryInjection, false, '待推送配置必须携带开关后的值');
  assert.equal(cfg.baseUrl, 'http://127.0.0.1:1', '其余配置字段不被覆盖（叠加不覆盖）');
  console.log('  -> PASS: 能力开关群拨动即走 apply（pending + 失败回填 + 横幅）');
}

// ---------------------------------------------------------------------------
// 2. 滑杆即效：松手（change）立即 applyKnobNow，失败逐键回填
// ---------------------------------------------------------------------------
{
  const SLIDERS = [
    ['遗忘衰减强度', 'memoryFade'],
    ['好奇心强度', 'curiosityStrength'],
    ['语气情绪饱和度', 'toneSaturation'],
    ['整合节奏', 'consolidationCadence'],
    ['检索温度', 'morphologyTemperature'],
    ['顾问数量', 'councilAdvisors'],
  ];
  for (const [label, key] of SLIDERS) {
    const region = new RegExp(`aria-label="${label}"[\\s\\S]{0,240}?onchange=\\{\\(\\) => applyKnobNow\\(\\['${key}'\\]`).exec(settingsSrc);
    assert.ok(region, `「${label}」滑杆松手必须立即 applyKnobNow(['${key}'])（滑杆即效）`);
  }

  const knobRegion = /async function applyKnobNow[\s\S]*?\n  \}/.exec(settingsSrc);
  assert.ok(knobRegion, 'SettingsView 必须有 applyKnobNow（旋钮即效 apply 缝）');
  assert.ok(
    knobRegion[0].includes('await onSave(configWithCapabilities(config, attempted))'),
    'applyKnobNow 必须走同一条 onSave apply 路径',
  );
  assert.ok(knobRegion[0].includes('previous[key]'), '滑杆推送失败必须回填旧值');
  assert.ok(knobRegion[0].includes('runLiveApply('), '滑杆即效必须复用统一 pending/横幅 外壳');

  // 纯映射：滑杆值直达注入字段（生效 = 运行时环境变量收到新值）。
  const env = capabilityEnvFromConfig({...DEFAULT_CAPABILITY_TOGGLES, memoryFade: 2.0});
  assert.equal(env.tune_memory_fade, 2.0, '滑杆值必须映射到 tune_memory_fade');
  console.log('  -> PASS: 六根滑杆松手即效 + 失败逐键回填');
}

// ---------------------------------------------------------------------------
// 3. 文本失焦/回车提交：整组 focusout / Enter 提交，✓ 与「未保存」微提示
// ---------------------------------------------------------------------------
{
  assert.equal(isTextCommitKey('Enter'), true, '回车 = 提交');
  assert.equal(isTextCommitKey('Escape'), false, '非回车不提交');
  assert.equal(focusLeavesGroup(true), false, '焦点仍在组内（Tab/点组内按钮）不提交');
  assert.equal(focusLeavesGroup(false), true, '焦点离开整组必须提交');
  assert.deepEqual(
    [textCommitFlag(true, false), textCommitFlag(false, true), textCommitFlag(false, false)],
    ['dirty', 'ack', null],
    '字段旁注状态：未提交显「未保存」，提交成功闪 ✓',
  );

  // 源码镜像：组容器接线 + 提交入口 + 字段旁注。
  assert.ok(
    settingsSrc.includes('onfocusout={(e) => groupFocusOut(e, providerGroupEl, () => void submitProviderGroup())}'),
    '服务商组必须整组失焦提交',
  );
  assert.ok(
    settingsSrc.includes('onkeydown={(e) => groupKeydown(e, () => void submitProviderGroup())}'),
    '服务商组必须支持回车整组提交',
  );
  assert.ok(
    settingsSrc.includes('onfocusout={() => void submitGatewayUrl()}'),
    '网关地址必须失焦提交',
  );
  assert.ok(
    settingsSrc.includes('onfocusout={() => void submitWorkspaceDir()}'),
    '工作区路径必须失焦提交',
  );
  assert.ok(
    /onfocusout=\{\(e\) =>\s*\n?\s*groupFocusOut\(e, personasGroupEl, \(\) => void submitPersonas\(\)\)\}/.test(settingsSrc),
    '人设组必须整组失焦提交',
  );
  assert.ok(settingsSrc.includes('class="field-flag field-dirty"'), '未提交必须显「未保存」微提示');
  assert.ok(settingsSrc.includes('class="field-flag field-ack"'), '提交成功必须闪 ✓ 瞬间确认');
  assert.ok(settingsSrc.includes('>未保存</span>'), '「未保存」微提示文案在案');
  assert.ok(
    /function groupFocusOut[\s\S]*?focusLeavesGroup\(/.test(settingsSrc),
    '组失焦判定必须走纯函数 focusLeavesGroup',
  );
  assert.ok(
    /function groupKeydown[\s\S]*?isTextCommitKey\(/.test(settingsSrc),
    '回车提交判定必须走纯函数 isTextCommitKey',
  );
  assert.ok(
    settingsSrc.includes('function markTextDirty(key: string): void'),
    '输入即标「未提交」',
  );
  assert.ok(
    /function ackTextKeys\(keys: string\[\]\)[\s\S]*?setTimeout/.test(settingsSrc),
    '提交成功必须闪 ✓ 后收起',
  );
  console.log('  -> PASS: 文本失焦/回车提交 + ✓ 确认 + 「未保存」微提示接线在案');
}

// ---------------------------------------------------------------------------
// 4. 失败回填：各项提交失败都回填旧值 + 横幅；App 侧配置一并回退
// ---------------------------------------------------------------------------
{
  const runner = /async function runLiveApply[\s\S]*?\n  \}/.exec(settingsSrc);
  assert.ok(runner, 'SettingsView 必须有 runLiveApply（统一即效外壳）');
  assert.ok(runner[0].includes('rollback();'), '失败必须回填该项旧值');
  assert.ok(runner[0].includes('liveApplyError = errorBannerFrom(err)'), '失败必须亮错误横幅');
  assert.ok(runner[0].includes('if (seq !== liveApplySeq) return;'), '回填只对最后一次在途改动生效');

  const providerRegion = /async function submitProviderGroup[\s\S]*?\n  \}/.exec(settingsSrc);
  assert.ok(providerRegion, '必须有 submitProviderGroup');
  assert.ok(
    providerRegion[0].includes('loadProviderGroup(providerCommitted)'),
    '服务商组提交失败必须整组回填旧值',
  );

  const gatewayRegion = /async function submitGatewayUrl[\s\S]*?\n  \}/.exec(settingsSrc);
  assert.ok(gatewayRegion, '必须有 submitGatewayUrl');
  assert.ok(gatewayRegion[0].includes('editBaseUrl = gatewayCommitted'), '网关地址提交失败必须回填旧值');

  const personaRegion = /async function submitPersonas[\s\S]*?\n  \}/.exec(settingsSrc);
  assert.ok(personaRegion, '必须有 submitPersonas');
  assert.ok(
    personaRegion[0].includes('personas = personasCommitted.personas.map'),
    '人设组提交失败必须整组回填旧值',
  );

  const workspaceRegion = /async function submitWorkspaceDir[\s\S]*?\n  \}/.exec(settingsSrc);
  assert.ok(workspaceRegion, '必须有 submitWorkspaceDir');
  assert.ok(workspaceRegion[0].includes('workspaceDir = workspaceCommitted'), '工作区路径提交失败必须回填旧值');

  // App 的 apply 缝：推送失败必须回退本地配置并把错误抛回控件层。
  assert.ok(appSrc.includes('await pushProviderEnvAndRefresh(newCfg);'), 'App 的 onSave 必须等待推送完成');
  assert.ok(appSrc.includes('applyBackendConfigOrThrow(provider, capabilities)'), '推送必须用失败可感知的 apply 入口');
  assert.ok(appSrc.includes('config = previousCfg;'), '推送失败必须把本地配置回退到改动前');
  assert.ok(/catch \(err\) \{[\s\S]*?config = previousCfg;[\s\S]*?throw err;/.test(appSrc), '回退后必须把错误抛回控件层亮横幅');
  console.log('  -> PASS: 失败回填 + 横幅 + App 侧配置回退接线在案');
}

// ---------------------------------------------------------------------------
// 5. 组提交：服务商组（端点+模型+密钥+协议头）整组一次落位，无半截配置
// ---------------------------------------------------------------------------
{
  const base = {
    baseUrl: 'http://127.0.0.1:8080',
    apiKey: 'gw-key',
    model: 'old-model',
    capabilities: DEFAULT_CAPABILITY_TOGGLES,
    personas: [{id: 'p1', name: '甲', persona: 'text'}],
    activePersonaId: 'p1',
    customBg: true,
  };
  const group = {
    protocol: 'anthropic',
    preset: 'anthropic',
    baseUrl: '  https://api.example.test  ',
    apiKey: ' sk-new ',
    model: ' model-x ',
    anthropicVersion: ' 2023-06-01 ',
  };
  const merged = configWithProviderGroup(base, group);
  assert.equal(merged.provider.protocol, 'anthropic', '协议必须整组落位');
  assert.equal(merged.provider.baseUrl, 'https://api.example.test', '端点必须整组落位（trim）');
  assert.equal(merged.provider.apiKey, 'sk-new', '密钥必须整组落位（trim）');
  assert.equal(merged.provider.model, 'model-x', '模型必须整组落位（trim）');
  assert.equal(merged.provider.anthropicVersion, '2023-06-01', '协议头必须跟协议走');
  assert.deepEqual(merged.anthropicConfig, {
    preset: 'anthropic',
    baseUrl: 'https://api.example.test',
    apiKey: 'sk-new',
    model: 'model-x',
    anthropicVersion: '2023-06-01',
  }, 'Anthropic 镜像缓冲必须同步换');
  // 叠加不覆盖：其余字段原样。
  assert.equal(merged.baseUrl, 'http://127.0.0.1:8080', '网关地址不被组提交顺手改写');
  assert.deepEqual(merged.personas, base.personas, '人设不被组提交覆盖');
  assert.equal(merged.customBg, true, '个性化字段不被组提交覆盖');
  assert.equal(merged.model, 'model-x', '顶层活动模型随组模型走');

  const openaiGroup = configWithProviderGroup(base, {...group, protocol: 'openai', preset: 'custom'});
  assert.equal(openaiGroup.provider.anthropicVersion, undefined, 'OpenAI 协议不带协议头');
  assert.deepEqual(openaiGroup.anthropicConfig, base.anthropicConfig, '非当前协议的镜像缓冲不动');
  assert.deepEqual(openaiGroup.openaiConfig, {
    preset: 'custom',
    baseUrl: 'https://api.example.test',
    apiKey: 'sk-new',
    model: 'model-x',
  }, 'OpenAI 镜像缓冲必须同步换');

  const empty = configWithProviderGroup(base, {...group, protocol: 'openai', model: ''}, 'deepseek-v4-flash');
  assert.equal(empty.model, 'deepseek-v4-flash', '模型留空时按回退默认值落位');

  // 快照：纯空白差异不算改动（空提交不打扰运行时）。
  assert.equal(
    providerGroupSnapshot({...group, baseUrl: ' https://api.example.test  '}),
    providerGroupSnapshot(group),
    'trim 后一致的组必须判为同快照',
  );
  assert.equal(isSameAsCommitted(providerGroupSnapshot(group), providerGroupSnapshot(group)), true, '同快照 = 空提交');
  assert.equal(isSameAsCommitted(providerGroupSnapshot(group), providerGroupSnapshot({...group, model: 'other'})), false, '改模型必须判为有改动');

  // 源码镜像：整组一个提交入口，值没变不推送。
  assert.ok(
    settingsSrc.includes('const updated = configWithProviderGroup(config, providerGroupDraft(), DEFAULT_MODEL_ID);'),
    '组提交必须整组一次写回（无半截配置）',
  );
  assert.ok(
    /async function submitProviderGroup[\s\S]*?isSameAsCommitted\(/.test(settingsSrc),
    '组提交必须先做空提交判定',
  );
  console.log('  -> PASS: 服务商组整组提交（四件一次落位 + 叠加不覆盖 + 空提交跳过）');
}

// ---------------------------------------------------------------------------
// 6. 危险动作二次确认：登记表全覆盖，按钮只开弹层、确认后即效
// ---------------------------------------------------------------------------
{
  const keys = Object.keys(DANGER_ACTION_CONFIRMATIONS).sort();
  assert.deepEqual(
    keys,
    ['clearCustomBg', 'clearLocalData', 'deleteStoredKey', 'removePersona'].sort(),
    '危险动作登记表必须恰为 删数据/清记忆/断开连接 类四项',
  );
  for (const key of keys) {
    const entry = DANGER_ACTION_CONFIRMATIONS[key];
    assert.ok(entry.title && entry.message && entry.confirmText, `${key} 必须带齐二次确认文案`);
    assert.equal(requiresDangerConfirm(key), true, `${key} 必须走二次确认`);
  }

  // 源码镜像：按钮只 requestDanger（不直接执行），确认后 runDangerAction 即效。
  assert.ok(settingsSrc.includes("onclick={() => requestDanger('clearLocalData')}"), '清空本地数据必须二次确认');
  assert.ok(settingsSrc.includes("onclick={() => requestDanger('removePersona', p.id)}"), '删除伙伴必须二次确认');
  assert.ok(settingsSrc.includes("onclick={() => requestDanger('deleteStoredKey')}"), '删除已存密钥必须二次确认');
  assert.ok(!/onclick=\{deleteStoredKey\}/.test(settingsSrc), '删除密钥不允许点开即删');
  assert.ok(!/onclick=\{removePersona\(/.test(settingsSrc), '删除伙伴不允许点开即删');
  assert.ok(
    /onConfirm=\{\(\) => void runDangerAction\(\)\}/.test(settingsSrc),
    '确认弹层确认后才执行（即效）',
  );
  assert.ok(/async function runDangerAction[\s\S]*?case 'clearLocalData'/.test(settingsSrc), 'runDangerAction 收口全部危险动作');

  // 自定义背景清除（删数据类）同样二次确认。
  assert.ok(themePanelSrc.includes('showClearBgConfirm'), '清除已传图片必须二次确认');
  assert.ok(
    themePanelSrc.includes('DANGER_ACTION_CONFIRMATIONS.clearCustomBg'),
    '清除已传图片的确认文案必须来自登记表',
  );
  assert.ok(!/onclick=\{clearBg\}/.test(themePanelSrc), '清除已传图片不允许点开即删');
  console.log('  -> PASS: 四类危险动作即点只开确认弹层，确认后即效');
}

// ---------------------------------------------------------------------------
// 7. 预设/恢复基线即效：点击即走 apply（无确认框残留），失败回填
// ---------------------------------------------------------------------------
{
  // 三档性格预设 + 恢复基线：点击即生效。
  for (const name of ['省心', '均衡', '深度记忆', '恢复基线']) {
    assert.ok(
      settingsSrc.includes(`onclick={() => applyDispositionPreset('${name}')}`),
      `「${name}」预设必须点击即生效（不再挂确认框）`,
    );
  }
  const presetRegion = /function applyDispositionPreset[\s\S]*?\n  \}/.exec(settingsSrc);
  assert.ok(presetRegion, '必须有 applyDispositionPreset');
  assert.ok(presetRegion[0].includes('void applyKnobNow(keys, next);'), '预设必须走即效 apply 路径');
  assert.ok(presetRegion[0].includes('DISPOSITION_BASELINE_RESET'), '恢复基线必须回基线 + 自学习关');
  assert.ok(
    presetRegion[0].includes("'memoryFade', 'curiosityStrength', 'toneSaturation', 'consolidationCadence', 'selfTuning'"),
    '恢复基线的回填键必须含四个旋钮 + 自学习',
  );

  // 推荐配置预设同语义。
  assert.ok(settingsSrc.includes('onclick={applyRecommendedPreset}'), '「应用推荐配置」必须点击即生效');
  const recommendedRegion = /function applyRecommendedPreset\(\)[\s\S]*?\n  \}/.exec(settingsSrc);
  assert.ok(recommendedRegion, '必须有 applyRecommendedPreset');
  assert.ok(
    recommendedRegion[0].includes('void applyKnobNow([...RECOMMENDED_CAPABILITY_PRESET], next);'),
    '推荐配置必须走即效 apply 路径',
  );

  // 认知深度档位（预设档）同语义。
  const depthRegion = /function applyCognitiveDepth\(\)[\s\S]*?\n  \}/.exec(settingsSrc);
  assert.ok(depthRegion, '必须有 applyCognitiveDepth');
  assert.ok(
    depthRegion[0].includes("void applyKnobNow(['judge', 'council'], next);"),
    '认知深度档位必须点选即生效',
  );

  // 确认框式预设已全部下线（第 1 级不需要确认；确认是危险动作专属）。
  for (const gone of ['dispositionPresetPending', 'confirmDispositionPreset', 'showRecommendedConfirm', 'pendingDispositionValues']) {
    assert.ok(!settingsSrc.includes(gone), `预设确认框残留 ${gone} 必须清干净`);
  }

  // 纯映射：预设/基线写进待推送配置后直达注入字段。
  const presetCfg = configWithCapabilities(
    {baseUrl: 'u', model: 'm'},
    {...DEFAULT_CAPABILITY_TOGGLES, memoryFade: 1.5, consolidationCadence: 2},
  );
  const env = capabilityEnvFromConfig(presetCfg.capabilities);
  assert.equal(env.tune_memory_fade, 1.5, '预设值必须随 apply 注入');
  assert.equal(env.tune_consolidation_cadence, 2, '预设值必须随 apply 注入');
  console.log('  -> PASS: 三档预设/恢复基线/推荐配置/认知档位全部即点即生效');
}

console.log('--- All Settings Live-Apply (three-tier) Checks PASSED! ---');
