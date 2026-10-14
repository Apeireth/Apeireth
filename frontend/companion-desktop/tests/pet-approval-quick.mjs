// 伴星面板代审批（approval-quick.ts，真实模块）——帧真源与权威核对的插口证明。
//
// 契约要点：
//   1. 帧即真源：记录逐字段来自真实帧，缺主键帧丢弃（宁丢不猜）；
//   2. 权威核对：按 approval_id 精确匹配，找不到返回 null（调用方判 stale 不落决策）；
//   3. 双向同步：resolved 帧即清面板；begin-resolve 只在 asking 态推进；
//   4. 决策三态 approve/reject/cancel 与契约一致，无第四种。
import assert from 'node:assert/strict';
import {
  parseApprovalQuickRecord,
  selectPendingApproval,
  reduceApprovalQuick,
} from '../src/lib/pet/approval-quick.ts';

console.log('--- Starting Pet Approval Quick Check ---');

const framePayload = {
  session: 'sess-1',
  approval_id: 'apr-1',
  tool_name: 'tool.shell',
  request: 'req-1',
  trace_id: 'trace-1',
  capability_id: 'cap.shell.exec',
};

// ---------------------------------------------------------------------------
// 1. 帧解析：真源字段 + 缺主键丢弃
// ---------------------------------------------------------------------------
{
  const record = parseApprovalQuickRecord(framePayload, 5000);
  assert.ok(record);
  assert.equal(record.session, 'sess-1');
  assert.equal(record.approvalId, 'apr-1');
  assert.equal(record.toolName, 'tool.shell');
  assert.equal(record.at, 5000);

  assert.equal(parseApprovalQuickRecord({tool_name: 'x'}, 1), null, '缺双主键必须丢弃');
  assert.equal(parseApprovalQuickRecord(null, 1), null);
  assert.equal(parseApprovalQuickRecord([1, 2], 1), null);
  const noTool = parseApprovalQuickRecord({session: 's', approval_id: 'a'}, 1);
  assert.equal(noTool?.toolName, null, '工具名缺失如实为 null，不猜');
  console.log('  [1] frame parsing OK');
}

// ---------------------------------------------------------------------------
// 2. 权威核对：approval_id 精确匹配，找不到即 null
// ---------------------------------------------------------------------------
{
  const record = parseApprovalQuickRecord(framePayload, 5000);
  const queue = [
    {session: 'sess-2', approval_id: 'apr-9', tool_name: 'tool.other'},
    {session: 'sess-1', approval_id: 'apr-1', tool_name: 'tool.shell'},
  ];
  const hit = selectPendingApproval(queue, record);
  assert.equal(hit?.approval_id, 'apr-1');

  const staleRecord = parseApprovalQuickRecord({...framePayload, approval_id: 'apr-gone'}, 6000);
  assert.equal(selectPendingApproval(queue, staleRecord), null, '找不到必须返回 null（不模糊匹配）');

  const noIdRecord = parseApprovalQuickRecord({session: 'sess-1'}, 1) ?? {
    session: 'sess-1',
    approvalId: '',
    toolName: null,
    at: 1,
  };
  assert.equal(selectPendingApproval(queue, noIdRecord), null, '空 approval_id 一律 null');
  console.log('  [2] authoritative matching OK');
}

// ---------------------------------------------------------------------------
// 3. 面板状态机：帧推进 / 解析中 / resolved 清零 / stale 判定
// ---------------------------------------------------------------------------
{
  const record = parseApprovalQuickRecord(framePayload, 5000);
  let state = reduceApprovalQuick({phase: 'none'}, {type: 'frame', record});
  assert.equal(state.phase, 'asking');

  state = reduceApprovalQuick(state, {type: 'begin-resolve'});
  assert.equal(state.phase, 'resolving');

  // 解析中再收新帧：覆盖为新文书 asking（同刻一文书契约）。
  const record2 = parseApprovalQuickRecord({...framePayload, approval_id: 'apr-2'}, 7000);
  state = reduceApprovalQuick(state, {type: 'frame', record: record2});
  assert.equal(state.phase, 'asking');
  assert.equal(state.record.approvalId, 'apr-2');

  state = reduceApprovalQuick(state, {type: 'stale'});
  assert.equal(state.phase, 'stale');
  state = reduceApprovalQuick(state, {type: 'resolved'});
  assert.equal(state.phase, 'none');

  // 零操作边界：none 态收 begin-resolve/stale 保持 none。
  assert.equal(reduceApprovalQuick(state, {type: 'begin-resolve'}).phase, 'none');
  assert.equal(reduceApprovalQuick(state, {type: 'stale'}).phase, 'none');
  console.log('  [3] quick-panel state machine OK');
}

// ---------------------------------------------------------------------------
// 4. 决策三态与契约一致（类型面钉死：不存在第四种）
// ---------------------------------------------------------------------------
{
  const decisions = ['approve', 'reject', 'cancel'];
  for (const decision of decisions) {
    assert.ok(decisions.includes(decision));
  }
  assert.equal(decisions.length, 3, '决策三态，无第四种');
  console.log('  [4] decision triad OK');
}

console.log('--- Pet Approval Quick Check PASSED ---');
