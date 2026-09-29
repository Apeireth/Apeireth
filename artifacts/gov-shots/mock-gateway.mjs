// 治理卷宗视觉自查专用 mock gateway —— 真形状数据（契约 §6/§7/§8/§9 逐字段对齐
// canonical_entry.rs / panels.rs / introspection.rs），仅供 Edge 无头截图验证布局。
// 不 ship、不进 src；GOV_EMPTY=1 时全部列表为空（空态截图）。
import http from 'node:http';

const EMPTY = process.env.GOV_EMPTY === '1';
const NOW = Date.now();
const iso = (ms) => new Date(ms).toISOString();

const S1 = '11111111-1111-4111-8111-111111111111';
const S2 = '22222222-2222-4222-8222-222222222222';

const cap = (id, read, write, operations) => ({id, supported: true, read, write, version: 1, operations, available: true});
const manifest = {
  schema_version: 1,
  runtime: {service: 'apeireth-gateway-2.0', version: '2.0.0-rc.1-mock'},
  capabilities: [
    {name: 'health', capabilities: [cap('health', true, false, ['check'])]},
    {name: 'models', capabilities: [cap('models.list', true, false, ['list'])]},
    {name: 'providers', capabilities: [cap('providers.list', true, false, ['list'])]},
    {name: 'runtime', capabilities: [cap('runtime.snapshot.read', true, false, ['read'])]},
    {name: 'chat', capabilities: [cap('chat.completions', true, true, ['complete', 'stream'])]},
    {name: 'sessions', capabilities: [cap('sessions.read', true, false, ['list'])]},
    {name: 'tools', capabilities: [cap('tools.list', true, false, ['list'])]},
    {name: 'permissions', capabilities: [
      cap('permissions.approval.read', true, false, ['list']),
      cap('permissions.approval.resolve', false, true, ['resolve']),
      cap('permissions.grants.read', true, false, ['list']),
      cap('permissions.revoke', false, true, ['revoke']),
    ]},
    {name: 'safety', capabilities: [
      cap('safety.guard.status.read', true, false, ['read']),
      cap('safety.guard.events.read', true, false, ['list']),
      cap('safety.guard.evaluate', false, true, ['evaluate']),
    ]},
    {name: 'trace', capabilities: [cap('trace.read', true, false, ['list', 'detail'])]},
    {name: 'audit', capabilities: [cap('audit.read', true, false, ['list'])]},
    {name: 'activity', capabilities: [cap('activity.sse', true, false, ['subscribe'])]},
    // ⑤ 状态条自查需要 memory.read 真计数路径；⑥ 主从化补全治理与图谱能力族
    {name: 'memory', capabilities: [
      cap('memory.read', true, false, ['list']),
      cap('memory.write', false, true, ['append']),
      cap('memory.forget', false, true, ['forget']),
      cap('memory.protect', false, true, ['protect']),
      cap('memory.unprotect', false, true, ['unprotect']),
      cap('memory.graph.read', true, false, ['read']),
    ]},
  ],
};

const sessions = [
  {id: S1, title: '整理仓库 README', created_at: NOW - 3600_000, updated_at: NOW - 120_000, message_count: 6, revision: 0},
  {id: S2, title: '夜间长谈', created_at: NOW - 86400_000, updated_at: NOW - 3600_000, message_count: 18, revision: 0},
];

const approvalsS1 = [{
  session: S1,
  approval_id: 'appr-9f2c71',
  request: 'req-88af01',
  trace_id: 'trace-5f3a91',
  capability_id: 'tool.repo',
  tool_name: 'tool.repo',
  governance_hook: 'pre_execution',
  governance_reason: '写入仓库文件属于高危调用，需要主人批准',
  command_text: 'repo.write path="README.md" mode="append"',
  arguments_summary: '{ "path": "README.md", "content": "… 追加 42 行" }',
  created_at: iso(NOW - 95_000),
  expires_at: iso(NOW + 1500_000),
}];

const grants = [
  {permission: 'execute_tool:tool.repo', capability: 'tool.repo', granted_at: null},
  {permission: 'memory.read', capability: 'memory.read', granted_at: NOW - 7200_000},
];

const guardStatus = {
  enabled: true,
  fast_guard_active: true,
  chain_guard_active: true,
  intent_guard_active: true,
  cross_turn_monitoring_active: false,
  active_chains: 1,
  total_evaluations: EMPTY ? 0 : 128,
  total_allowed: EMPTY ? 0 : 113,
  total_denied: EMPTY ? 0 : 4,
  total_approval_required: EMPTY ? 0 : 11,
  dataset_recording_enabled: true,
  ml_classifier_available: false,
  ml_model_version: null,
  ml_mode: 'off',
  ml_reason: null,
  feature_schema_version: 'v1',
  dataset_version: 'none',
};

const guardEvents = EMPTY ? [] : [
  {timestamp_ms: NOW - 300_000, session_id: S1, trace_id: 'trace-5f3a91', round: 2, capability_id: 'tool.repo', stage: 'chain_guard', decision: 'approval_required', risk_score: 0.72, reasons: ['写路径超出声明范围 repo:read'], evidence: ['path=README.md', 'declared=repo:read']},
  {timestamp_ms: NOW - 900_000, session_id: S1, trace_id: 'trace-40bc22', round: 1, capability_id: 'memory.read', stage: 'fast_guard', decision: 'allow', risk_score: 0.08, reasons: [], evidence: []},
  {timestamp_ms: NOW - 5400_000, session_id: S2, trace_id: 'trace-17de09', round: 3, capability_id: 'tool.shell', stage: 'decision_fusion', decision: 'deny', risk_score: 0.94, reasons: ['检测到破坏性命令模式 rm -rf'], evidence: ['cmd="rm -rf /tmp/build"']},
];

const traces = EMPTY ? [] : [
  {trace_id: 'trace-5f3a91', span_count: 4, started_at: NOW - 300_000, root_span: {span_id: 'sp-1', kind: 'turn', actor: 'runtime', status: 'ok', summary: '整理仓库 README（一轮对话）', started_at: NOW - 300_000, ended_at: NOW - 295_000, session_id: S1}},
  {trace_id: 'trace-17de09', span_count: 3, started_at: NOW - 5400_000, root_span: {span_id: 'sp-9', kind: 'turn', actor: 'runtime', status: 'error', summary: '清理构建目录（被守卫否决）', started_at: NOW - 5400_000, ended_at: NOW - 5396_000, session_id: S2}},
];

const spansByTrace = {
  'trace-5f3a91': [
    {span_id: 'sp-1', parent_span_id: null, kind: 'turn', actor: 'runtime', status: 'ok', summary: '整理仓库 README（一轮对话）', started_at: NOW - 300_000, ended_at: NOW - 295_000, session_id: S1},
    {span_id: 'sp-2', parent_span_id: 'sp-1', kind: 'provider', actor: 'provider.mock', status: 'ok', summary: '模型生成 2 轮', started_at: NOW - 299_800, ended_at: NOW - 296_200, session_id: S1},
    {span_id: 'sp-3', parent_span_id: 'sp-1', kind: 'tool', actor: 'tool.repo', status: 'ok', summary: 'repo.read README.md', started_at: NOW - 296_000, ended_at: NOW - 295_800, session_id: S1},
    {span_id: 'sp-4', parent_span_id: 'sp-1', kind: 'governance', actor: 'governance', status: 'ok', summary: 'repo.write 转人工审批', started_at: NOW - 295_700, ended_at: NOW - 295_100, session_id: S1},
  ],
  'trace-17de09': [
    {span_id: 'sp-9', parent_span_id: null, kind: 'turn', actor: 'runtime', status: 'error', summary: '清理构建目录（被守卫否决）', started_at: NOW - 5400_000, ended_at: NOW - 5396_000, session_id: S2},
    {span_id: 'sp-10', parent_span_id: 'sp-9', kind: 'tool', actor: 'tool.shell', status: 'error', summary: 'shell.exec 被否决', started_at: NOW - 5399_000, ended_at: NOW - 5397_000, session_id: S2},
    {span_id: 'sp-11', parent_span_id: 'sp-9', kind: 'governance', actor: 'guard', status: 'ok', summary: 'decision_fusion deny risk=0.94', started_at: NOW - 5398_900, ended_at: NOW - 5398_500, session_id: S2},
  ],
};

const auditEvents = EMPTY ? [] : [
  {ts: NOW - 295_000, event: 'chat.turn.completed', service: 'runtime', detail: 'session=11111111… rounds=2'},
  {ts: NOW - 296_000, event: 'approval.required', service: 'governance', detail: 'tool.repo appr-9f2c71'},
  {ts: NOW - 900_000, event: 'guard.evaluation', service: 'guard', detail: 'memory.read allow risk=0.08'},
  {ts: NOW - 5400_000, event: 'guard.evaluation', service: 'guard', detail: 'tool.shell deny risk=0.94'},
  {ts: NOW - 7200_000, event: 'memory.append', service: 'memory', detail: 'episode mem-01'},
];

// ---- ⑥ 记忆卷宗主从化自查夹具：真形状 episodes / graph / 治理 mutation ----
// category/importance 部分缺省（契约 §5 rc 诚实边界：schema 不存，可省略）——
// ep-m1 带真值验证显示，其余缺省验证「未分类」。
const memoryEpisodes = EMPTY ? [] : [
  {id: 'ep-m1a2b3c4', timestamp: NOW - 2600_000, role: 'assistant', content: '主人偏好纸质书胜过电子书——他说翻页的手感是阅读的一部分。下次推荐书目时优先给纸质版本信息。', session_id: S2, category: '工作记忆', importance: 0.72, protected: true, status: 'active'},
  {id: 'ep-m2d4e5f6', timestamp: NOW - 5300_000, role: 'user', content: '仓库的 README 开头那段话别动，是我写的。', session_id: S1, protected: false, status: 'active'},
  {id: 'ep-m3g6h7i8', timestamp: NOW - 8600_000, role: 'assistant', content: '夜间长谈里提到：上线前他想再听一次完整的开场动画，但只在额度充足的时候。', session_id: S2, protected: false, status: 'active'},
  {id: 'ep-m4j8k9l0', timestamp: NOW - 17400_000, role: 'user', content: '部署文档里补一段回滚步骤。', session_id: S1, protected: false, status: 'active'},
];

// graph：session→episode 包含边（契约 §5 v1 语义）；ep-m3 刻意无边（验证「还没织进图谱」空态）
const graphData = EMPTY ? {nodes: [], edges: []} : {
  nodes: [
    {id: S1, label: '整理仓库 README', kind: 'session'},
    {id: S2, label: '夜间长谈', kind: 'session'},
    {id: 'ep-m1a2b3c4', label: '主人偏好纸质书…', kind: 'episode'},
    {id: 'ep-m2d4e5f6', label: 'README 开头别动', kind: 'episode'},
    {id: 'ep-m4j8k9l0', label: '补回滚步骤', kind: 'episode'},
  ],
  edges: [
    {from: S2, to: 'ep-m1a2b3c4', weight: 1.0},
    {from: S1, to: 'ep-m2d4e5f6', weight: 1.0},
    {from: S1, to: 'ep-m4j8k9l0', weight: 0.6, label: '包含'},
  ],
};

// 治理乐观锁：每条 episode 内存 rev（初始 0），expected_rev 不符 → 409 conflict（契约 §5）。
// 截图 409 冲突态：先 curl 一次 protect（rev 0→1，模拟「别处改过」），UI 再点即冲突。
const memoryRevs = new Map();
const memoryGovern = new Map(); // id → {protected, status}
function episodeMutation(kind, id, body) {
  const ep = memoryEpisodes.find((e) => e.id === id);
  if (!ep) return {status: 404, body: {error: {code: 'not_found', message: `episode ${id} 不存在`}}};
  const rev = memoryRevs.get(id) ?? 0;
  const expected = typeof body?.expected_rev === 'number' ? body.expected_rev : -1;
  if (expected !== rev) {
    return {status: 409, body: {error: {code: 'conflict', message: `revision conflict: expected ${expected}, current ${rev}`}}};
  }
  const cur = memoryGovern.get(id) ?? {protected: !!ep.protected, status: ep.status ?? 'active'};
  if (kind === 'protect') cur.protected = true;
  if (kind === 'unprotect') cur.protected = false;
  if (kind === 'forget') cur.status = 'forgotten';
  memoryGovern.set(id, cur);
  const next = rev + 1;
  memoryRevs.set(id, next);
  return {status: 200, body: {ok: true, rev: next, id, status: cur.status, protected: cur.protected, revision: next, content: ep.content}};
}


function json(res, body, status = 200) {
  const data = JSON.stringify(body);
  res.writeHead(status, {
    'Content-Type': 'application/json; charset=utf-8',
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Headers': 'content-type, authorization',
    'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
  });
  res.end(data);
}

const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://127.0.0.1');
  const p = url.pathname;
  if (req.method === 'OPTIONS') return json(res, {});
  if (p === '/health') return json(res, {status: 'ok', execution_owner: 'apeireth-runtime::canonical'});
  if (p === '/v1/apeireth/capabilities') return json(res, manifest);
  if (p === '/v1/models') return json(res, {object: 'list', data: [{id: 'mock-model', object: 'model', created: 0, owned_by: 'mock'}]});
  if (p === '/v1/providers') return json(res, {providers: []});
  if (p === '/v1/runtime/snapshot' || p === '/v1/apeireth/') return json(res, {mock: true});
  if (p === '/v1/modules' || p === '/v1/organs') return json(res, {modules: [], organs: []});
  if (p === '/v1/workbench/turn') return json(res, {turn: null});
  if (p === '/v1/tools/list') return json(res, {tools: []});
  if (p === '/v1/memory/append' && req.method === 'POST') {
    // 契约 §5：{session, role?, content} → 201 + 新 episode（会话不存在自动入账本）
    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => {
      let parsed = null;
      try { parsed = JSON.parse(body); } catch { /* fallthrough */ }
      if (!parsed || typeof parsed.content !== 'string' || !parsed.content.trim()) {
        return json(res, {error: {code: 'bad_request', message: 'content 必填'}}, 400);
      }
      const ep = {
        id: `ep-${Math.random().toString(16).slice(2, 10)}`,
        timestamp: Date.now(),
        role: parsed.role === 'assistant' ? 'assistant' : 'user',
        content: parsed.content,
        session_id: typeof parsed.session === 'string' ? parsed.session : S1,
        protected: false,
        status: 'active',
      };
      memoryEpisodes.unshift(ep);
      json(res, ep, 201);
    });
    return;
  }
  if (p === '/v1/panel/memory/episodes') {
    // 契约 §5：?limit=&q=&session=；q 小写包含过滤 content，session 等值过滤（UUID）
    const q = (url.searchParams.get('q') || '').toLowerCase();
    const ses = url.searchParams.get('session') || '';
    const list = memoryEpisodes
      .map((e) => {
        const g = memoryGovern.get(e.id);
        return g ? {...e, protected: g.protected, status: g.status} : e;
      })
      .filter((e) => e.status !== 'forgotten') // forgotten 不再默认检索（契约 §5）
      .filter((e) => (q ? e.content.toLowerCase().includes(q) : true))
      .filter((e) => (ses ? e.session_id === ses : true));
    return json(res, {episodes: list});
  }
  // 契约 §5 治理 mutation：forget/protect/unprotect 带 expected_rev 乐观锁（409=冲突）
  const mutMatch = p.match(/^\/v1\/apeireth\/memory\/episodes\/([^/]+)\/(forget|protect|unprotect)$/);
  if (mutMatch && req.method === 'POST') {
    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => {
      let parsed = null;
      try { parsed = JSON.parse(body); } catch { /* null 即无 body */ }
      const r = episodeMutation(mutMatch[2], decodeURIComponent(mutMatch[1]), parsed);
      json(res, r.body, r.status);
    });
    return;
  }
  if (p === '/v1/panel/graph') return json(res, graphData);
  if (p === '/v1/panel/sessions') return json(res, {sessions});
  if (p === '/v1/approvals') {
    const s = url.searchParams.get('session');
    return json(res, {session: s, approvals: EMPTY ? [] : s === S1 ? approvalsS1 : []});
  }
  if (p === '/v1/approvals/resolve' && req.method === 'POST') {
    return json(res, {session: S1, request: 'req-88af01', trace_id: 'trace-5f3a91', text: '已继续执行（mock）', served_by: 'mock', rounds: 3, usage: {input_tokens: 1, output_tokens: 1}, trace: {entries: []}, events: []});
  }
  if (p === '/v1/panel/grants') return json(res, {grants: EMPTY ? [] : grants});
  if (p === '/v1/panel/grants/revoke' && req.method === 'POST') return json(res, {ok: true});
  if (p === '/v1/safety/guard/status' || p === '/v1/panel/safety/guard/status') return json(res, guardStatus);
  if (p === '/v1/safety/guard/events' || p === '/v1/panel/safety/guard/events') return json(res, {events: guardEvents});
  if (p === '/v1/safety/guard/evaluate' && req.method === 'POST') {
    return json(res, {decision: 'approval_required', stage: 'chain_guard', risk_score: 0.66, reasons: ['写操作超出最小权限建议'], evidence: ['capability=tool.repo']});
  }
  if (p === '/v1/panel/traces') return json(res, {traces});
  if (p.startsWith('/v1/panel/traces/')) {
    const id = decodeURIComponent(p.slice('/v1/panel/traces/'.length));
    return json(res, {trace_id: id, spans: spansByTrace[id] ?? []});
  }
  if (p === '/v1/panel/audit') return json(res, {events: auditEvents});
  if (p === '/v1/chat/completions' && req.method === 'POST') {
    // 打断按钮截图专用：慢速真流式（OpenAI 形状 choices[0].delta.content，
    // 与 streamChat gateway 分支解析对齐）。400ms × 20 帧 = 8s 忙碌窗，
    // 足够截「打断出现态」；流会自然收口（busy 复位），不永挂。
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Access-Control-Allow-Origin': '*',
    });
    const words = ['夜色', '落在', '星野', '上，', '远山的', '轮廓', '像', '一段', '未完的', '句子。'];
    let i = 0;
    const timer = setInterval(() => {
      res.write(`data: {"choices":[{"delta":{"content":"${words[i % words.length]} "}}]}\n\n`);
      i++;
      if (i >= 20) {
        clearInterval(timer);
        res.write('data: [DONE]\n\n');
        res.end();
      }
    }, 400);
    req.on('close', () => clearInterval(timer)); // 前端打断 = 连接断开，停帧
    return;
  }
  if (p === '/v1/apeireth/events') {
    // 对齐 canonical 真实语义：SSE 长持有不收口（15s 注释心跳保活，防代理掐线）。
    // 历史注记：旧版发一帧 300ms 收口是迁就 --virtual-time-budget 截图法；
    // 现役截图全走 CDP，收口反而让「已连接」稳态截不到。断连/重连态由
    // k5-shot lost 模式杀进程制造，不靠服务端收口模拟。
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Access-Control-Allow-Origin': '*',
    });
    res.write(`event: backend_ready\ndata: {"type":"backend_ready","at":${NOW}}\n\n`);
    const hb = setInterval(() => res.write(`: hb ${Date.now()}\n\n`), 15000);
    req.on('close', () => clearInterval(hb));
    return;
  }
  return json(res, {error: {code: 'not_found', message: `mock 未覆盖 ${p}`}}, 404);
});

server.listen(8080, '127.0.0.1', () => {
  console.log(`mock gateway on 127.0.0.1:8080 (empty=${EMPTY})`);
});
