// 关怀卡（V3-a 双引擎关怀上屏，真实模块 src/lib/care-card.ts）：
//   1. 真值渲染：规则列表（四类触发器/参数/最近触发/下次触发）+ 打扰纪律
//      （勿扰窗口/频率上限/专注抑制）+ 心跳态两列（下次 poll / 最近命中）。
//   2. 空态：读面缺席 = 「暂无接口」诚实空态，null → 「—」，不编 0 不编时刻。
//   3. 只读纪律：写面指引只登记命令文本（care_add/care_remove 停人工审批），
//      本卡零动作面；触发器语义描述与下次触发推算逐字对齐 organ/care/rules.rs。
//   4. 换源渲染：真源 = runtime.fetchCare（GET /v1/panel/care，care_list 帧同形 +
//      关怀节真值，与 tool.care_schedule / tool.self_status 同账同源）；形状不符
//      行丢弃不编默认值；读面缺席 = CARE_NO_FACE 诚实空态。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

import {
  CARE_NO_FACE,
  CARE_READ_ONLY_NOTE,
  CARE_TRIGGER_KINDS,
  CARE_TRIGGER_LABELS,
  CARE_WRITE_GUIDANCE,
  careCardView,
  careNextTriggerAtMs,
  careRuleRow,
  careTriggerDescribe,
  formatMinutesOfDay,
  formatUtcTime,
} from '../src/lib/care-card.ts';

const testsDir = dirname(fileURLToPath(import.meta.url));
const careSrc = readFileSync(join(testsDir, '..', 'src', 'lib', 'care-card.ts'), 'utf8');
const panelSrc = readFileSync(
  join(testsDir, '..', 'src', 'lib', 'components', 'CareCard.svelte'),
  'utf8',
);
const workbenchSrc = readFileSync(
  join(testsDir, '..', 'src', 'lib', 'views', 'Workbench.svelte'),
  'utf8',
);

console.log('--- care card (V3-a) ---');

function check(name, fn) {
  fn();
  console.log(`  ok - ${name}`);
}

const NOW = Date.UTC(2024, 9, 4, 6, 0, 0); // 2024-10-04 06:00 UTC

/** 合同形状样例：care_list 帧 + self_status 关怀节同源拼装。 */
function sampleSnapshot() {
  return {
    status: {
      rules: 4,
      next_trigger_at_ms: Date.UTC(2024, 9, 4, 7, 30),
      last_care_at_ms: Date.UTC(2024, 9, 4, 5, 0),
      last_care_note: 'morning: spoke (rule_fallback)',
      heartbeat_next_poll_at_ms: Date.UTC(2024, 9, 4, 6, 5),
      heartbeat_last_hit_at_ms: Date.UTC(2024, 9, 4, 5, 0),
      reason: null,
    },
    rules: [
      {
        id: 'inactive-check',
        trigger: {kind: 'inactive', hours: 6},
        material: '问问他最近怎么样',
        enabled: true,
        created_at_ms: Date.UTC(2024, 9, 1),
        last_fired_at_ms: Date.UTC(2024, 9, 4, 4, 0),
      },
      {
        id: 'morning',
        trigger: {kind: 'daily_moment', minutes_of_day: 450},
        material: '早安问候',
        enabled: true,
        created_at_ms: Date.UTC(2024, 9, 1),
      },
      {
        id: 'anniv',
        trigger: {kind: 'anniversary', month: 4, day: 21},
        material: '纪念日祝福',
        enabled: true,
        created_at_ms: Date.UTC(2024, 9, 1),
      },
      {
        id: 'interval-check',
        trigger: {kind: 'interval', hours: 12},
        material: '嘘寒问暖',
        enabled: false,
        created_at_ms: Date.UTC(2024, 9, 1),
        last_fired_at_ms: Date.UTC(2024, 9, 4, 4, 0),
      },
    ],
    discipline: {
      quiet_start_minutes: 23 * 60,
      quiet_end_minutes: 8 * 60,
      frequency_max: 1,
      frequency_window_hours: 4,
      daily_max: 5,
      focus_gap_minutes: 3,
      catchup_minutes: 120,
    },
    lastCare: {rule_id: 'morning', at_ms: Date.UTC(2024, 9, 4, 5, 0), judge: 'rule_fallback', spoke: true, note: '早安问候'},
  };
}

// ---------------------------------------------------------------------------
// 1. 真值渲染：规则列表（四类触发器/参数/最近触发/下次触发）
// ---------------------------------------------------------------------------
check('规则列表: 四类触发器 describe 文案逐字同源 + 参数上屏', () => {
  assert.equal(CARE_TRIGGER_KINDS.length, 4, '触发器闭合词表恒四类');
  assert.deepEqual(
    CARE_TRIGGER_KINDS.map((kind) => CARE_TRIGGER_LABELS[kind]),
    ['久未联系', '作息时刻', '纪念日', '间隔循环'],
  );
  assert.equal(careTriggerDescribe({kind: 'inactive', hours: 6}), '久未联系 6 小时无交互');
  assert.equal(careTriggerDescribe({kind: 'daily_moment', minutes_of_day: 450}), '每日作息时刻 07:30');
  assert.equal(careTriggerDescribe({kind: 'anniversary', month: 4, day: 21}), '纪念日 04-21');
  assert.equal(careTriggerDescribe({kind: 'interval', hours: 12}), '间隔循环 每 12 小时');
  assert.equal(formatMinutesOfDay(450), '07:30', 'HH:mm 零填充同 format_hhmm');
  assert.ok(
    careTriggerDescribe({kind: 'cron'}).includes('触发器参数缺位'),
    '词表外触发器不编语义（原样报真因）',
  );
});

check('规则列表: 最近触发/下次触发两列真值 + 语义推算标注', () => {
  const view = careCardView(sampleSnapshot(), NOW);
  assert.equal(view.available, true);
  assert.equal(view.ruleRows.length, 4);
  const inactive = view.ruleRows[0];
  assert.equal(inactive.kindLabel, '久未联系');
  assert.equal(inactive.trigger, '久未联系 6 小时无交互');
  assert.equal(inactive.lastFired, '2024-10-04 04:00', '最近触发真值（UTC 上屏）');
  assert.equal(inactive.nextTrigger, '2024-10-04 10:00', 'next = last_fired + 6h（发起锚续算）');
  const morning = view.ruleRows[1];
  assert.equal(morning.lastFired, '—', '尚无发起记录 → 「—」（不编时刻）');
  assert.ok(morning.lastFiredNote.includes('尚无发起记录'));
  assert.equal(morning.nextTrigger, '2024-10-04 07:30', 'daily_moment 当日到点');
  const anniv = view.ruleRows[2];
  assert.equal(anniv.nextTrigger, '2025-04-21 00:00', '纪念日年翻转（UTC 零点）');
  const interval = view.ruleRows[3];
  assert.equal(interval.enabled, '停用（留档）', 'enabled=false 如实标注');
  assert.equal(interval.nextTrigger, '2024-10-04 16:00', 'interval = last_fired + 12h');
});

check('规则列表: 下次触发推算 = next_occurrence_ms 同口径（过点翻日/锚点缺位退化）', () => {
  // daily_moment 过点 → 次日同一时刻（与 rules.rs 同口径）。
  const past = careNextTriggerAtMs(
    {id: 'm', trigger: {kind: 'daily_moment', minutes_of_day: 450}, created_at_ms: 0},
    Date.UTC(2024, 9, 4, 8, 0),
  );
  assert.equal(formatUtcTime(past), '2024-10-05 07:30');
  // inactive 无发起锚 → 创建锚 + N 小时（交互锚点缺位同口径退化）。
  const inactive = careNextTriggerAtMs(
    {id: 'i', trigger: {kind: 'inactive', hours: 6}, created_at_ms: Date.UTC(2024, 9, 4, 0, 0)},
    NOW,
  );
  assert.equal(formatUtcTime(inactive), '2024-10-04 06:00', '创建锚 00:00 + 6h');
  // 参数缺位 → null（不编时刻）。
  assert.equal(careNextTriggerAtMs({id: 'x', trigger: {kind: 'interval'}, created_at_ms: 0}, NOW), null);
});

// ---------------------------------------------------------------------------
// 2. 打扰纪律只读展示 + 心跳态两列
// ---------------------------------------------------------------------------
check('打扰纪律: 勿扰窗口/频率上限/专注抑制只读展示（跨午夜标注）', () => {
  const view = careCardView(sampleSnapshot(), NOW);
  const byKey = Object.fromEntries(view.disciplineRows.map((row) => [row.key, row]));
  assert.equal(byKey.quiet.value, '23:00-08:00');
  assert.ok(byKey.quiet.note.includes('跨午夜'), '起 > 止 = 跨午夜如实标注');
  assert.equal(byKey.frequency.value, '每 4 小时 ≤ 1 次 · 每日 ≤ 5 次');
  assert.equal(byKey.focus.value, '连续交互 3 分钟内不插话');
  assert.equal(byKey.catchup.value, '过点 120 分钟内补发');
});

check('打扰纪律: 显式 null = 关窗如实报（不拿默认窗冒充）', () => {
  const snapshot = sampleSnapshot();
  snapshot.discipline = {
    quiet_start_minutes: null,
    quiet_end_minutes: null,
    frequency_max: 1,
    frequency_window_hours: 4,
    daily_max: 5,
    focus_gap_minutes: 3,
    catchup_minutes: 120,
  };
  const view = careCardView(snapshot, NOW);
  const quiet = view.disciplineRows.find((row) => row.key === 'quiet');
  assert.equal(quiet.value, '未开勿扰窗口（显式关窗）');
});

check('心跳态两列: 下次 poll / 最近命中真值；null = 未接线/尚无命中如实「—」', () => {
  const view = careCardView(sampleSnapshot(), NOW);
  assert.equal(view.heartbeatRows.length, 2, '心跳态恒两列');
  assert.deepEqual(
    view.heartbeatRows.map((row) => row.label),
    ['心跳 · 下次 poll', '心跳 · 最近命中'],
  );
  assert.equal(view.heartbeatRows[0].value, '2024-10-04 06:05');
  assert.equal(view.heartbeatRows[1].value, '2024-10-04 05:00');

  const bare = sampleSnapshot();
  bare.status = {
    rules: null,
    next_trigger_at_ms: null,
    last_care_at_ms: null,
    last_care_note: '尚无关怀记录',
    heartbeat_next_poll_at_ms: null,
    heartbeat_last_hit_at_ms: null,
    reason: 'care heartbeat producer 未接线',
  };
  const emptyView = careCardView(bare, NOW);
  assert.equal(emptyView.heartbeatRows[0].value, '—', 'null → 「—」，不冒充在跑');
  assert.ok(emptyView.heartbeatRows[0].note.includes('未运行/未接线'));
  assert.equal(emptyView.heartbeatRows[1].value, '—');
  assert.ok(emptyView.heartbeatRows[1].note.includes('尚无心跳命中'));
  assert.equal(emptyView.summaryRows[0].value, '—', '规则数 null → 「—」，不编 0');
  assert.equal(emptyView.note, 'care heartbeat producer 未接线', 'reason 原样带出');
});

// ---------------------------------------------------------------------------
// 3. 空态（读面缺席）：「暂无接口」诚实空态
// ---------------------------------------------------------------------------
check('空态: 读面缺席 = 全「—」+ 暂无接口真因（零假行、零假数）', () => {
  const view = careCardView(null);
  assert.equal(view.available, false);
  assert.deepEqual(view.ruleRows, [], '空态零假行');
  assert.ok(view.note.includes('暂无接口'));
  assert.ok(CARE_NO_FACE.includes('不硬造端点'), '缺读面口径如实');
  for (const row of [...view.heartbeatRows, ...view.summaryRows, ...view.disciplineRows]) {
    assert.equal(row.value, '—', `${row.key} 缺席 = 「—」`);
  }
  assert.ok(!JSON.stringify(view).includes('"0"'), '空态不拿 0 冒充真值');
});

// ---------------------------------------------------------------------------
// 4. 只读纪律：零动作面 + 写面归口 care_schedule 工具
// ---------------------------------------------------------------------------
check('只读纪律: 指引只登记 care_schedule 命令文本（停人工审批），零代跑', () => {
  assert.equal(CARE_WRITE_GUIDANCE.length, 3);
  for (const row of CARE_WRITE_GUIDANCE) {
    assert.ok(row.command.startsWith('tool.care_schedule'), `${row.key} 指引 = care_schedule 工具面`);
    assert.ok(!row.command.startsWith('apeireth '), 'CLI 暂无 care 子命令，不假造 CLI 指引');
  }
  assert.ok(CARE_READ_ONLY_NOTE.includes('不代跑'), '只读纪律注在案');
  assert.ok(CARE_READ_ONLY_NOTE.includes('停人工审批'), '写面治理档如实（每调用停人工审批）');
  assert.ok(!careSrc.includes('fetch('), '关怀逻辑件零网络面');
  assert.ok(!careSrc.includes('invoke('), '关怀逻辑件零 IPC 写面');
  assert.ok(!panelSrc.includes('fetch('), '关怀卡零 fetch（只读展示）');
  assert.ok(!/\b(POST|PATCH|DELETE|PUT)\b/.test(panelSrc), '关怀卡零写动词');
});

// ---------------------------------------------------------------------------
// 5. 源码镜像：面板/工作台接线
// ---------------------------------------------------------------------------
check('源码镜像: 面板走纯函数 + 心跳两列 + 指引区；工作台挂三卡', () => {
  assert.ok(panelSrc.includes('careCardView('), '展示映射走纯函数');
  assert.ok(panelSrc.includes('view.heartbeatRows'), '心跳态两列经视图投影上屏');
  assert.ok(panelSrc.includes('view.writeGuidance'), '写面指引区在案');
  assert.ok(panelSrc.includes('只读纪律'), '只读纪律脚注在案');
  assert.ok(workbenchSrc.includes('<CareCard'), '工作台挂关怀卡');
  assert.ok(workbenchSrc.includes('<TraitCard'), '工作台挂性格卡');
  assert.ok(workbenchSrc.includes('<ProposalCard'), '工作台挂提案卡');
});

check('规则行投影: 缺源列「—」+ 真因（不编造）', () => {
  const row = careRuleRow(
    {id: 'x', trigger: {kind: 'interval'}, material: '', enabled: undefined, created_at_ms: 0},
    NOW,
  );
  assert.equal(row.material, '—', '素材缺位 → 「—」');
  assert.equal(row.enabled, '—', 'enabled 缺位 → 「—」（不默认启用）');
  assert.equal(row.nextTrigger, '—', '参数缺位不编时刻');
  assert.ok(row.nextTriggerNote.includes('不编时刻'));
});

// ---------------------------------------------------------------------------
// 6. 换源渲染（真模块 runtime.fetchCare → careCardView）：读面真数据点亮全卡；
//    形状不符行丢弃不编默认值；501/404 = 诚实空态保留
// ---------------------------------------------------------------------------
{
  const runtime = await import('../src/lib/runtime.ts');
  /** 合同形状样例：GET /v1/panel/care（care_list 帧同形三节 + 关怀节真值）。 */
  const CARE_FACE_BODY = {
    status: {
      rules: 2,
      next_trigger_at_ms: Date.UTC(2024, 9, 4, 7, 30),
      last_care_at_ms: Date.UTC(2024, 9, 4, 5, 0),
      last_care_note: 'morning: spoke (rule_fallback): 早安问候',
      heartbeat_next_poll_at_ms: Date.UTC(2024, 9, 4, 6, 5),
      heartbeat_last_hit_at_ms: Date.UTC(2024, 9, 4, 5, 0),
      reason: null,
    },
    rules: [
      {
        id: 'morning',
        trigger: {kind: 'daily_moment', minutes_of_day: 450},
        trigger_kind: 'daily_moment',
        material: '早安问候',
        enabled: true,
        created_at_ms: Date.UTC(2024, 9, 1),
        last_fired_at_ms: Date.UTC(2024, 9, 4, 4, 0),
        next_trigger_at_ms: Date.UTC(2024, 9, 4, 7, 30),
      },
      {id: 'broken-row'}, // 形状不符行：丢弃，不编默认值
    ],
    discipline: {
      quiet_start_minutes: 23 * 60,
      quiet_end_minutes: 8 * 60,
      frequency_max: 1,
      frequency_window_hours: 4,
      daily_max: 5,
      focus_gap_minutes: 3,
      catchup_minutes: 120,
    },
    last_care: {
      rule_id: 'morning',
      at_ms: Date.UTC(2024, 9, 4, 5, 0),
      judge: 'rule_fallback',
      spoke: true,
      note: '早安问候',
    },
  };
  const realFetch = globalThis.fetch;
  const realWindow = globalThis.window;
  const calls = [];
  globalThis.window = {
    __TAURI_INTERNALS__: {
      invoke: async (cmd) => (cmd === 'get_gateway_token' ? 'tok-care-face-1' : null),
    },
  };
  globalThis.fetch = async (input, init) => {
    calls.push({url: String(input), headers: new Headers(init?.headers)});
    return new Response(JSON.stringify(CARE_FACE_BODY), {
      status: 200,
      headers: {'content-type': 'application/json'},
    });
  };
  try {
    const snapshot = await runtime.fetchCare({baseUrl: 'http://gateway.test', apiKey: '', model: ''});
    assert.equal(calls.length, 1, '换源后取数只走读面一次');
    assert.ok(
      calls[0].url.endsWith('/v1/panel/care'),
      `真源 = GET /v1/panel/care（实际=${calls[0].url}）`,
    );
    assert.equal(
      calls[0].headers.get('authorization'),
      'Bearer tok-care-face-1',
      '只读面同过令牌门缝（gatewayAuthHeaders 单源）',
    );
    assert.ok(snapshot !== null, '200 = 真值快照');
    assert.equal(snapshot.rules.length, 1, '形状不符行丢弃（不编默认值）');
    assert.equal(snapshot.lastCare.rule_id, 'morning', 'last_care → lastCare 归一');
    const view = careCardView(snapshot, NOW);
    assert.equal(view.available, true);
    assert.equal(view.ruleRows.length, 1);
    assert.equal(view.ruleRows[0].id, 'morning');
    assert.equal(view.ruleRows[0].nextTrigger, '2024-10-04 07:30');
    assert.equal(view.ruleRows[0].nextTriggerNote, '真值（读面给定）', '读面给定 = 真值列');
    assert.equal(view.ruleRows[0].lastFired, '2024-10-04 04:00', '发起锚真值列上屏');
    const byKey = Object.fromEntries(view.disciplineRows.map((row) => [row.key, row]));
    assert.equal(byKey.quiet.value, '23:00-08:00', '打扰纪律真值');
    assert.equal(view.heartbeatRows[0].value, '2024-10-04 06:05', '心跳两列真值');
    const summary = Object.fromEntries(view.summaryRows.map((row) => [row.key, row]));
    assert.equal(summary.rules.value, '2', '规则数真值（不编 0）');
    assert.equal(summary.next_trigger.value, '2024-10-04 07:30');
    assert.equal(summary.last_care.value, '2024-10-04 05:00');
  } finally {
    globalThis.fetch = realFetch;
    if (realWindow === undefined) delete globalThis.window;
    else globalThis.window = realWindow;
  }
  check('换源后关怀真值渲染: fetchCare 真数据点亮规则/纪律/心跳/摘要（坏行丢弃）', () => {});
}

{
  // 读面缺席（501/404）→ null = 诚实空态承接（不硬造端点、不静默降级成空表）；
  // 陌生载荷同判 null（不把不认识的响应冒充空规则表）。
  const runtime = await import('../src/lib/runtime.ts');
  const {normalizeCareSnapshot} = await import('../src/lib/care-card.ts');
  const realFetch = globalThis.fetch;
  try {
    for (const status of [501, 404]) {
      globalThis.fetch = async () =>
        new Response(JSON.stringify({error: {code: 'unsupported'}}), {
          status,
          headers: {'content-type': 'application/json'},
        });
      const snapshot = await runtime.fetchCare({baseUrl: 'http://gateway.test', apiKey: '', model: ''});
      assert.equal(snapshot, null, `读面缺席 ${status} → null（诚实空态）`);
    }
    assert.equal(normalizeCareSnapshot({bogus: 1}), null, '陌生载荷 → null（不冒充空表）');
    assert.equal(normalizeCareSnapshot(null), null);
  } finally {
    globalThis.fetch = realFetch;
  }
  check('换源后读面缺席: 501/404 → null（CARE_NO_FACE 诚实空态保留）', () => {});
}

{
  // 源码镜像：换源接线（loadCare 注入缝 ← fetchCare 只读面）。
  const runtimeSrc = readFileSync(join(testsDir, '..', 'src', 'lib', 'runtime.ts'), 'utf8');
  assert.ok(runtimeSrc.includes('/v1/panel/care'), 'runtime.ts 只读 fetch 面在案');
  assert.ok(
    /fetchCare[\s\S]{0,500}gatewayAuthHeaders\(\)/.test(runtimeSrc),
    'fetchCare 过令牌门缝（gatewayAuthHeaders 单源）',
  );
  assert.ok(workbenchSrc.includes('loadCare={'), '一行换源：注入缝接真源');
  assert.ok(workbenchSrc.includes('fetchCare(config)'), '真源 = runtime.fetchCare');
  assert.ok(!workbenchSrc.includes('loadCareSnapshot'), '旧占位缝已换（不留假旋钮）');
  check('源码镜像: 换源接线（loadCare 注入缝 ← runtime.fetchCare）', () => {});
}

console.log('care-card: all checks passed');
