// 深望浏览面纯逻辑（browse-store.ts，真实模块）——准入闸与抓取转文本证明。
//
// 契约要点：
//   1. 协议白名单只放 HTTP/HTTPS，危险协议全数拒绝（安全第一闸）；
//   2. 站点策略精确/通配匹配，不命中如实拒绝；
//   3. 抓取转文本剥脚本/样式/标签（不等同语义树）；
//   4. 历史账本有界 50 条，去重、最新在前。
import assert from 'node:assert/strict';
import {
  normalizeBrowseUrl,
  checkBrowsePolicy,
  extractReadableText,
  pushHistory,
  BROWSE_HISTORY_LIMIT,
} from '../src/lib/starport/apps/browse-store.ts';

console.log('--- Starting Starport Browse Check ---');

// ---------------------------------------------------------------------------
// 1. 协议准入矩阵（安全第一闸）
// ---------------------------------------------------------------------------
{
  const ok = normalizeBrowseUrl('https://example.com/path?q=1');
  assert.ok(ok);
  assert.equal(ok.host, 'example.com');
  assert.equal(normalizeBrowseUrl('http://example.com')?.host, 'example.com');

  for (const bad of [
    'javascript:alert(1)',
    'data:text/html,<b>x</b>',
    'file:///C:/Windows/system32',
    'vbscript:msgbox',
    'about:blank',
    'blob:https://example.com/uuid',
    'ftp://example.com/pub',
    'mailto:a@b.c',
    'not a url',
    '',
  ]) {
    assert.equal(normalizeBrowseUrl(bad), null, `必须拒绝：${bad}`);
  }
  console.log('  [1] protocol gate OK');
}

// ---------------------------------------------------------------------------
// 2. 站点策略：精确 + 通配 + 拒绝理由
// ---------------------------------------------------------------------------
{
  const target = normalizeBrowseUrl('https://docs.example.com/page');
  const policy = {allowedHosts: ['example.org', '*.example.com']};
  assert.equal(checkBrowsePolicy(target, policy).ok, true, '通配后缀命中');

  const exact = normalizeBrowseUrl('https://example.org/');
  assert.equal(checkBrowsePolicy(exact, policy).ok, true, '精确命中');

  const wild = normalizeBrowseUrl('https://example.com/');
  assert.equal(checkBrowsePolicy(wild, policy).ok, true, '裸域等于通配根');

  const denied = normalizeBrowseUrl('https://evil.test/');
  const result = checkBrowsePolicy(denied, policy);
  assert.equal(result.ok, false);
  assert.match(result.reason, /evil\.test/, '拒绝理由如实带站点名');

  const openPolicy = {allowedHosts: null};
  assert.equal(checkBrowsePolicy(denied, openPolicy).ok, true, 'null 策略=全放行');
  console.log('  [2] host policy OK');
}

// ---------------------------------------------------------------------------
// 3. 抓取转文本：剥除与还原
// ---------------------------------------------------------------------------
{
  const html = `
    <html><head><style>body {color: red}</style>
    <script>alert('x')</script></head>
    <body>
      <h1>Title</h1>
      <!-- comment -->
      <p>A&nbsp;&amp;&nbsp;B</p>
      <p>Second</p>
    </body></html>`;
  const text = extractReadableText(html);
  assert.ok(!text.includes('alert'), '脚本必须剥除');
  assert.ok(!text.includes('color: red'), '样式必须剥除');
  assert.ok(!text.includes('comment'), '注释必须剥除');
  assert.ok(!text.includes('<h1>'), '标签必须剥除');
  assert.match(text, /Title/);
  assert.match(text, /A & B/, '基础实体还原');
  console.log('  [3] fetch-to-text OK');
}

// ---------------------------------------------------------------------------
// 4. 历史账本：去重、最新在前、有界
// ---------------------------------------------------------------------------
{
  let history = [];
  for (let i = 0; i < BROWSE_HISTORY_LIMIT + 5; i += 1) {
    history = pushHistory(history, {
      href: `https://example.com/${i}`,
      host: 'example.com',
      at: i,
      title: null,
    });
  }
  assert.equal(history.length, BROWSE_HISTORY_LIMIT, '历史有界');
  assert.equal(history[0].href, 'https://example.com/54', '最新在前');

  history = pushHistory(history, {
    href: 'https://example.com/54',
    host: 'example.com',
    at: 999,
    title: 'revisit',
  });
  assert.equal(history.length, BROWSE_HISTORY_LIMIT, '重复访问不涨账');
  assert.equal(history[0].at, 999, '重复访问上浮到最前');
  console.log('  [4] history ledger OK');
}

console.log('--- Starport Browse Check PASSED ---');
