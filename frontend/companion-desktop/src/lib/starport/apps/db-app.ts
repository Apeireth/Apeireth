// 矿脉 · 数据库台装配层（批次推进）
//
// 契约：
//   1. 真引擎真文件 —— 走 Tauri query_sqlite_file 真实只读查询（原生侧
//      READ_ONLY 打开 + 只读口径第二道闸 + 千行上限截断如实标注）；
//   2. 前端只读口径 —— classifySql 先行（纵深第一道），写类/多语句本地即拒；
//   3. CSV 导出真件 —— toCsv 标准转义，Blob 真下载；
//   4. 外部数据库驱动为扩展接线缝（界面如实注明，不设假连接钮）；
//   5. mount 返回 cleanup（清理恰好一次）。
//
// 行为在 db-store.ts（测试钉死）；本文件纯装配。

import type {AppContribution} from '../contribution-registry';
import {classifySql, toCsv} from './db-store.ts';

interface SqlQueryResult {
  columns: string[];
  rows: unknown[][];
  row_count: number;
  truncated: boolean;
  duration_ms: number;
}

function isDesktop(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function tauriInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const {invoke} = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

type Doc = Document;

function el<K extends keyof HTMLElementTagNameMap>(
  doc: Doc,
  tag: K,
  className?: string,
  text?: string,
): HTMLElementTagNameMap[K] {
  const node = doc.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

export function createDbApp(): AppContribution {
  return {
    id: 'starport-db',
    title: '矿脉',
    kind: 'internal',
    mount: (container: unknown) => {
      const root = container as HTMLElement;
      const doc = root.ownerDocument;
      const note = el(doc, 'div', 'mine-note', '只读 SQL：SQLite 文件直查（千行上限/截断如实）+ PostgreSQL 真协议驱动（SSLRequest 协商 TLS；MD5/SCRAM 认证为驱动扩展缝）；其他库驱动逐个点亮。');
      const modeSel = el(doc, 'select', 'mine-mode');
      for (const [value, label] of [
        ['sqlite', 'SQLite 文件'],
        ['postgres', 'PostgreSQL'],
        ['mysql', 'MySQL'],
      ]) {
        const option = el(doc, 'option', undefined, label);
        option.value = value;
        modeSel.append(option);
      }
      const pathInput = el(doc, 'input', 'mine-path');
      pathInput.placeholder = 'SQLite 文件绝对路径';
      const pgRow = el(doc, 'div', 'mine-pg');
      const pgHost = el(doc, 'input', 'mine-pg-host');
      pgHost.placeholder = '主机';
      const pgPort = el(doc, 'input', 'mine-pg-port');
      pgPort.placeholder = '端口(5432)';
      const pgUser = el(doc, 'input', 'mine-pg-user');
      pgUser.placeholder = '用户';
      const pgPass = el(doc, 'input', 'mine-pg-pass');
      pgPass.type = 'password';
      pgPass.placeholder = '口令（只走内存）';
      const pgDb = el(doc, 'input', 'mine-pg-db');
      pgDb.placeholder = '库名（可空）';
      const pgTls = el(doc, 'select', 'mine-pg-tls');
      for (const mode of ['require', 'plain-insecure']) {
        const option = el(doc, 'option', undefined, mode);
        option.value = mode;
        pgTls.append(option);
      }
      pgRow.append(pgHost, pgPort, pgUser, pgPass, pgDb, pgTls);
      const sqlInput = el(doc, 'textarea', 'mine-sql');
      sqlInput.placeholder = 'SELECT …（只读口径：SELECT/WITH/PRAGMA/EXPLAIN）';
      const runBtn = el(doc, 'button', 'mine-run', '查询');
      const exportBtn = el(doc, 'button', 'mine-export', '导出 CSV');
      const status = el(doc, 'div', 'mine-status');
      const tableHost = el(doc, 'div', 'mine-table');
      root.append(note, modeSel, pathInput, pgRow, sqlInput, runBtn, exportBtn, status, tableHost);

      let lastResult: SqlQueryResult | null = null;

      if (!isDesktop()) {
        status.textContent = '数据库面需要桌面壳（当前为 Web 构建）——查询禁用，不设假按钮。';
        runBtn.disabled = true;
        exportBtn.disabled = true;
        return () => {
          root.textContent = '';
        };
      }

      const run = async (): Promise<void> => {
        try {
          classifySql(sqlInput.value);
        } catch (caught) {
          status.textContent = `只读口径拒绝（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
          return;
        }
        status.textContent = '查询中…';
        tableHost.textContent = '';
        lastResult = null;
        try {
          const connConfig = {
            host: pgHost.value.trim(),
            port: Number(pgPort.value.trim() || (modeSel.value === 'mysql' ? '3306' : '5432')),
            tls: pgTls.value === 'require' ? 'require' : 'plain-insecure',
            allow_insecure_plain: pgTls.value === 'plain-insecure',
            user: pgUser.value.trim(),
            database: pgDb.value.trim() || null,
            password: pgPass.value || null,
          };
          const result =
            modeSel.value === 'postgres'
              ? await tauriInvoke<SqlQueryResult>('query_postgres', {
                  config: connConfig,
                  sql: sqlInput.value,
                })
              : modeSel.value === 'mysql'
                ? await tauriInvoke<SqlQueryResult>('query_mysql', {
                    config: connConfig,
                    sql: sqlInput.value,
                  })
                : await tauriInvoke<SqlQueryResult>('query_sqlite_file', {
                    path: pathInput.value,
                    sql: sqlInput.value,
                  });
          lastResult = result;
          const table = el(doc, 'table', 'mine-grid');
          const head = el(doc, 'tr');
          for (const column of result.columns) head.append(el(doc, 'th', undefined, column));
          table.append(head);
          for (const row of result.rows) {
            const tr = el(doc, 'tr');
            for (const cell of row) tr.append(el(doc, 'td', undefined, cell === null ? '∅' : String(cell)));
            table.append(tr);
          }
          tableHost.append(table);
          status.textContent = `${result.row_count} 行 · ${result.duration_ms}ms${result.truncated ? ' · 已截断（千行上限）' : ''}`;
        } catch (caught) {
          status.textContent = `查询失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
        }
      };

      runBtn.addEventListener('click', () => void run());
      exportBtn.addEventListener('click', () => {
        if (!lastResult) {
          status.textContent = '暂无结果可导出。';
          return;
        }
        const blob = new Blob([toCsv(lastResult.columns, lastResult.rows)], {type: 'text/csv'});
        const url = URL.createObjectURL(blob);
        const link = el(doc, 'a');
        link.href = url;
        link.download = 'starport-query.csv';
        link.click();
        URL.revokeObjectURL(url);
        status.textContent = '已导出 starport-query.csv';
      });

      return () => {
        root.textContent = '';
      };
    },
  };
}
