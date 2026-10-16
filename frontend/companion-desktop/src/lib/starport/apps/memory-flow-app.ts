// 深流 · 记忆压缩检索面装配层（星港内置应用面）
//
// 契约：
//   1. 行为在纯件（memory-flow-store.ts，5 组测试钉死）；本文件只做 DOM
//      装配与真实通道接线（接线事实由源镜像面钉住）；
//   2. 真实通道 —— 「沉淀入记忆」走契约 §5 POST /v1/memory/append（复用
//      runtime.appendMemoryEpisode，面板账本自管 session）；「从记忆召回」
//      走 GET /v1/panel/memory/episodes（复用 runtime.fetchMemoryEpisodes）；
//   3. 失败如实外显（写入返回 false / 查询抛错都进错误帧，不假成功）；
//   4. 清理链 —— 卸载即置 disposed（runtime 调用面暂不支持中止信号，晚到
//      结果按 disposed 丢弃，如实声明）+ 清 DOM。
//
// 装配层由 StarportPanel（Vite 构建面）加载；纯件被 Node 直测。

import type {ApeirethConfig} from '../../types';
import type {AppContribution} from '../contribution-registry';
import {appendMemoryEpisode, fetchMemoryEpisodes} from '../../runtime';
import {
  appendObservation,
  removeObservation,
  listObservations,
  compressObservations,
  rankRecall,
  buildMemoryContent,
  OBSERVATION_KINDS,
  type Observation,
} from './memory-flow-store.ts';

const COMPRESS_BUDGET = 8;

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

export function createMemoryFlowApp(options: {config: ApeirethConfig}): AppContribution {
  const {config} = options;
  return {
    id: 'starport-memory',
    title: '深流',
    kind: 'internal',
    mount: (container: unknown) => {
      const root = container as HTMLElement;
      const doc = root.ownerDocument;
      let disposed = false;
      let items: Observation[] = [];
      let seq = 0;

      const note = el(doc, 'div', 'memflow-note', '深流切片：本地账本+压缩+召回；沉淀/召回走真实记忆通道。');
      // —— 录入表单 ——
      const form = el(doc, 'div', 'memflow-form');
      const kindSel = el(doc, 'select', 'memflow-kind');
      for (const kind of OBSERVATION_KINDS) {
        const option = el(doc, 'option', undefined, kind);
        option.value = kind;
        kindSel.append(option);
      }
      const textInput = el(doc, 'input', 'memflow-text');
      textInput.placeholder = '观察内容（决策/修复/安全/笔记）';
      const sourceInput = el(doc, 'input', 'memflow-source');
      sourceInput.placeholder = '来源（工具/事件名，可空）';
      const addBtn = el(doc, 'button', 'memflow-add', '记录');
      form.append(kindSel, textInput, sourceInput, addBtn);
      // —— 过滤栏 ——
      const filterBar = el(doc, 'div', 'memflow-filter');
      const filterKind = el(doc, 'select', 'memflow-filter-kind');
      const anyOption = el(doc, 'option', undefined, '全部 kind');
      anyOption.value = '';
      filterKind.append(anyOption);
      for (const kind of OBSERVATION_KINDS) {
        const option = el(doc, 'option', undefined, kind);
        option.value = kind;
        filterKind.append(option);
      }
      const filterText = el(doc, 'input', 'memflow-filter-text');
      filterText.placeholder = '文本过滤';
      filterBar.append(filterKind, filterText);
      // —— 账本列表 ——
      const listHost = el(doc, 'div', 'memflow-list');
      // —— 压缩视图 ——
      const foldBar = el(doc, 'div', 'memflow-fold-bar');
      const foldBtn = el(doc, 'button', 'memflow-fold', '折叠要点');
      const foldOut = el(doc, 'pre', 'memflow-fold-out');
      foldBar.append(foldBtn, foldOut);
      // —— 召回框 ——
      const recallBar = el(doc, 'div', 'memflow-recall');
      const recallInput = el(doc, 'input', 'memflow-recall-query');
      recallInput.placeholder = '召回词项（空白分隔，多词 OR）';
      const recallLocal = el(doc, 'button', 'memflow-recall-local', '本地召回');
      const recallRemote = el(doc, 'button', 'memflow-recall-remote', '从记忆召回');
      const recallOut = el(doc, 'pre', 'memflow-recall-out');
      recallBar.append(recallInput, recallLocal, recallRemote, recallOut);
      root.append(note, form, filterBar, listHost, foldBar, recallBar);

      function fail(message: string): void {
        note.textContent = message;
      }

      function renderList(): void {
        const kindFilter = (filterKind.value || undefined) as string | undefined;
        const visible = listObservations(items, {
          kind: kindFilter,
          text: filterText.value || undefined,
        });
        listHost.textContent = '';
        listHost.append(el(doc, 'div', 'memflow-count', `账本 ${items.length} 条（视图 ${visible.length}）`));
        for (const item of visible) {
          const row = el(doc, 'div', 'memflow-row');
          row.append(
            el(doc, 'span', 'memflow-row-kind', item.kind),
            el(doc, 'span', 'memflow-row-text', item.text),
            el(doc, 'span', 'memflow-row-source', item.source || '—'),
          );
          const sinkBtn = el(doc, 'button', 'memflow-sink', '沉淀');
          sinkBtn.addEventListener('click', () => void sinkOne(item));
          const delBtn = el(doc, 'button', 'memflow-del', '删除');
          delBtn.addEventListener('click', () => {
            try {
              items = removeObservation(items, item.id);
              renderList();
            } catch (caught) {
              fail(`删除失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`);
            }
          });
          row.append(sinkBtn, delBtn);
          listHost.append(row);
        }
      }

      async function sinkOne(item: Observation): Promise<void> {
        try {
          const ok = await appendMemoryEpisode(config, buildMemoryContent(item));
          if (disposed) return;
          fail(ok ? `已沉淀：${item.text.slice(0, 24)}` : '沉淀失败（通道返回未写入，如实报）。');
        } catch (caught) {
          if (disposed) return;
          fail(`沉淀失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`);
        }
      }

      addBtn.addEventListener('click', () => {
        try {
          seq += 1;
          items = appendObservation(items, {
            id: `obs-${Date.now()}-${seq}`,
            at: Date.now(),
            kind: kindSel.value,
            text: textInput.value,
            source: sourceInput.value,
          });
          textInput.value = '';
          note.textContent = '已记入账本（沉淀入记忆后持久化）。';
          renderList();
        } catch (caught) {
          fail(`记录失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`);
        }
      });

      foldBtn.addEventListener('click', () => {
        try {
          const {lines, dropped} = compressObservations(items, COMPRESS_BUDGET);
          foldOut.textContent =
            lines.join('\n') +
            (dropped > 0 ? `\n（超预算丢最旧 ${dropped} 条，如实报）` : '');
        } catch (caught) {
          fail(`折叠失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`);
        }
      });

      recallLocal.addEventListener('click', () => {
        const ranked = rankRecall(items, recallInput.value);
        recallOut.textContent = ranked
          .slice(0, 10)
          .map((item) => `${item.kind} ${item.text}`)
          .join('\n');
      });

      recallRemote.addEventListener('click', () => {
        void (async () => {
          try {
            const episodes = await fetchMemoryEpisodes(config, recallInput.value, 20);
            if (disposed) return;
            recallOut.textContent = episodes.length
              ? episodes.map((episode) => episode.content).join('\n')
              : '（记忆通道无命中）';
          } catch (caught) {
            if (disposed) return;
            fail(`记忆召回失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`);
          }
        })();
      });

      renderList();
      return () => {
        disposed = true;
        root.textContent = '';
      };
    },
  };
}
