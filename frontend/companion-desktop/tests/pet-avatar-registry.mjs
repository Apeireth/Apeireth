// 形象层注册表（avatar-registry.ts，真实模块）——双形态可插拔的插口证明。
//
// 契约要点：
//   1. 内置双层注册（光态哨兵默认 / 微光简模回落），同吃一份 PetFrame 契约；
//   2. 认不出的 id 一律回落微光简模，桌宠绝不空窗；
//   3. 主题包形象清单严格校验：保留字 id / 路径逃逸 / 绝对路径 / 未知大版本
//      一律拒绝（不猜不修）；未知字段容忍（前向兼容）。
import assert from 'node:assert/strict';
import {
  registerBuiltInAvatarLayers,
  registerThemePackAvatar,
  registerAvatarPackV2,
  parseAvatarPackManifestV2,
  resolveAvatarLayer,
  resolveAvatarLayerOrFallback,
  listAvatarLayers,
  parseThemePackManifest,
  AVATAR_LICENSE_WHITELIST,
} from '../src/lib/pet/avatar-registry.ts';
import {
  DEFAULT_AVATAR_LAYER_ID,
  FALLBACK_AVATAR_LAYER_ID,
} from '../src/lib/pet/pet-contract.ts';

console.log('--- Starting Pet Avatar Registry Check ---');

const dummyLoad = async () => ({default: null});

// ---------------------------------------------------------------------------
// 1. 内置双层注册 + 解析（默认形象层 / 回落形象层）
// ---------------------------------------------------------------------------
{
  registerBuiltInAvatarLayers({lightSentinel: dummyLoad, emberMinimal: dummyLoad});
  const def = resolveAvatarLayer(DEFAULT_AVATAR_LAYER_ID);
  assert.ok(def, '默认形象层必须注册');
  assert.equal(def.descriptor.kind, 'builtin');
  assert.equal(def.descriptor.displayName, '光态哨兵');
  assert.equal(def.descriptor.tier0Safe, true);

  const fallback = resolveAvatarLayer(FALLBACK_AVATAR_LAYER_ID);
  assert.ok(fallback, '回落形象层必须注册');
  assert.equal(fallback.descriptor.displayName, '微光简模');

  assert.equal(listAvatarLayers().length, 2);
  console.log('  -> PASS: 内置双层注册（光态哨兵 + 微光简模）');
}

// ---------------------------------------------------------------------------
// 2. 认不出的 id → 回落微光简模（桌宠绝不空窗）
// ---------------------------------------------------------------------------
{
  const entry = resolveAvatarLayerOrFallback('no-such-layer');
  assert.equal(entry.descriptor.id, FALLBACK_AVATAR_LAYER_ID);
  console.log('  -> PASS: 未知形象层回落微光简模');
}

// ---------------------------------------------------------------------------
// 3. 主题包形象清单校验：合法形状通过，越界形状拒绝
// ---------------------------------------------------------------------------
{
  const good = parseThemePackManifest(
    JSON.stringify({
      schemaVersion: 1,
      id: 'theme-aurora',
      displayName: '极光形象层',
      module: 'layers/aurora.mjs',
      extra: '未知字段容忍',
    }),
  );
  assert.deepEqual(good, {
    schemaVersion: 1,
    id: 'theme-aurora',
    displayName: '极光形象层',
    module: 'layers/aurora.mjs',
  });

  const bad = [
    'not json',
    '{"schemaVersion":2,"id":"x","displayName":"x","module":"a.mjs"}',
    '{"schemaVersion":1,"id":"light-sentinel","displayName":"x","module":"a.mjs"}',
    '{"schemaVersion":1,"id":"ember-minimal","displayName":"x","module":"a.mjs"}',
    '{"schemaVersion":1,"id":"x","displayName":"x","module":"../escape.mjs"}',
    '{"schemaVersion":1,"id":"x","displayName":"x","module":"layers/../a.mjs"}',
    '{"schemaVersion":1,"id":"x","displayName":"x","module":"C:/abs/a.mjs"}',
    '{"schemaVersion":1,"id":"x","displayName":"x","module":"/abs/a.mjs"}',
    '{"schemaVersion":1,"id":"x","displayName":"x","module":"a.ts"}',
    '{"schemaVersion":1,"id":"","displayName":"x","module":"a.mjs"}',
  ];
  for (const json of bad) {
    assert.equal(parseThemePackManifest(json), null, `非法清单必须拒绝: ${json}`);
  }
  console.log('  -> PASS: 主题包清单严格校验（保留字/逃逸/绝对路径/版本拒绝）');
}

// ---------------------------------------------------------------------------
// 4. 主题包注册走同一张表（换形象不换契约）
// ---------------------------------------------------------------------------
{
  const manifest = parseThemePackManifest(
    JSON.stringify({schemaVersion: 1, id: 'theme-aurora', displayName: '极光形象层', module: 'layers/aurora.mjs'}),
  );
  assert.ok(manifest);
  registerThemePackAvatar(manifest, dummyLoad);
  const entry = resolveAvatarLayerOrFallback('theme-aurora');
  assert.equal(entry.descriptor.kind, 'theme-pack');
  assert.equal(entry.descriptor.tier0Safe, false);
  assert.equal(listAvatarLayers().length, 3);
  console.log('  -> PASS: 主题包形象层与内置层同表注册（插口活证据）');
}

// ---------------------------------------------------------------------------
// 5. 形象包清单 v2：许可红线进代码（白名单外/署名缺失/来源缺失一律拒）
// ---------------------------------------------------------------------------
{
  const goodV2 = {
    schemaVersion: 2,
    id: 'pack-official-test',
    displayName: '测试形象',
    module: 'layers/test.mjs',
    license: {
      spdx: 'CC-BY-4.0',
      attribution: '测试形象 © 某作者，CC-BY-4.0',
      source: 'https://example.com/pack',
      modifications: '裁切为半身 + 重映射四姿态参数',
    },
    capabilities: {
      postures: ['corner_quiet', 'head_up', 'curled_asleep', 'leaning_close'],
      expressions: ['neutral', 'warm'],
      visemes: ['closed', 'small', 'mid', 'wide'],
      gaze: true,
      blink: true,
    },
    tier0Safe: false,
    extra: '未知字段容忍',
  };

  const ok = parseAvatarPackManifestV2(JSON.stringify(goodV2));
  assert.ok(ok, '白名单内 + 署名齐全的 v2 清单必须通过');
  assert.equal(ok.license.spdx, 'CC-BY-4.0');
  assert.equal(ok.tier0Safe, false);

  // CC0 档允许空署名
  const cc0 = parseAvatarPackManifestV2(
    JSON.stringify({
      ...goodV2,
      license: {...goodV2.license, spdx: 'CC0-1.0', attribution: ''},
    }),
  );
  assert.ok(cc0, '公有领域档允许空署名');

  const reject = (mutate, why) => {
    const bad = mutate(structuredClone(goodV2));
    assert.equal(parseAvatarPackManifestV2(JSON.stringify(bad)), null, `必须拒绝: ${why}`);
  };
  reject((m) => ((m.license.spdx = 'CC-BY-NC-4.0'), m), '非商用类许可');
  reject((m) => ((m.license.spdx = 'CC-BY-ND-4.0'), m), '禁改类许可');
  reject((m) => ((m.license.spdx = 'Some-Custom-1.0'), m), '白名单外许可');
  reject((m) => ((m.license.attribution = ''), m), '署名档缺署名文案');
  reject((m) => ((m.license.source = ''), m), '缺原始来源');
  reject((m) => ((m.license.source = 'ftp://example.com/x'), m), '非 http 来源');
  reject((m) => ((m.license.modifications = ''), m), '缺修改记录');
  reject((m) => ((m.capabilities.postures = ['sleeping']), m), '未知姿态声明');
  reject((m) => ((m.tier0Safe = true), m), '形象包冒充 T0 档');
  reject((m) => ((m.id = 'light-sentinel'), m), '保留字 id');
  reject((m) => ((m.module = '../escape.mjs'), m), '模块路径逃逸');

  // —— 模型侧（v2 model 字段，与 module 二选一）——
  const modelOnly = parseAvatarPackManifestV2(
    JSON.stringify({
      schemaVersion: 2,
      id: 'pack-model-only',
      displayName: '纯模型形象包',
      model: 'models/avatar.vrm',
      license: {
        spdx: 'CC0-1.0',
        attribution: '',
        source: 'https://example.com/model',
        modifications: '无',
      },
      capabilities: goodV2.capabilities,
      tier0Safe: false,
    }),
  );
  assert.ok(modelOnly, '纯模型形象包（无层模块）必须通过');
  assert.equal(modelOnly.model, 'models/avatar.vrm');
  assert.equal('module' in modelOnly, false, '没给模块就不造键');

  const both = parseAvatarPackManifestV2(
    JSON.stringify({...goodV2, model: 'models/avatar.glb'}),
  );
  assert.ok(both && both.module === 'layers/test.mjs' && both.model === 'models/avatar.glb', '层模块与模型可同包并存');

  reject((m) => (delete m.module, m), '层模块与模型双无');
  reject((m) => ((m.module = undefined, m.model = '../escape.vrm'), m), '模型路径逃逸');
  reject((m) => ((m.module = undefined, m.model = 'models/avatar.fbx'), m), '模型扩展名非白名单');
  reject((m) => ((m.module = undefined, m.model = '/abs/avatar.vrm'), m), '模型绝对路径');

  // 白名单本身口径：非商用/禁改绝不在表内
  assert.equal(AVATAR_LICENSE_WHITELIST['CC-BY-NC-4.0'], undefined);
  assert.equal(AVATAR_LICENSE_WHITELIST['CC0-1.0'], 'public-domain');
  assert.equal(AVATAR_LICENSE_WHITELIST.MIT, 'attribution');

  registerAvatarPackV2(ok, async () => ({default: null}));
  assert.equal(listAvatarLayers().length, 4, 'v2 形象包注册走同一张表');
  console.log('  -> PASS: v2 许可红线进代码（白名单/署名/来源/姿态全拒坏样）');
}

console.log('--- All Pet Avatar Registry Checks PASSED! ---');
