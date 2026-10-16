// 自定义背景上传校验纯逻辑单测（规范 §8 增补④，2026-09-22 主人拍板）
// validateCustomBgFile 是 DOM-free 纯函数；IndexedDB 三件套不在 Node 侧测
// （无 indexedDB 全局），这正是它们与校验逻辑分离的原因。
import assert from 'node:assert/strict';

import {
  validateCustomBgFile,
  CUSTOM_BG_MAX_BYTES,
  CUSTOM_BG_TYPES,
} from '../src/lib/bg-store.ts';

console.log('--- Starting Custom Background Validation Check ---');

// 1. 合法图片全类型放行
for (const type of CUSTOM_BG_TYPES) {
  const v = validateCustomBgFile(1024 * 500, type);
  assert.ok(v.ok, `${type} 应放行`);
}

// 2. 类型白名单：非图片/脚本伪装拒绝
{
  const bad = ['text/html', 'application/pdf', 'image/svg+xml', 'video/mp4', ''];
  for (const type of bad) {
    const v = validateCustomBgFile(1024, type);
    assert.ok(!v.ok, `${type || '空类型'} 应拒绝`);
    if (!v.ok) assert.ok(v.reason.length > 0, '拒绝必须带用户可读理由（0 装：不许静默失败）');
  }
}

// 3. 体积边界：0 / 超限拒绝，恰在上限放行
{
  assert.ok(!validateCustomBgFile(0, 'image/png').ok, '空文件拒绝');
  assert.ok(!validateCustomBgFile(CUSTOM_BG_MAX_BYTES + 1, 'image/png').ok, '超 12MB 拒绝');
  assert.ok(validateCustomBgFile(CUSTOM_BG_MAX_BYTES, 'image/png').ok, '恰好 12MB 放行');
}

console.log('Custom background validation check passed.');
