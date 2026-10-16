// 铭刻台 · 文本编辑内置件（星港内置应用面 I 批 2）
//
// 契约（I-starport-builtins.md「铭刻台」文本面）：
//   1. 贡献点契约 —— createTextEditorApp() 返回 AppContribution
//      （id 'starport-text'，title '铭刻台'，kind 'internal'），mount 只拿
//      容器与上下文，返回 cleanup；壳层关闭视图必走清理链；
//   2. 编辑面三件套 —— 行号栏 + 文本编辑区 + 查找栏（命中表按行:列外显）；
//   3. 会话生命周期 —— mount 创建文本会话（store 持久编辑内容），cleanup
//      删除会话并清空容器、幂等可重入（不留孤儿会话、不留孤儿 DOM）；
//   4. 错误即帧 —— 查找失败/校验拒绝外显到查找结果区，不吞错不假装；
//   5. 0 装 —— 无 DOM 环境且未注入 dom 端口即抛错；文本操作一律走
//      text-editor-store 纯逻辑（越界/不符在那层抛，本层不重造校验）。
//
// 测试：tests/starport-text-editor.mjs（注入 dom 桩直测挂载/清理链）。

import type {AppContribution, AppMountContext} from '../contribution-registry';
import {
  createTextEditorStore,
  findText,
  TextEditorError,
  type TextEditorStore,
} from './text-editor-store.ts';

export const STARPORT_TEXT_APP_ID = 'starport-text';
export const STARPORT_TEXT_APP_TITLE = '铭刻台';

/** 最小 DOM 端口（真实环境用全局 document，测试注入桩；本模块不锁定完整 DOM 类型）。 */
export interface TextEditorDom {
  createElement(tag: string): unknown;
}

/** mount 容器最小端口（真实元素天然满足）。 */
interface MountTarget {
  appendChild(node: unknown): unknown;
  textContent?: unknown;
}

/** 元素最小端口（真实元素天然满足；桩按需实现同名成员）。 */
interface ElementLike {
  className?: string;
  textContent?: string;
  value?: string;
  style?: Record<string, string>;
  appendChild(node: unknown): unknown;
  remove(): void;
  addEventListener(type: string, handler: (event: {target?: unknown}) => void): void;
}

export function createTextEditorApp(
  options: {dom?: TextEditorDom; store?: TextEditorStore} = {},
): AppContribution {
  return {
    id: STARPORT_TEXT_APP_ID,
    title: STARPORT_TEXT_APP_TITLE,
    kind: 'internal',
    mount: (container: unknown, context: AppMountContext) =>
      mountTextEditor(container, context, options),
  };
}

function mountTextEditor(
  container: unknown,
  context: AppMountContext,
  options: {dom?: TextEditorDom; store?: TextEditorStore},
): () => void {
  const dom: TextEditorDom | null =
    options.dom ?? (typeof document === 'undefined' ? null : (document as unknown as TextEditorDom));
  if (!dom) {
    throw new TextEditorError('invalid-argument', '无 DOM 环境：需注入 dom 端口');
  }
  const root = container as MountTarget;
  if (typeof root?.appendChild !== 'function') {
    throw new TextEditorError('invalid-argument', '容器不满足挂载端口（缺 appendChild）');
  }
  const store = options.store ?? createTextEditorStore();
  const session = store.create(`${STARPORT_TEXT_APP_TITLE}-${context.viewId}`);

  const el = (tag: string): ElementLike => dom.createElement(tag) as ElementLike;
  const wrapper = el('div');
  wrapper.className = 'starport-text-editor';

  // —— 行号栏（随编辑实时刷新） ——
  const gutter = el('div');
  gutter.className = 'starport-text-gutter';
  function refreshGutter(text: string): void {
    const total = text.split('\n').length;
    const numbers: string[] = [];
    for (let i = 1; i <= total; i += 1) numbers.push(String(i));
    gutter.textContent = numbers.join('\n');
  }

  // —— 文本编辑区 ——
  const area = el('textarea');
  area.className = 'starport-text-area';
  area.value = session.content;
  refreshGutter(session.content);
  area.addEventListener('input', (event) => {
    const target = (event.target ?? area) as ElementLike;
    const text = typeof target.value === 'string' ? target.value : '';
    const before = typeof area.value === 'string' ? area.value : '';
    area.value = text;
    refreshGutter(text);
    // 光标行从插入点前的换行数推算（1-based，契约 §1）。
    const caret = text.length >= before.length ? text.length : text.length;
    const cursorLine = text.slice(0, caret).split('\n').length;
    store.updateContent(session.id, text, cursorLine);
  });

  // —— 查找栏（命中表按 行:列 外显；失败即帧） ——
  const findBar = el('div');
  findBar.className = 'starport-text-find';
  const findInput = el('input');
  findInput.className = 'starport-text-find-input';
  const findButton = el('button');
  findButton.className = 'starport-text-find-btn';
  findButton.textContent = '查找';
  const findResult = el('span');
  findResult.className = 'starport-text-find-result';
  findButton.addEventListener('click', () => {
    const needle = typeof findInput.value === 'string' ? findInput.value : '';
    try {
      const hits = findText(area.value ?? '', needle);
      findResult.textContent =
        hits.length === 0
          ? '无命中'
          : `命中 ${hits.length}：${hits.map((hit) => `${hit.line}:${hit.column}`).join('、')}`;
    } catch (caught) {
      // 错误即帧（契约 §4）：校验拒绝/空查找词外显，不吞。
      const message = caught instanceof TextEditorError ? caught.message : String(caught);
      findResult.textContent = `查找失败：${message}`;
    }
  });
  findBar.appendChild(findInput);
  findBar.appendChild(findButton);
  findBar.appendChild(findResult);

  wrapper.appendChild(gutter);
  wrapper.appendChild(area);
  wrapper.appendChild(findBar);
  root.appendChild(wrapper);

  // —— 清理链：会话不留孤儿、DOM 不留孤儿、幂等可重入（契约 §3） ——
  let disposed = false;
  return () => {
    if (disposed) return;
    disposed = true;
    store.remove(session.id);
    wrapper.remove();
    if (typeof root.textContent === 'string' || root.textContent === undefined) {
      root.textContent = '';
    }
  };
}
