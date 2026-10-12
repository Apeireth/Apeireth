// 用户资料本地存取契约（lib/user-profile.ts 真模块）——「暂只随应用存本地」
// 那层的地基：
//   ① 空资料缺省 + 读回是副本；
//   ② 昵称/头像/签名存取往返，updatedAt 盖章；
//   ③ 归一化守门：脏 JSON / 非图片头像 / 超限字段一律收敛，读侧不炸；
//   ④ 订阅广播（侧栏头像实时跟随）+ 退订即停；
//   ⑤ 写侧如实抛：localStorage 不可用/满不让改动静默丢失。
// localStorage 内存 shim：模块顶层不触 DOM（仅函数内引用），动态 import 保证
// shim 先于任何 load/save 调用就位（与 config-persistence.mjs 同纪律）。
import assert from 'node:assert/strict';

const store = new Map();
globalThis.localStorage = {
  getItem: (k) => (store.has(k) ? store.get(k) : null),
  setItem: (k, v) => void store.set(k, String(v)),
  removeItem: (k) => void store.delete(k),
  clear: () => store.clear(),
};

const {
  AVATAR_MAX_CHARS,
  EMPTY_USER_PROFILE,
  USER_PROFILE_STORAGE_KEY,
  loadUserProfile,
  normalizeUserProfile,
  saveUserProfile,
  subscribeUserProfile,
} = await import('../src/lib/user-profile.ts');

console.log('--- Starting user profile (local persistence) check ---');

// ① 空资料缺省：store 空 → 全空；返回副本，改动不污染下一次读回
{
  const empty = loadUserProfile();
  assert.deepEqual(empty, {...EMPTY_USER_PROFILE}, '空 store 回落空资料');
  empty.nickname = 'mutated';
  assert.equal(loadUserProfile().nickname, '', '读回是副本，改动不污染');
}

// ② 往返：昵称/头像/签名存后活着读回，updatedAt 盖章并往返一致
{
  const saved = saveUserProfile({
    nickname: '阿星',
    avatar: 'data:image/png;base64,AAAA',
    bio: '在星环上晒太阳',
    updatedAt: 0,
  });
  assert.equal(saved.nickname, '阿星');
  assert.ok(saved.updatedAt > 0, '写入盖 updatedAt');
  const back = loadUserProfile();
  assert.equal(back.nickname, '阿星', '昵称活着读回');
  assert.equal(back.avatar, 'data:image/png;base64,AAAA', '头像活着读回');
  assert.equal(back.bio, '在星环上晒太阳', '签名活着读回');
  assert.equal(back.updatedAt, saved.updatedAt, 'updatedAt 往返一致');
  assert.ok(store.has(USER_PROFILE_STORAGE_KEY), '落盘到本地单键');
}

// ③ 归一化守门：脏数据一律收敛到安全缺省/截断，不炸界面
{
  store.set(USER_PROFILE_STORAGE_KEY, '{not json');
  assert.deepEqual(loadUserProfile(), {...EMPTY_USER_PROFILE}, '脏 JSON 回落空资料');

  assert.equal(normalizeUserProfile({avatar: 'data:text/html,<x>'}).avatar, '', '非图片 data URL 拒收');
  assert.equal(normalizeUserProfile({avatar: 'https://x/y.png'}).avatar, '', '远程 URL 拒收（本地资料不外带）');
  assert.equal(
    normalizeUserProfile({avatar: `data:image/png;base64,${'A'.repeat(AVATAR_MAX_CHARS)}`}).avatar,
    '',
    '超限头像拒收',
  );
  assert.equal(normalizeUserProfile({nickname: 'x'.repeat(200)}).nickname.length, 32, '昵称截断 32');
  assert.equal(normalizeUserProfile({bio: 'x'.repeat(400)}).bio.length, 120, '签名截断 120');
  assert.deepEqual(
    normalizeUserProfile({nickname: 42, bio: null, avatar: 7, updatedAt: -5}),
    {...EMPTY_USER_PROFILE},
    '脏字段全部收敛',
  );
}

// ④ 订阅广播：保存即通知（侧栏头像实时跟随），退订后不再收到
{
  const seen = [];
  const unsubscribe = subscribeUserProfile((p) => seen.push(p.nickname));
  saveUserProfile({...EMPTY_USER_PROFILE, nickname: '一响'});
  saveUserProfile({...EMPTY_USER_PROFILE, nickname: '再响'});
  unsubscribe();
  saveUserProfile({...EMPTY_USER_PROFILE, nickname: '不响'});
  assert.deepEqual(seen, ['一响', '再响'], '保存广播 + 退订即停');
}

// ⑤ 写侧如实抛：localStorage 不可用时不让改动静默丢失
{
  globalThis.localStorage = {
    getItem: () => null,
    setItem: () => {
      throw new Error('quota');
    },
    removeItem: () => {},
    clear: () => {},
  };
  assert.throws(
    () => saveUserProfile({...EMPTY_USER_PROFILE, nickname: '存不下'}),
    /quota/,
    '写失败如实抛（UI 显示可读理由）',
  );
}

console.log('--- All user profile checks passed ---');
