// 提案卡（提案账上屏，真实模块 src/lib/proposal-card.ts + runtime.ts fetch 面）：
//   1. 真值渲染：提案账最近 N 条（新 → 旧）IM 卡风格 + 「建议你亲自确认」行。
//   2. 空态/读面缺席：「暂无接口」诚实空态；形状不符行丢弃（不编默认值）。
//   3. 鉴权缝：取数只走 gatewayAuthHeaders() 单源（Bearer 令牌缝）。
//   4. 只读纪律：提案账 append-only，本卡零写面（不代决、不代批、不回写）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

import {
  PROPOSAL_CONFIRM_LINE,
  PROPOSAL_LEDGER_LIMIT,
  PROPOSAL_NO_FACE,
  normalizeProposalEntry,
  normalizeProposalLedger,
  proposalBody,
  proposalCardView,
  proposalTitle,
} from '../src/lib/proposal-card.ts';

const testsDir = dirname(fileURLToPath(import.meta.url));
const proposalSrc = readFileSync(join(testsDir, '..', 'src', 'lib', 'proposal-card.ts'), 'utf8');
const panelSrc = readFileSync(
  join(testsDir, '..', 'src', 'lib', 'components', 'ProposalCard.svelte'),
  'utf8',
);
const workbenchSrc = readFileSync(
  join(testsDir, '..', 'src', 'lib', 'views', 'Workbench.svelte'),
  'utf8',
);

console.log('--- proposal card (提案账) ---');

function check(name, fn) {
  fn();
  console.log(`  ok - ${name}`);
}

/** 合同形状样例：GET /v1/panel/memory/streams?kind=proposal（§4.3，created_at = unix 秒）。 */
function sampleScan() {
  return {
    kind: 'proposal',
    subject: 'companion-main',
    count: 3,
    entries: [
      {
        id: 'prop-1',
        subject_id: 'proposal_stream: 技能沉淀',
        session_id: 'me',
        created_at: Date.UTC(2024, 9, 4, 2, 15) / 1000,
        payload: {title: '把发布前检查单沉淀为技能', content: '建议沉淀为技能卡，触发词「发布」。'},
        source: 'ai_generated',
        tags: ['proposal'],
      },
      {
        id: 'prop-2',
        subject_id: '',
        session_id: null,
        created_at: Date.UTC(2024, 9, 3, 23, 0) / 1000,
        payload: {summary: '整理本周记忆', text: '本周记忆条目较多，建议做一次整编。'},
        source: 'ai_generated',
        tags: [],
      },
      {
        id: 'prop-3',
        subject_id: 'proposal_stream: 契约回写',
        session_id: 'me',
        created_at: Date.UTC(2024, 9, 4, 1, 0) / 1000,
        payload: {note: '契约字段与代码不一致处建议回写文档。'},
        source: '',
        tags: ['proposal', 'docs'],
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// 1. 真值渲染：最近 N 条 IM 卡 + 「建议你亲自确认」行
// ---------------------------------------------------------------------------
check('真值: IM 卡行投影（标题/正文/时刻/来源/标签）+ 「建议你亲自确认」逐字', () => {
  const view = proposalCardView(normalizeProposalLedger(sampleScan()));
  assert.equal(view.available, true);
  assert.equal(view.rows.length, 3);
  assert.deepEqual(view.rows.map((row) => row.id), ['prop-1', 'prop-3', 'prop-2'], '新 → 旧定序');
  const first = view.rows[0];
  assert.equal(first.title, '把发布前检查单沉淀为技能');
  assert.equal(first.body, '建议沉淀为技能卡，触发词「发布」。');
  assert.equal(first.at, '2024-10-04 02:15', 'created_at unix 秒 → UTC 上屏');
  assert.equal(first.source, 'ai_generated');
  assert.equal(first.tags, 'proposal');
  assert.equal(first.confirmLine, '建议你亲自确认');
  assert.equal(PROPOSAL_CONFIRM_LINE, '建议你亲自确认');
  for (const row of view.rows) {
    assert.equal(row.confirmLine, PROPOSAL_CONFIRM_LINE, '每卡都带「建议你亲自确认」行');
  }
});

check('真值: 最近 N 条限幅（limit 可配，缺省 8）', () => {
  const ledger = normalizeProposalLedger(sampleScan());
  assert.equal(PROPOSAL_LEDGER_LIMIT, 8);
  const limited = proposalCardView(ledger, 2);
  assert.equal(limited.rows.length, 2);
  assert.deepEqual(limited.rows.map((row) => row.id), ['prop-1', 'prop-3']);
});

check('真值: payload 自由 JSON 的标题/正文取值口径（取不到如实「—」/JSON 摘录）', () => {
  assert.equal(proposalTitle({title: 'T'}), 'T');
  assert.equal(proposalTitle({note: 'x'}), '—', '无约定标题字段 → 「—」（不编标题）');
  assert.equal(proposalBody({content: 'C'}), 'C');
  const excerpt = proposalBody({blob: 'a'.repeat(300)});
  assert.ok(excerpt.endsWith('…'), '取不到正文 → JSON 有界摘录（截断显式「…」）');
  assert.ok(excerpt.length <= 241, '摘录有界');
});

// ---------------------------------------------------------------------------
// 2. 空态/形状纪律
// ---------------------------------------------------------------------------
check('空态: 读面缺席 = 「暂无接口」诚实空态（零假行）；空账如实空列表', () => {
  const noFace = proposalCardView(null);
  assert.equal(noFace.available, false);
  assert.deepEqual(noFace.rows, []);
  assert.ok(noFace.note.includes('暂无接口'));
  assert.ok(PROPOSAL_NO_FACE.includes('不硬造端点'), '缺读面口径如实');
  assert.equal(noFace.confirmLine, PROPOSAL_CONFIRM_LINE);

  const empty = proposalCardView(normalizeProposalLedger({kind: 'proposal', subject: 'companion-main', count: 0, entries: []}));
  assert.equal(empty.available, true, '读面在场 = 可用（空账不是缺读面）');
  assert.deepEqual(empty.rows, [], '空账零假行');
});

check('形状纪律: 形状不符的行丢弃（不编默认值冒充真值）', () => {
  assert.equal(normalizeProposalEntry(null), null);
  assert.equal(normalizeProposalEntry({id: '', created_at: 1}), null, '缺 id 丢弃');
  assert.equal(normalizeProposalEntry({id: 'x'}), null, '缺 created_at 丢弃');
  assert.equal(normalizeProposalEntry({id: 'x', created_at: NaN}), null, '非有限时刻丢弃');
  const ledger = normalizeProposalLedger({
    entries: [{id: 'ok', created_at: 1, payload: {title: 't'}}, {bogus: 1}, 'not-an-entry'],
  });
  assert.equal(ledger.entries.length, 1);
  assert.equal(ledger.subject, '', 'subject 未回显 = 空串（不编主体）');
});

// ---------------------------------------------------------------------------
// 3. 鉴权缝 + 只读 fetch（真模块 runtime.ts fetchProposalLedger）
// ---------------------------------------------------------------------------
{
  const runtime = await import('../src/lib/runtime.ts');
  const realFetch = globalThis.fetch;
  const realWindow = globalThis.window;
  const calls = [];
  globalThis.window = {
    __TAURI_INTERNALS__: {
      invoke: async (cmd) => (cmd === 'get_gateway_token' ? 'tok-proposal-face-1' : null),
    },
  };
  globalThis.fetch = async (input, init) => {
    calls.push({url: String(input), method: init?.method ?? 'GET', headers: new Headers(init?.headers)});
    return new Response(JSON.stringify(sampleScan()), {
      status: 200,
      headers: {'content-type': 'application/json'},
    });
  };
  try {
    const ledger = await runtime.fetchProposalLedger({baseUrl: 'http://gateway.test', apiKey: '', model: ''});
    assert.equal(calls.length, 1, '取数只走读面一次');
    assert.ok(
      calls[0].url.endsWith('/v1/panel/memory/streams?kind=proposal&limit=8'),
      `真源 = GET /v1/panel/memory/streams?kind=proposal（实际=${calls[0].url}）`,
    );
    assert.equal(calls[0].method, 'GET', '只读纪律：提案账取数恒 GET');
    assert.equal(
      calls[0].headers.get('authorization'),
      'Bearer tok-proposal-face-1',
      '只读面同过令牌门缝（gatewayAuthHeaders 单源）',
    );
    assert.ok(ledger !== null, '200 = 真值账');
    const view = proposalCardView(ledger);
    assert.equal(view.rows.length, 3);
    assert.equal(view.rows[0].title, '把发布前检查单沉淀为技能');
    assert.equal(view.rows[0].confirmLine, PROPOSAL_CONFIRM_LINE);
  } finally {
    globalThis.fetch = realFetch;
    if (realWindow === undefined) delete globalThis.window;
    else globalThis.window = realWindow;
  }
  check('鉴权缝: fetchProposalLedger 恒 GET + Bearer 令牌单源（gatewayAuthHeaders）', () => {});
}

{
  // 读面缺席（501/404）→ null = 诚实空态承接（不硬造端点、不静默降级成空列表）。
  const runtime = await import('../src/lib/runtime.ts');
  const realFetch = globalThis.fetch;
  try {
    for (const status of [501, 404]) {
      globalThis.fetch = async () =>
        new Response(JSON.stringify({error: {code: 'unsupported'}}), {
          status,
          headers: {'content-type': 'application/json'},
        });
      const ledger = await runtime.fetchProposalLedger({baseUrl: 'http://gateway.test', apiKey: '', model: ''});
      assert.equal(ledger, null, `读面缺席 ${status} → null（诚实空态）`);
    }
  } finally {
    globalThis.fetch = realFetch;
  }
  check('读面缺席: 501/404 → null（诚实空态保留，不摆假数据）', () => {});
}

// ---------------------------------------------------------------------------
// 4. 只读纪律 + 源码镜像
// ---------------------------------------------------------------------------
check('只读纪律: 提案卡零写面（append-only 账不代决/不代批/不回写）', () => {
  assert.ok(!proposalSrc.includes('fetch('), '提案逻辑件零网络面');
  assert.ok(!proposalSrc.includes('invoke('), '提案逻辑件零 IPC 面');
  assert.ok(!panelSrc.includes('fetch('), '提案卡零 fetch（取数走注入缝）');
  assert.ok(!panelSrc.includes('invoke('), '提案卡零 IPC');
  assert.ok(!/\b(POST|PATCH|DELETE|PUT)\b/.test(panelSrc), '提案卡零写动词');
  assert.ok(panelSrc.includes('只读纪律'), '只读纪律脚注在案');
  assert.ok(panelSrc.includes('不代决、不代批、不回写'), 'append-only 纪律上屏');
});

check('源码镜像: 取数缝/IM 卡行/工作台接线', () => {
  assert.ok(panelSrc.includes('proposalCardView('), '展示映射走纯函数');
  assert.ok(panelSrc.includes('row.confirmLine'), '「建议你亲自确认」行经视图投影上屏');
  assert.ok(panelSrc.includes('loadLedger'), '取数缝（读面一行换源）');
  assert.ok(workbenchSrc.includes('<ProposalCard'), '工作台挂提案卡');
  assert.ok(workbenchSrc.includes('fetchProposalLedger(config)'), '真源 = runtime.fetchProposalLedger');
});

console.log('proposal-card: all checks passed');
