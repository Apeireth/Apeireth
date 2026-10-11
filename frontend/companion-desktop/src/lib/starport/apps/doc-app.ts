// 星笺 · 内置文档件（AppContribution，I 批 2 MVP）
//
// 契约：
//   1. 双栏面 —— 左源码（textarea）/ 右分段预览；预览只用 textContent
//      落字，零 innerHTML/零插值面（分段结构即渲染契约，杜绝 XSS）；
//   2. 渲染态直改链 —— 点段「编辑」→ verifyEditInvariant 通过才进编辑态
//      → 输入框预填 visualSlice（渲染态文本）→ 「提交」经 minimalSegmentEdit
//      归约出最小段内编辑，走 mapRenderedEdit 段内区间事务（段完整性 +
//      expectedLocal 校验，只替换真正变化的片段）→ 成功后重编译刷新；
//      失败如实亮错，不吞不假成功；
//   3. 可编程岛 —— 岛段渲染为**沙箱 iframe**（sandbox="allow-scripts"，
//      不带 allow-same-origin：岛跑在不透明源里，天然拿不到宿主 DOM/存储），
//      srcdoc 即 islandPayload 原样载荷；岛内脚本错误在岛内自显，宿主不吞
//      不代理。重编译/切页/卸载即**销毁 iframe**（销毁浏览上下文，岛内
//      定时器与动画帧随之强制释放——浏览器语义）+ IslandRegistry 对账；
//   4. 清理恰好一次 —— cleanup 整批释放岛 + 拔根节点复位容器，重挂载从
//      干净态起；
//   5. 0 装 —— 状态行只报真实结果（事务已提交 / 无改动 / 错误原因），不编造。
//
// MVP 边界：Markdown 正文 + 围栏代码块；段内增量编辑。扩展位
// （样式化渲染、版本分支/提案合入）随批接，见 doc-pipeline 契约。
//
// 注册：装配面（faces-install.ts，共享集成面）统一注册
//   registry.registerApp(createDocApp())
// 本文件只交定义，不触注册表（战场隔离）。

import type {AppContribution, AppMountContext} from '../contribution-registry';
import {
  compileDocument,
  verifyEditInvariant,
  visualSlice,
  mapRenderedEdit,
  minimalSegmentEdit,
  stableIslandId,
  islandPayload,
  IslandRegistry,
  type DocumentSegment,
  type CompiledDocument,
} from './doc-pipeline.ts';

export const STARPORT_DOC_APP_ID = 'starport-doc';

/** 首开样例真源（正文 / 围栏代码 / 可编程岛，覆盖三类分段）。 */
const INITIAL_SOURCE = [
  '# 星笺',
  '',
  '这是唯一真源文档的最小样例：左栏是源码，右栏是编译出的编辑段。',
  '',
  '```',
  '// 围栏代码块 —— 受保护区域，整块为一个编辑段',
  'fn main() {',
  '    println!("唯一真源");',
  '}',
  '```',
  '',
  '```island name=demo-island',
  '<div style="font-family: sans-serif"><b>可编程岛</b>：脚本只活在沙箱里，切页即销毁。</div>',
  '<script>setInterval(function () {}, 1000);</script>',
  '```',
  '',
  '正文段、代码段与岛屿段交替切分；点右栏「编辑」走区间事务提交。',
  '',
].join('\n');

interface EditDraft {
  segment: DocumentSegment;
  /** 进入编辑态时的渲染态文本（提交时最小段内编辑的比对基线）。 */
  expectedLocal: string;
}

export function createDocApp(): AppContribution {
  return {
    id: STARPORT_DOC_APP_ID,
    title: '星笺',
    kind: 'internal',
    mount: (container: unknown, _context: AppMountContext) => {
      const root = container as {appendChild?: (node: unknown) => unknown};
      if (typeof root?.appendChild !== 'function') {
        throw new Error('星笺挂载容器非法：缺 appendChild');
      }

      let source = INITIAL_SOURCE;
      let compiled: CompiledDocument = compileDocument(source);
      let draft: EditDraft | null = null;
      // 岛生命周期账本：重渲/切页/卸载整批强制释放（dispose 恰好一次）。
      const islands = new IslandRegistry();

      // —— 骨架（全部 document.createElement，预览落字只走 textContent） ——
      const panel = document.createElement('div');
      panel.className = 'starport-doc';
      const header = document.createElement('div');
      header.className = 'doc-header';
      const title = document.createElement('span');
      title.textContent = '星笺 · 唯一真源文档';
      const status = document.createElement('span');
      status.className = 'doc-status';
      status.textContent = '就绪（编译 ' + compiled.segments.length + ' 段）';
      header.appendChild(title);
      header.appendChild(status);

      const body = document.createElement('div');
      body.className = 'doc-body';
      const left = document.createElement('div');
      left.className = 'doc-left';
      const banner = document.createElement('div');
      banner.className = 'doc-banner';
      banner.style.display = 'none';
      const sourceArea = document.createElement('textarea');
      sourceArea.className = 'doc-source';
      sourceArea.readOnly = true;
      sourceArea.value = source;
      const actions = document.createElement('div');
      actions.className = 'doc-actions';
      const commitBtn = document.createElement('button');
      commitBtn.type = 'button';
      commitBtn.textContent = '事务提交';
      commitBtn.style.display = 'none';
      const abortBtn = document.createElement('button');
      abortBtn.type = 'button';
      abortBtn.textContent = '放弃';
      abortBtn.style.display = 'none';
      actions.appendChild(commitBtn);
      actions.appendChild(abortBtn);
      left.appendChild(banner);
      left.appendChild(sourceArea);
      left.appendChild(actions);

      const right = document.createElement('div');
      right.className = 'doc-preview';
      body.appendChild(left);
      body.appendChild(right);
      panel.appendChild(header);
      panel.appendChild(body);
      root.appendChild(panel);

      function setStatus(text: string, isError: boolean): void {
        status.textContent = text;
        status.style.color = isError ? '#e88' : '';
      }

      function renderPreview(): void {
        // 重渲前整批释放在册岛：销毁 iframe 即销毁其浏览上下文，岛内
        // 定时器/动画帧随之强制释放（浏览器语义，见文件头契约 §3）。
        islands.disposeAll();
        right.textContent = '';
        for (const segment of compiled.segments) {
          const block = document.createElement('div');
          block.className = 'doc-seg doc-seg-' + segment.kind;
          const islandId =
            segment.kind === 'island'
              ? stableIslandId(source, {start: segment.start, end: segment.end})
              : null;
          const kindLabel =
            segment.kind === 'code' ? '代码段' : segment.kind === 'island' ? '岛屿段' : '正文段';
          const label = document.createElement('div');
          label.className = 'doc-seg-label';
          label.textContent =
            segment.id + ' · ' + kindLabel + (islandId ? ' · ' + islandId : '') +
            ' · [' + segment.start + ', ' + segment.end + ') · ' + segment.hash;
          const editBtn = document.createElement('button');
          editBtn.type = 'button';
          editBtn.className = 'doc-seg-edit';
          editBtn.textContent = '编辑';
          editBtn.addEventListener('click', () => enterEdit(segment));
          block.appendChild(label);
          if (segment.kind === 'island' && islandId !== null) {
            // 沙箱 iframe：不带 allow-same-origin —— 岛跑在不透明源里，
            // 拿不到宿主 DOM/存储；srcdoc 即原样载荷，岛内脚本错误在岛内
            // 自显（宿主不吞不代理）。释放即移除 iframe（销毁上下文）。
            const frame = document.createElement('iframe');
            frame.className = 'doc-island-frame';
            frame.setAttribute('sandbox', 'allow-scripts');
            frame.srcdoc = islandPayload(segment);
            islands.register(islandId, () => {
              frame.remove();
            });
            block.appendChild(frame);
          } else {
            const contentEl = document.createElement('pre');
            contentEl.className = 'doc-seg-content';
            contentEl.textContent = segment.content;
            block.appendChild(contentEl);
          }
          block.appendChild(editBtn);
          right.appendChild(block);
        }
      }

  function enterEdit(segment: DocumentSegment): void {
        // 编辑不变量：临时编辑文本 ≡ 原源码片段，不成立拒绝进入（0 装）。
        if (!verifyEditInvariant(segment.content, source, {start: segment.start, end: segment.end})) {
          setStatus('拒绝进入编辑：编辑不变量不成立（索引与真源不符）。', true);
          return;
        }
        let local: string;
        try {
          // 渲染态文本进输入框（MVP 与源片段恒等；段过期即抛，不带病进编辑）。
          local = visualSlice(source, segment);
        } catch (caught) {
          setStatus('拒绝进入编辑：' + (caught instanceof Error ? caught.message : String(caught)), true);
          return;
        }
        draft = {segment, expectedLocal: local};
        sourceArea.readOnly = false;
        sourceArea.value = local;
        banner.style.display = '';
        banner.textContent =
          '编辑 ' + segment.id + '（渲染态直改：段内偏移映射回真源，提交时校验段完整性与原文）';
        commitBtn.style.display = '';
        abortBtn.style.display = '';
        setStatus('编辑态：' + segment.id, false);
      }

      function exitEdit(): void {
        draft = null;
        sourceArea.readOnly = true;
        sourceArea.value = source;
        banner.style.display = 'none';
        commitBtn.style.display = 'none';
        abortBtn.style.display = 'none';
      }

      function commitEdit(): void {
        if (!draft) return;
        const segment = draft.segment;
        const nextLocal = sourceArea.value;
        if (nextLocal === draft.expectedLocal) {
          exitEdit();
          setStatus('无改动（真源未动）。', false);
          return;
        }
        try {
          // 渲染态直改：最小段内编辑（前后缀裁剪，只留真正变化的片段）→
          // 段内区间事务（段完整性 + expectedLocal 校验，防并发写坏）。
          const localEdit = minimalSegmentEdit(draft.expectedLocal, nextLocal);
          source = mapRenderedEdit(source, compiled.segments, {
            segmentId: segment.id,
            ...localEdit,
          });
          compiled = compileDocument(source);
          exitEdit();
          renderPreview();
          setStatus('事务已提交：' + segment.id + '（重编译 ' + compiled.segments.length + ' 段）', false);
        } catch (caught) {
          // 失败如实亮错并停在编辑态（不吞错、不假成功）。
          setStatus('事务失败：' + (caught instanceof Error ? caught.message : String(caught)), true);
        }
      }

      commitBtn.addEventListener('click', commitEdit);
      abortBtn.addEventListener('click', () => {
        exitEdit();
        setStatus('已放弃编辑（真源未动）。', false);
      });

      renderPreview();

      // 清理恰好一次：整批释放在册岛（销毁 iframe 上下文）+ 拔根节点
      // 带走全部监听，重挂载从干净态起。
      return () => {
        draft = null;
        islands.disposeAll();
        panel.remove();
      };
    },
  };
}
