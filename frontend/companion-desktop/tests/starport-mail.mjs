// 信使邮件面纯逻辑（mail-store.ts，真实模块）——.eml 进出闸与头注入防御证明。
//
// 契约要点：
//   1. .eml 序列化/解析往返闭环（CRLF 行界、必备头缺谁报谁）；
//   2. 头注入防御：From/To/Subject 含 CR/LF 一律拒绝（不清洗不猜）；
//   3. 状态机 draft→queued→sent 固定，非法迁移抛错；
//   4. 账本 CRUD/过滤/排序、id 冲突与不存在如实抛错。
import assert from 'node:assert/strict';
import {
  MailError,
  toEml,
  parseEml,
  transitionStatus,
  createMailbox,
  MAIL_SUBJECT_MAX,
  MAIL_BODY_MAX,
} from '../src/lib/starport/apps/mail-store.ts';

console.log('--- Starting Starport Mail Check ---');

const base = {
  id: 'm1',
  direction: 'out',
  from: 'me@home.test',
  to: ['you@there.test'],
  subject: '你好',
  body: '第一行\n第二行',
  at: 1728000000000,
  status: 'draft',
};

// ---------------------------------------------------------------------------
// 1. .eml 序列化格式 + 往返闭环
// ---------------------------------------------------------------------------
{
  const eml = toEml(base);
  assert.ok(eml.includes('\r\n\r\n'), '头与正文之间 CRLF 空行');
  assert.ok(eml.startsWith('Date: '), 'Date 头在首');
  assert.ok(eml.includes('From: me@home.test'));
  assert.ok(eml.includes('To: you@there.test'));
  assert.ok(eml.includes('Subject: 你好'), '非 ASCII 直存 UTF-8');
  assert.ok(eml.includes('X-Mail-Status: draft'));

  const parsed = parseEml(eml, 'fallback');
  assert.equal(parsed.id, 'm1', 'x-mail-id 回填');
  assert.equal(parsed.from, base.from);
  assert.deepEqual(parsed.to, base.to);
  assert.equal(parsed.subject, base.subject);
  assert.equal(parsed.body.replace(/\r\n/g, '\n'), base.body, '正文往返');
  assert.equal(parsed.at, base.at);
  assert.equal(parsed.status, 'received', '解析入站即 received');
  console.log('  [1] eml round-trip OK');
}

// ---------------------------------------------------------------------------
// 2. 头注入防御（0 装：含换行一律拒）
// ---------------------------------------------------------------------------
{
  for (const [field, patch] of [
    ['From', {from: 'a@b.c\r\nBcc: evil@x.test'}],
    ['From', {from: 'a@b.c\nX-Injected: 1'}],
    ['Subject', {subject: 'hi\r\nBcc: evil@x.test'}],
  ]) {
    assert.throws(
      () => toEml({...base, ...patch}),
      (error) => error instanceof MailError && error.code === 'header-injection',
      `${field} 头注入必须拒绝`,
    );
  }
  assert.throws(
    () => toEml({...base, to: ['ok@x.test', 'bad\r\n@x.test']}),
    (error) => error instanceof MailError && error.code === 'header-injection',
    'To 收件人头注入必须拒绝',
  );
  console.log('  [2] header injection guard OK');
}

// ---------------------------------------------------------------------------
// 3. 地址与限额校验
// ---------------------------------------------------------------------------
{
  for (const bad of [{from: 'no-at-sign'}, {from: 'a b@c.d'}, {from: ''}]) {
    assert.throws(
      () => toEml({...base, ...bad}),
      (error) => error instanceof MailError && error.code === 'invalid-address',
    );
  }
  assert.throws(() => toEml({...base, to: []}), (error) => error instanceof MailError && error.code === 'invalid-address');
  assert.throws(
    () => toEml({...base, subject: 'x'.repeat(MAIL_SUBJECT_MAX + 1)}),
    (error) => error instanceof MailError && error.code === 'limit-exceeded',
  );
  assert.throws(
    () => toEml({...base, body: 'x'.repeat(MAIL_BODY_MAX + 1)}),
    (error) => error instanceof MailError && error.code === 'limit-exceeded',
  );
  console.log('  [3] address/limit validation OK');
}

// ---------------------------------------------------------------------------
// 4. 解析 0 装：缺必备头缺谁报谁、坏日期拒
// ---------------------------------------------------------------------------
{
  const full = toEml(base);
  for (const header of ['Date', 'From', 'To', 'Subject']) {
    const stripped = full
      .split('\r\n')
      .filter((line) => !line.startsWith(`${header}:`))
      .join('\r\n');
    assert.throws(
      () => parseEml(stripped, 'x'),
      (error) =>
        error instanceof MailError &&
        error.code === 'missing-header' &&
        error.message.includes(header.toLowerCase()),
      `缺 ${header} 必须报 ${header.toLowerCase()}`,
    );
  }
  assert.throws(
    () => parseEml(full.replace(/^Date: .+$/m, 'Date: not-a-date'), 'x'),
    (error) => error instanceof MailError && error.code === 'missing-header',
  );
  assert.throws(() => parseEml('', 'x'), (error) => error instanceof MailError && error.code === 'invalid-argument');
  console.log('  [4] parse zero-fabrication OK');
}

// ---------------------------------------------------------------------------
// 5. 状态机 + 账本
// ---------------------------------------------------------------------------
{
  const msg = {...base};
  const queued = transitionStatus(msg, 'queued');
  const sent = transitionStatus(queued, 'sent');
  assert.equal(sent.status, 'sent');
  assert.throws(() => transitionStatus(sent, 'draft'), (error) => error instanceof MailError && error.code === 'invalid-transition');
  assert.throws(() => transitionStatus(msg, 'sent'), (error) => error instanceof MailError && error.code === 'invalid-transition', 'draft 直跳 sent 拒绝');
  assert.throws(() => transitionStatus({...base, direction: 'in', status: 'received'}, 'queued'), (error) => error instanceof MailError && error.code === 'invalid-transition');

  const box = createMailbox();
  box.add(msg);
  box.add({...base, id: 'm2', direction: 'in', status: 'received', at: base.at + 1000});
  assert.throws(() => box.add({...base}), (error) => error instanceof MailError && error.code === 'duplicate-id');
  assert.throws(() => box.remove('ghost'), (error) => error instanceof MailError && error.code === 'missing-message');
  assert.throws(() => box.update('ghost', {}), (error) => error instanceof MailError && error.code === 'missing-message');
  assert.deepEqual(box.list().map((m) => m.id), ['m2', 'm1'], '最新在前');
  assert.deepEqual(box.list({direction: 'in'}).map((m) => m.id), ['m2']);
  assert.deepEqual(box.list({status: 'draft'}).map((m) => m.id), ['m1']);
  box.update('m1', {subject: '改题'});
  assert.equal(box.list({status: 'draft'})[0].subject, '改题');
  box.remove('m1');
  assert.equal(box.list().length, 1);
  console.log('  [5] status machine + mailbox OK');
}

console.log('--- Starport Mail Check PASSED ---');
