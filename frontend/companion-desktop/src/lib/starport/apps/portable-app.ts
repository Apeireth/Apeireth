// 星槎 · 便携面装配层（批次推进）
//
// 契约：
//   1. 状态真读 —— 走 Tauri get_portable_mode 真实判定（exe 同级 portable.flag
//      文件或 data/ 目录立便携，形态不符不猜），无假状态；
//   2. 打包随身是接线缝 —— 界面如实说明（打包脚本/发布面后续批），不设假按钮；
//   3. mount 返回 cleanup（清理恰好一次）。

import type {AppContribution} from '../contribution-registry';

interface PortableModeStatus {
  active: boolean;
  data_dir: string | null;
}

function isDesktop(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function tauriInvoke<T>(command: string): Promise<T> {
  const {invoke} = await import('@tauri-apps/api/core');
  return invoke<T>(command);
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

export function createPortableApp(): AppContribution {
  return {
    id: 'starport-portable',
    title: '星槎',
    kind: 'internal',
    mount: (container: unknown) => {
      const root = container as HTMLElement;
      const doc = root.ownerDocument;
      const note = el(doc, 'div', 'raft-note', '便携判定：exe 同级 portable.flag 文件或 data/ 目录即数据随身（形态不符不猜）；打包随身为接线缝（后续批）。');
      const status = el(doc, 'div', 'raft-status', '读取中…');
      const detail = el(doc, 'div', 'raft-detail');
      root.append(note, status, detail);

      if (!isDesktop()) {
        status.textContent = '便携判定需要桌面壳（当前为 Web 构建）。';
        return () => {
          root.textContent = '';
        };
      }

      void tauriInvoke<PortableModeStatus>('get_portable_mode')
        .then((result) => {
          status.textContent = result.active ? '便携模式生效' : '常规安装模式';
          detail.textContent = result.data_dir ? `数据目录：${result.data_dir}` : '数据目录：（常规链路）';
        })
        .catch((caught: unknown) => {
          status.textContent = `判定失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
        });

      return () => {
        root.textContent = '';
      };
    },
  };
}
