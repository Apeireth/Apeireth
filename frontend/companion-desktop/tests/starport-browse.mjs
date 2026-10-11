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
  extractSemanticTree,
  findSemanticNode,
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

// ---------------------------------------------------------------------------
// 5. 语义树：去噪提取 + 确定性句柄 + 查找
// ---------------------------------------------------------------------------
{
  const html = `
    <html><head><style>.x{}</style><script>evil()</script></head><body>
      <!-- 噪声注释 -->
      <h1>标题甲</h1>
      <p>正文一。</p>
      <a href="https://a.test/x">链接一</a>
      <img src="pic.png" alt="图甲" />
      <input name="q" type="text" placeholder="搜索词" />
      <button>提交</button>
      <h2>标题乙</h2>
      <p></p>
    </body></html>`;
  const tree = extractSemanticTree(html);
  // 文档序句柄：heading:1, text:1, link:1, image:1, form-field:1, button:1, heading:2。
  assert.deepEqual(
    tree.map((node) => node.handle),
    ['heading:1', 'text:1', 'link:1', 'image:1', 'form-field:1', 'button:1', 'heading:2'],
    '文档序确定性句柄',
  );
  assert.equal(tree[0].level, 1);
  assert.equal(tree[2].href, 'https://a.test/x');
  assert.equal(tree[4].name, 'q');
  assert.ok(!tree.some((node) => node.text.includes('evil')), '脚本噪声剥除');
  assert.ok(!tree.some((node) => node.text.includes('噪声注释')), '注释噪声剥除');
  assert.ok(!tree.some((node) => node.text === ''), '空文本节点不产出');

  // 确定性：同输入恒同句柄。
  assert.deepEqual(
    extractSemanticTree(html).map((node) => node.handle),
    tree.map((node) => node.handle),
  );
  assert.equal(findSemanticNode(tree, 'link:1')?.text, '链接一');
  assert.equal(findSemanticNode(tree, 'link:99'), null, '未知句柄返回 null');
  assert.deepEqual(extractSemanticTree(''), [], '空输入空树');
  console.log('  [5] semantic tree OK');
}

console.log('--- Starport Browse Check PASSED ---');
