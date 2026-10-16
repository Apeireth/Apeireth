// 用户自带形象装载口（user-avatar.ts，真实模块）——格式合法性校验 + 免责纪律。
//
// 契约要点（avatar-pack-admission.md §1b 渠道三分）：
//   1. BYO 只校验格式合法性（扩展名/体积/容器魔数），不审用户资产许可；
//   2. 免责文案是装载前必呈现的常量（用户自担许可责任），文案不许漂移；
//   3. 读不到文件头 = 不猜不收（0 装）。
import assert from 'node:assert/strict';
import {
  validateUserAvatarFile,
  normalizeVrmMetaLicense,
  USER_AVATAR_EXTENSIONS,
  USER_AVATAR_MAX_BYTES,
  USER_AVATAR_DISCLAIMER,
  EMPTY_USER_AVATAR_META,
} from '../src/lib/pet/user-avatar.ts';

console.log('--- Starting User Avatar Port Check ---');

const gltfHead = new Uint8Array([0x67, 0x6c, 0x54, 0x46, 0x02, 0x00, 0x00, 0x00]); // "glTF" + ver
const badHead = new Uint8Array([0x50, 0x4b, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00]); // ZIP 头

// ---------------------------------------------------------------------------
// 1. 扩展名白名单：只收 .vrm / .glb（大小写不敏感）
// ---------------------------------------------------------------------------
{
  assert.equal(validateUserAvatarFile('my-avatar.vrm', 1024, gltfHead).ok, true);
  assert.equal(validateUserAvatarFile('my-avatar.VRM', 1024, gltfHead).ok, true);
  assert.equal(validateUserAvatarFile('model.glb', 1024, gltfHead).ok, true);
  for (const name of ['model.fbx', 'model.obj', 'model.png', 'model.zip', 'model']) {
    const r = validateUserAvatarFile(name, 1024, gltfHead);
    assert.equal(r.ok, false, `必须拒绝: ${name}`);
    assert.ok(r.reason.includes(USER_AVATAR_EXTENSIONS[0]), '拒绝理由要给可用格式');
  }
  console.log('  -> PASS: 扩展名白名单 .vrm/.glb');
}

// ---------------------------------------------------------------------------
// 2. 体积纪律：空文件拒、超上限拒、理由带真实数字
// ---------------------------------------------------------------------------
{
  assert.equal(validateUserAvatarFile('a.vrm', 0, gltfHead).ok, false);
  const over = validateUserAvatarFile('a.vrm', USER_AVATAR_MAX_BYTES + 1, gltfHead);
  assert.equal(over.ok, false);
  assert.ok(over.reason.includes('64MB'), '超限理由要带上限');
  assert.equal(validateUserAvatarFile('a.vrm', USER_AVATAR_MAX_BYTES, gltfHead).ok, true);
  console.log('  -> PASS: 体积上限 64MB 纪律');
}

// ---------------------------------------------------------------------------
// 3. 容器魔数：glTF 二进制头才收；读不到头不猜不收
// ---------------------------------------------------------------------------
{
  const wrong = validateUserAvatarFile('a.vrm', 1024, badHead);
  assert.equal(wrong.ok, false);
  assert.ok(wrong.reason.includes('glTF'), '拒绝理由要指明期望容器');
  assert.equal(validateUserAvatarFile('a.vrm', 1024, null).ok, false, '读不到头不猜不收');
  assert.equal(validateUserAvatarFile('a.vrm', 1024, new Uint8Array([0x67])).ok, false, '头不完整不猜');
  console.log('  -> PASS: 容器魔数校验（不猜不收）');
}

// ---------------------------------------------------------------------------
// 4. 免责纪律：文案必含责任转移与自核提示；元数据缺省诚实为 null
// ---------------------------------------------------------------------------
{
  assert.ok(USER_AVATAR_DISCLAIMER.includes('许可责任由你自担'), '免责文案必须含责任转移');
  assert.ok(USER_AVATAR_DISCLAIMER.includes('自行核对'), '免责文案必须含自核提示');
  assert.ok(USER_AVATAR_DISCLAIMER.includes('不分发'), '免责文案必须含平台不分发声明');
  for (const v of Object.values(EMPTY_USER_AVATAR_META)) {
    assert.equal(v, null, '未声明元数据必须诚实为 null');
  }
  console.log('  -> PASS: 免责文案与元数据缺省纪律');
}

// ---------------------------------------------------------------------------
// 5. 内嵌许可元数据归一：防御性读取，未声明 = null（不替用户解读）
// ---------------------------------------------------------------------------
{
  assert.deepEqual(normalizeVrmMetaLicense(null), EMPTY_USER_AVATAR_META);
  assert.deepEqual(normalizeVrmMetaLicense('junk'), EMPTY_USER_AVATAR_META);
  const modern = normalizeVrmMetaLicense({
    licenseUrl: 'https://example.com/license',
    commercialUsage: 'allow',
    allowRedistribution: 'allow',
  });
  assert.deepEqual(modern, {
    licenseUrl: 'https://example.com/license',
    commercialUsage: 'allow',
    redistribution: 'allow',
  });
  // 旧代字段名也能认（防御性跨代归一）
  const legacy = normalizeVrmMetaLicense({
    otherLicenseUrl: 'https://example.com/old-license',
    commercialUssageName: 'personalOnly',
  });
  assert.equal(legacy.licenseUrl, 'https://example.com/old-license');
  assert.equal(legacy.commercialUsage, 'personalOnly');
  assert.equal(legacy.redistribution, null, '没写就是没声明，不猜');
  // 空白字符串 = 未声明（不是声明了空白）
  assert.equal(normalizeVrmMetaLicense({licenseUrl: '   '}).licenseUrl, null);
  console.log('  -> PASS: 内嵌许可元数据跨代归一（未声明 = null）');
}

console.log('--- All User Avatar Port Checks PASSED! ---');
