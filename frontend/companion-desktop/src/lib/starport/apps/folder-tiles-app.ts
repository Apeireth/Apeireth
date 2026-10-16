// 泊位磁贴 · 装配层（批次推进）
//
// 契约：
//   1. 真目录数据 —— 「浏览」走 Tauri list_workspace_dir 真实只读列举（原生侧
//      照实形态/隐藏不滤/错误带路径），栅格磁贴由纯件 buildTilesFromListing
//      归一（目录优先、id 可复算）；
//   2. 手动磁贴走纯件账本（增删/校验/展示序全在 folder-tiles-store 钉死）；
//   3. 映射持久化在本机 —— localStorage 适配器注入 TilesStore（键
//      'apeireth:starport:folder-tiles'），每次变更真落存；坏载荷如实报错帧，
//      不静默清库；
//   4. 本面只读列举 + 本地磁贴账本，不做写盘（写盘是原生侧后续批的契约缝）；
//   5. 桌面壳守卫 + 失败如实外显 + disposed 丢晚到结果 + cleanup 恰好一次。

import type {AppContribution} from '../contribution-registry';
import {
  TileError,
  TilesStore,
  serializeTileSnapshot,
  buildTilesFromListing,
  type TileGroup,
  type TilesPersistenceAdapter,
} from './folder-tiles-store.ts';

interface DirEntryInfo {
  name: string;
  is_dir: boolean;
  size_bytes: number;
}

interface DirListing {
  path: string;
  entries: DirEntryInfo[];
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

/** 拼接基路径与相对名（Windows 混分隔符可容）。 */
function joinPath(base: string, name: string): string {
  return `${base}${base.endsWith('/') || base.endsWith('\\') ? '' : '/'}${name}`;
}

export const FOLDER_TILES_STORAGE_KEY = 'apeireth:starport:folder-tiles';

/**
 * localStorage 持久化适配器（真落存）。
 * load：无记录 = 全新起步（null）；JSON 语法坏 = corrupt-persistence 如实抛。
 * save：稳定键序序列化真写入。
 */
function localStorageAdapter(): TilesPersistenceAdapter {
  return {
    load(): unknown {
      const raw = globalThis.localStorage?.getItem(FOLDER_TILES_STORAGE_KEY) ?? null;
      if (raw === null) return null;
      try {
        return JSON.parse(raw);
      } catch (caught) {
        throw new TileError(
          'corrupt-persistence',
          `磁贴载荷不是合法 JSON（如实报，不静默清库）：${caught instanceof Error ? caught.message : String(caught)}`,
        );
      }
    },
    save(groups: TileGroup[]): void {
      globalThis.localStorage?.setItem(FOLDER_TILES_STORAGE_KEY, serializeTileSnapshot(groups));
    },
  };
}

export function createFolderTilesApp(): AppContribution {
  return {
    id: 'starport-folders',
    title: '泊位磁贴',
    kind: 'internal',
    mount: (container: unknown) => {
      const root = container as HTMLElement;
      const doc = root.ownerDocument;
      let disposed = false;

      const note = el(doc, 'div', 'tiles-note', '真实目录列举（目录优先/名称序/隐藏照实）+ 写盘（建目录/改名/删——非空目录防误删拒绝）；磁贴映射持久化在本机（localStorage，键 apeireth:starport:folder-tiles）。');
      const bar = el(doc, 'div', 'tiles-bar');
      const pathInput = el(doc, 'input', 'tiles-path');
      pathInput.placeholder = '目录绝对路径';
      const browseBtn = el(doc, 'button', 'tiles-browse', '浏览');
      const mkdirInput = el(doc, 'input', 'tiles-mkdir-name');
      mkdirInput.placeholder = '新目录名';
      const mkdirBtn = el(doc, 'button', 'tiles-mkdir', '建目录');
      const renameToInput = el(doc, 'input', 'tiles-rename-to');
      renameToInput.placeholder = '改名目标名';
      bar.append(pathInput, browseBtn, mkdirInput, mkdirBtn, renameToInput);
      const status = el(doc, 'div', 'tiles-status');
      const grid = el(doc, 'div', 'tiles-grid');

      const manualForm = el(doc, 'div', 'tiles-manual-form');
      const nameInput = el(doc, 'input', 'tiles-name');
      nameInput.placeholder = '磁贴名称';
      const targetInput = el(doc, 'input', 'tiles-target');
      targetInput.placeholder = '目标（路径/网址/说明）';
      const kindSel = el(doc, 'select', 'tiles-kind');
      for (const kind of ['dir', 'file', 'link']) {
        const option = el(doc, 'option', undefined, kind);
        option.value = kind;
        kindSel.append(option);
      }
      const addBtn = el(doc, 'button', 'tiles-add', '加磁贴');
      manualForm.append(nameInput, targetInput, kindSel, addBtn);
      const manualHost = el(doc, 'div', 'tiles-manual');
      root.append(note, bar, status, grid, manualForm, manualHost);

      // 持久化账本：构造期坏载荷如实报错帧（不静默清库），交互面整体禁用。
      let store: TilesStore | null = null;
      try {
        const opened = new TilesStore({adapter: localStorageAdapter()});
        if (!opened.getGroups().some((group) => group.id === 'manual')) {
          opened.addGroup('manual', '手动磁贴');
        }
        store = opened;
      } catch (caught) {
        const code = caught instanceof TileError ? `[${caught.code}] ` : '';
        status.textContent = `磁贴载荷损坏（如实报，不静默清库）：${code}${caught instanceof Error ? caught.message : String(caught)}`;
        browseBtn.disabled = true;
        addBtn.disabled = true;
      }

      function renderManual(): void {
        manualHost.textContent = '';
        if (!store) return;
        for (const tileItem of store.listTiles('manual')) {
          const row = el(doc, 'div', 'tiles-row', `${tileItem.name} → ${tileItem.target}（${tileItem.kind}）`);
          const del = el(doc, 'button', 'tiles-del', '删除');
          del.addEventListener('click', () => {
            try {
              store?.removeTile('manual', tileItem.id);
              status.textContent = `已删磁贴（已落存）：${tileItem.name}`;
              renderManual();
            } catch (caught) {
              status.textContent = `操作失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
            }
          });
          row.append(del);
          manualHost.append(row);
        }
      }

      function renderGrid(base: string, listing: DirListing): void {
        grid.textContent = '';
        for (const tileItem of buildTilesFromListing(listing.entries)) {
          const cell = el(doc, 'button', `tiles-cell ${tileItem.kind}`, tileItem.name);
          if (tileItem.kind === 'dir') {
            // 目录磁贴即下钻（真目录浏览面）。
            cell.addEventListener('click', () => {
              pathInput.value = joinPath(base, tileItem.name);
              void browse();
            });
            // 写盘手势：改名（目标名走栏内 renameToInput）/ 删（非空目录原生侧拒）。
            const wrap = el(doc, 'div', 'tiles-cell-wrap');
            const renameBtn = el(doc, 'button', 'tiles-cell-act', '改名');
            renameBtn.addEventListener('click', () => {
              void (async () => {
                try {
                  const to = renameToInput.value.trim();
                  if (!to) {
                    status.textContent = '改名目标名为空（填「改名目标名」再点）。';
                    return;
                  }
                  await tauriInvoke('rename_entry', {path: base, from: tileItem.name, to});
                  status.textContent = `已改名：${tileItem.name} → ${to}`;
                  await browse();
                } catch (caught) {
                  if (disposed) return;
                  status.textContent = `改名失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
                }
              })();
            });
            const delBtn = el(doc, 'button', 'tiles-cell-act', '删');
            delBtn.addEventListener('click', () => {
              void (async () => {
                try {
                  await tauriInvoke('delete_entry', {path: base, name: tileItem.name});
                  status.textContent = `已删除：${tileItem.name}`;
                  await browse();
                } catch (caught) {
                  if (disposed) return;
                  status.textContent = `删除失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
                }
              })();
            });
            wrap.append(cell, renameBtn, delBtn);
            grid.append(wrap);
            continue;
          }
          grid.append(cell);
        }
      }

      async function browse(): Promise<void> {
        const target = pathInput.value.trim();
        if (!target) {
          status.textContent = '路径为空。';
          return;
        }
        status.textContent = '列举中…';
        try {
          const listing = await tauriInvoke<DirListing>('list_workspace_dir', {path: target});
          if (disposed) return;
          renderGrid(listing.path, listing);
          status.textContent = `${listing.entries.length} 个条目 · ${listing.path}`;
        } catch (caught) {
          if (disposed) return;
          status.textContent = `列举失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
        }
      }

      if (!isDesktop()) {
        status.textContent = '磁贴浏览与写盘需要桌面壳（当前为 Web 构建）——禁用，不设假按钮。';
        browseBtn.disabled = true;
        mkdirBtn.disabled = true;
      } else {
        browseBtn.addEventListener('click', () => void browse());
        mkdirBtn.addEventListener('click', () => {
          void (async () => {
            try {
              await tauriInvoke('create_dir_entry', {
                path: pathInput.value.trim(),
                name: mkdirInput.value,
              });
              status.textContent = `已建目录：${mkdirInput.value}`;
              mkdirInput.value = '';
              await browse();
            } catch (caught) {
              if (disposed) return;
              status.textContent = `建目录失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
            }
          })();
        });
      }

      addBtn.addEventListener('click', () => {
        try {
          if (!store) return;
          store.addTile('manual', {
            id: `m-${Date.now()}-${Math.floor(Math.random() * 1000)}`,
            name: nameInput.value,
            target: targetInput.value,
            kind: kindSel.value as 'dir' | 'file' | 'link',
          });
          nameInput.value = '';
          targetInput.value = '';
          status.textContent = '磁贴已入库并落存。';
          renderManual();
        } catch (caught) {
          const code = caught instanceof TileError ? `[${caught.code}] ` : '';
          status.textContent = `加磁贴失败（如实报）：${code}${caught instanceof Error ? caught.message : String(caught)}`;
        }
      });

      renderManual();
      return () => {
        disposed = true;
        root.textContent = '';
      };
    },
  };
}
