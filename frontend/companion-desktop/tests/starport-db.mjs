// 矿脉数据库面纯逻辑（db-store.ts，真实模块）——只读口径与 CSV 导出证明。
//
// 契约要点：
//   1. 只读口径：SELECT/WITH/PRAGMA/EXPLAIN 放行（注释/尾分号容忍），
//      写类/DDL/多语句/空语句一律拒绝（0 装）；
//   2. CSV 转义 RFC 4180 口径：引号双写、含 " , \n \r 必裹引号、null 空串；
//   3. 行列不齐/非数组抛 invalid-argument。
import assert from 'node:assert/strict';
import {classifySql, toCsv, DbError} from '../src/lib/starport/apps/db-store.ts';

console.log('--- Starting Starport DB Check ---');

// 1. 只读口径放行面
{
  for (const sql of [
    'SELECT 1',
    '  select * from t',
    '-- lead\nSELECT 1',
    '/* block */\nSELECT 1',
    'WITH x AS (SELECT 1) SELECT * FROM x',
    'PRAGMA table_info(t)',
    'EXPLAIN QUERY PLAN SELECT 1',
    'SELECT 1;',
    'SELECT 1; -- tail comment',
  ]) {
    assert.equal(classifySql(sql), 'read', `应放行：${sql}`);
  }
  console.log('  [1] read gate allow OK');
}

// 2. 只读口径拒绝面
{
  for (const sql of [
    '',
    '   ',
    '-- only comment',
    'INSERT INTO t VALUES (1)',
    'UPDATE t SET a = 1',
    'DELETE FROM t',
    'DROP TABLE t',
    'CREATE TABLE t (a)',
    'ATTACH DATABASE \'x\' AS y',
    'VACUUM',
    'SELECT 1; DROP TABLE t',
    'SELECT 1; SELECT 2',
  ]) {
    assert.throws(() => classifySql(sql), (error) => error instanceof DbError, `应拒绝：${sql}`);
  }
  assert.throws(
    () => classifySql('SELECT 1; DROP TABLE t'),
    (error) => error instanceof DbError && error.code === 'write-rejected',
  );
  assert.throws(
    () => classifySql(''),
    (error) => error instanceof DbError && error.code === 'empty-sql',
  );
  console.log('  [2] read gate reject OK');
}

// 3. CSV 转义与表头
{
  const csv = toCsv(['a', 'b,c'], [['x"y', 'plain'], [null, 'line\nbreak'], ['tail', 'q,q']]);
  const lines = csv.split('\r\n');
  assert.equal(lines[0], 'a,"b,c"');
  assert.equal(lines[1], '"x""y",plain');
  assert.equal(lines[2], ',"line\nbreak"');
  assert.equal(lines[3], 'tail,"q,q"');
  assert.equal(lines[4], '', '文件以行界收尾');
  console.log('  [3] csv escaping OK');
}

// 4. 0 装边界
{
  assert.throws(() => toCsv(['a'], [[1, 2]]), (error) => error instanceof DbError && error.code === 'invalid-argument');
  assert.throws(() => toCsv(['a'], ['not-array']), (error) => error instanceof DbError && error.code === 'invalid-argument');
  assert.throws(() => toCsv('nope', []), (error) => error instanceof DbError && error.code === 'invalid-argument');
  console.log('  [4] zero-fabrication boundaries OK');
}

console.log('--- Starport DB Check PASSED ---');
