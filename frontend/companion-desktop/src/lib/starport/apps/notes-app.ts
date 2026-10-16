// 星屑便签 · 内置件（星港内置应用面 I 批 2）
//
// 契约（I-starport-builtins.md「星屑」）：
//   1. 贡献点契约件：id 'starport-notes'、title '星屑'、kind 'internal'，
//      mount(container, context) 只在容器内造 DOM，返回清理函数；
//   2. 0 装：便签内容一律 textContent 上屏（零 innerHTML，注册面防注入
//      纪律延伸到渲染面）；存储错误原文外显在编辑器错误行，不吞不粉饰；
//   3. 清理恰好一次：cleanup 先摘全部监听（disposables 链）再清根节点，
//      不留 DOM、不留监听、不动持久化数据（视图死，数据活）；
//   4. 存储为应用实例级单例（多视图共享同一份便签账本，同一持久化适配器）；
//   5. 持久化适配器经 options 注入（生产接本地存储层，测试接记录桩），
//      本模块不认介质（与 notes-store.ts 契约 §4 同源）。
//
// DOM 面刻意收窄（createElement/appendChild/textContent/value/hidden/
// addEventListener/removeEventListener/firstChild/removeChild/remove +
// ownerDocument），Node 侧以同构假 DOM 直测挂载与清理
// （tests/starport-notes.mjs）。

import type {AppContribution, AppMountContext} from '../contribution-registry';
import {
  NotesStore,
  NotesStoreError,
  type Note,
  type NotesPersistenceAdapter,
} from './notes-store.ts';

export const STARPORT_NOTES_APP_ID = 'starport-notes';

export interface NotesAppOptions {
  /** 持久化适配器（缺省 = 纯内存模式，视图关闭数据即散——如实如此）。 */
  adapter?: NotesPersistenceAdapter | null;
  /** 测试注入时钟；默认 Date.now。 */
  now?: () => number;
}

interface ElementLike {
  className: string;
  textContent: string;
  value: string;
  hidden: boolean;
  type: string;
  placeholder: string;
  firstChild: ElementLike | null;
  appendChild(child: ElementLike): ElementLike;
  removeChild(child: ElementLike): ElementLike;
  addEventListener(type: string, listener: () => void): void;
  removeEventListener(type: string, listener: () => void): void;
  remove(): void;
}

interface DocumentLike {
  createElement(tag: string): ElementLike;
}

const NOTE_STYLE = `
.stardust { display: flex; flex-direction: column; gap: 8px; font-family: ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, monospace; }
.stardust-bar { display: flex; align-items: center; justify-content: space-between; }
.stardust-heading { font-size: 11px; letter-spacing: 0.18em; color: rgba(232, 224, 204, 0.8); }
.stardust-btn { appearance: none; border: 1px solid rgba(255, 210, 122, 0.4); background: transparent; color: #ffd27a; font-size: 11px; border-radius: 6px; padding: 3px 10px; cursor: pointer; }
.stardust-btn.plain { border-color: rgba(232, 224, 204, 0.25); color: rgba(232, 224, 204, 0.85); }
.stardust-editor { display: flex; flex-direction: column; gap: 6px; padding: 8px; border: 1px dashed rgba(232, 224, 204, 0.2); border-radius: 8px; }
.stardust-input, .stardust-area { width: 100%; box-sizing: border-box; background: rgba(11, 13, 18, 0.6); border: 1px solid rgba(232, 224, 204, 0.2); border-radius: 6px; color: rgba(232, 224, 204, 0.9); font-size: 11px; padding: 4px 6px; }
.stardust-area { min-height: 72px; resize: vertical; white-space: pre-wrap; }
.stardust-actions { display: flex; align-items: center; gap: 6px; }
.stardust-error { font-size: 10px; color: rgba(255, 138, 128, 0.9); word-break: break-all; }
.stardust-list { display: flex; flex-direction: column; gap: 6px; }
.stardust-empty { font-size: 10px; color: rgba(232, 224, 204, 0.45); letter-spacing: 0.12em; }
.stardust-note { border: 1px solid rgba(232, 224, 204, 0.16); border-radius: 8px; padding: 6px 8px; display: flex; flex-direction: column; gap: 4px; }
.stardust-note.pinned { border-color: rgba(255, 210, 122, 0.45); }
.stardust-note-title { font-size: 11px; color: #ffd27a; word-break: break-all; }
.stardust-note-body { font-size: 11px; color: rgba(232, 224, 204, 0.85); white-space: pre-wrap; word-break: break-word; }
.stardust-note-actions { display: flex; gap: 6px; }
`;

export function createNotesApp(options: NotesAppOptions = {}): AppContribution {
  const now = options.now ?? (() => Date.now());
  // 应用实例级存储：多视图共享一份账本（契约 §4）。
  const store = new NotesStore({adapter: options.adapter ?? null});

  return {
    id: STARPORT_NOTES_APP_ID,
    title: '星屑',
    kind: 'internal',
    mount: (container: unknown, _context: AppMountContext) => {
      const host = container as ElementLike;
      const doc: DocumentLike =
        (container as {ownerDocument?: DocumentLike}).ownerDocument ??
        ((globalThis as {document?: DocumentLike}).document as DocumentLike);
      const disposables: Array<() => void> = [];
      let cleaned = false;
      let editingId: string | null = null;
      let idSeq = 0;

      const on = (el: ElementLike, type: string, listener: () => void): void => {
        el.addEventListener(type, listener);
        disposables.push(() => el.removeEventListener(type, listener));
      };

      const el = (tag: string, className: string, text?: string): ElementLike => {
        const node = doc.createElement(tag);
        node.className = className;
        if (text !== undefined) node.textContent = text;
        return node;
      };

      const root = el('div', 'stardust');
      const style = doc.createElement('style');
      style.textContent = NOTE_STYLE;
      root.appendChild(style);

      const bar = el('div', 'stardust-bar');
      bar.appendChild(el('span', 'stardust-heading', '星屑便签'));
      const newBtn = el('button', 'stardust-btn', '新建');
      newBtn.type = 'button';
      bar.appendChild(newBtn);
      root.appendChild(bar);

      const editor = el('div', 'stardust-editor');
      editor.hidden = true;
      const titleInput = el('input', 'stardust-input');
      titleInput.type = 'text';
      titleInput.placeholder = '标题（可空）';
      const bodyInput = el('textarea', 'stardust-area');
      bodyInput.placeholder = '内容（与标题至少填一）';
      const actions = el('div', 'stardust-actions');
      const saveBtn = el('button', 'stardust-btn', '存');
      saveBtn.type = 'button';
      const cancelBtn = el('button', 'stardust-btn plain', '取消');
      cancelBtn.type = 'button';
      const errorLine = el('span', 'stardust-error', '');
      actions.appendChild(saveBtn);
      actions.appendChild(cancelBtn);
      actions.appendChild(errorLine);
      editor.appendChild(titleInput);
      editor.appendChild(bodyInput);
      editor.appendChild(actions);
      root.appendChild(editor);

      const listEl = el('div', 'stardust-list');
      root.appendChild(listEl);
      host.appendChild(root);

      function showError(message: string): void {
        errorLine.textContent = message;
      }

      function renderList(): void {
        let child = listEl.firstChild;
        while (child) {
          listEl.removeChild(child);
          child = listEl.firstChild;
        }
        const notes = store.list();
        if (notes.length === 0) {
          listEl.appendChild(el('div', 'stardust-empty', '暂无便签'));
          return;
        }
        for (const note of notes) {
          listEl.appendChild(renderNoteCard(note));
        }
      }

      function renderNoteCard(note: Note): ElementLike {
        const card = el('div', note.pinned ? 'stardust-note pinned' : 'stardust-note');
        card.appendChild(el('div', 'stardust-note-title', note.title || '（无题）'));
        card.appendChild(el('div', 'stardust-note-body', note.body));
        const cardActions = el('div', 'stardust-note-actions');
        const pinBtn = el('button', 'stardust-btn plain', note.pinned ? '取消置顶' : '置顶');
        pinBtn.type = 'button';
        const editBtn = el('button', 'stardust-btn plain', '编辑');
        editBtn.type = 'button';
        const deleteBtn = el('button', 'stardust-btn plain', '删除');
        deleteBtn.type = 'button';
        cardActions.appendChild(pinBtn);
        cardActions.appendChild(editBtn);
        cardActions.appendChild(deleteBtn);
        card.appendChild(cardActions);

        on(pinBtn, 'click', () => {
          try {
            store.togglePin(note.id);
            renderList();
          } catch (caught) {
            showError(errorMessage(caught));
          }
        });
        on(editBtn, 'click', () => {
          editingId = note.id;
          titleInput.value = note.title;
          bodyInput.value = note.body;
          errorLine.textContent = '';
          editor.hidden = false;
        });
        on(deleteBtn, 'click', () => {
          try {
            store.remove(note.id);
            if (editingId === note.id) {
              editingId = null;
              editor.hidden = true;
            }
            renderList();
          } catch (caught) {
            showError(errorMessage(caught));
          }
        });
        return card;
      }

      function openNew(): void {
        editingId = null;
        titleInput.value = '';
        bodyInput.value = '';
        errorLine.textContent = '';
        editor.hidden = false;
      }

      function save(): void {
        try {
          const title = titleInput.value;
          const body = bodyInput.value;
          if (editingId === null) {
            idSeq += 1;
            store.add({id: `note-${now()}-${idSeq}`, title, body}, now());
          } else {
            store.update(editingId, {title, body}, now());
          }
          editingId = null;
          editor.hidden = true;
          errorLine.textContent = '';
          renderList();
        } catch (caught) {
          // 0 装：校验错误原文外显，不吞、不粉饰（契约 §2）。
          showError(errorMessage(caught));
        }
      }

      on(newBtn, 'click', openNew);
      on(saveBtn, 'click', save);
      on(cancelBtn, 'click', () => {
        editingId = null;
        editor.hidden = true;
        errorLine.textContent = '';
      });

      renderList();

      return () => {
        if (cleaned) return;
        cleaned = true;
        for (const dispose of disposables.splice(0)) dispose();
        root.remove();
      };
    },
  };
}

function errorMessage(caught: unknown): string {
  return caught instanceof NotesStoreError
    ? `便签拒绝执行：${caught.message}`
    : `便签操作失败：${caught instanceof Error ? caught.message : String(caught)}`;
}
