// 星港 · 面应用装配集（I 批 3–5，DOM 装配层）
//
// 契约：
//   1. 装配层薄 —— 行为在各面纯逻辑件（browse-store/chart-store/organize-store/
//      lifeflow-store），本文件只做 DOM 装配与事件接线（纯逻辑件各自测试钉死，
//      装配层走源镜像测试钉接线事实）；
//   2. mount 契约同星港注册表：mount(container, context) 返回 cleanup（清理
//      恰好一次：移除 DOM + 解绑监听 + 中止在途请求）；
//   3. 深望准入第一闸在装配层强制走（normalizeBrowseUrl + checkBrowsePolicy），
//      页面内不设旁路；
//   4. 通译走真实模型通道（streamChat），失败如实外显错误帧，不假成功。
//
// 注：值导入带 .ts 后缀（Node ESM 直测解析所需，仓库成例）；本文件由
// StarportPanel（Vite 构建面）加载，源镜像测试见 tests/starport-faces.mjs 系。

import type {AppContribution} from '../contribution-registry';
import type {ContributionRegistry} from '../contribution-registry';
import type {ApeirethConfig} from '../../types';
import {streamChat} from '../../runtime';
import {
  normalizeBrowseUrl,
  checkBrowsePolicy,
  extractReadableText,
  pushHistory,
  extractSemanticTree,
  type BrowseHistoryEntry,
  type BrowsePolicy,
} from './browse-store.ts';
import {computeLineGeometry, computeBarGeometry, type ChartData} from './chart-store.ts';
import {
  createGroup,
  addShortcut,
  removeShortcut,
  type Group,
} from './organize-store.ts';
import {
  addSource,
  addItems,
  markRead,
  countUnread,
  listItems,
  type FeedSource,
  type FeedItem,
} from './lifeflow-store.ts';

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

// ---------------------------------------------------------------------------
// 深望 · 浏览面（MVP 切片：准入闸 + 内嵌视图 + 抓取转文本）
// ---------------------------------------------------------------------------
export function createBrowseApp(options: {policy?: BrowsePolicy} = {}): AppContribution {
  const policy: BrowsePolicy = options.policy ?? {allowedHosts: null};
  return {
    id: 'starport-browse',
    title: '深望',
    kind: 'internal',
    mount: (container: unknown) => {
      const root = container as HTMLElement;
      const doc = root.ownerDocument;
      const bar = el(doc, 'div', 'browse-bar');
      const input = el(doc, 'input', 'browse-url');
      input.type = 'text';
      input.placeholder = 'https://…（只放行 HTTP/HTTPS）';
      const go = el(doc, 'button', 'browse-go', '打开');
      const fetchBtn = el(doc, 'button', 'browse-fetch', '抓取转文本');
      const semanticBtn = el(doc, 'button', 'browse-semantic', '语义树');
      const note = el(doc, 'div', 'browse-note', '准入闸：仅 HTTP/HTTPS；抓取为 MVP 粗提取；语义句柄供操控内核（智能体放行等安全纵深契约）。');
      const frame = el(doc, 'iframe', 'browse-frame');
      frame.setAttribute('sandbox', 'allow-scripts allow-same-origin allow-forms');
      const textPane = el(doc, 'pre', 'browse-text');
      const treePane = el(doc, 'div', 'browse-tree');
      const historyList = el(doc, 'div', 'browse-history');
      bar.append(input, go, fetchBtn, semanticBtn);
      root.append(bar, note, frame, textPane, treePane, historyList);

      let history: BrowseHistoryEntry[] = [];
      let lastHtml = '';
      const controller = new AbortController();

      function gate(): {href: string} | null {
        const target = normalizeBrowseUrl(input.value);
        if (!target) {
          note.textContent = '拒绝：地址非法或协议不在 HTTP/HTTPS。';
          return null;
        }
        const verdict = checkBrowsePolicy(target, policy);
        if (!verdict.ok) {
          note.textContent = `拒绝：${verdict.reason}`;
          return null;
        }
        return target;
      }

      function renderHistory(): void {
        historyList.textContent = '';
        for (const item of history) {
          historyList.append(el(doc, 'div', 'browse-history-item', `${item.host} — ${item.title ?? item.href}`));
        }
      }

      const open = (): void => {
        const target = gate();
        if (!target) return;
        frame.src = target.href;
        history = pushHistory(history, {
          href: target.href,
          host: normalizeBrowseUrl(target.href)?.host ?? '',
          at: Date.now(),
          title: null,
        });
        note.textContent = `已打开：${target.href}`;
        renderHistory();
      };
      const fetchText = async (): Promise<void> => {
        const target = gate();
        if (!target) return;
        note.textContent = '抓取中…（跨域受限时如实报错，不假装成功）';
        try {
          const res = await fetch(target.href, {signal: controller.signal});
          const html = await res.text();
          lastHtml = html;
          textPane.textContent = extractReadableText(html);
          history = pushHistory(history, {
            href: target.href,
            host: normalizeBrowseUrl(target.href)?.host ?? '',
            at: Date.now(),
            title: target.href,
          });
          note.textContent = `抓取完成（HTTP ${res.status}）：${target.href}`;
          renderHistory();
        } catch (caught) {
          note.textContent = `抓取失败（跨域/网络，如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
        }
      };
      const renderTree = (): void => {
        if (!lastHtml) {
          note.textContent = '先抓取页面再提取语义树（无真源不提取）。';
          return;
        }
        const tree = extractSemanticTree(lastHtml);
        treePane.textContent = '';
        for (const node of tree) {
          const row = el(doc, 'div', 'browse-tree-row', `${node.handle} · ${node.kind} · ${node.text}`);
          treePane.append(row);
        }
        note.textContent = `语义树：${tree.length} 个节点（句柄确定性命名；智能体操控放行等安全纵深契约）。`;
      };
      go.addEventListener('click', open);
      fetchBtn.addEventListener('click', () => void fetchText());
      semanticBtn.addEventListener('click', renderTree);

      return () => {
        controller.abort();
        go.removeEventListener('click', open);
        frame.src = 'about:blank';
        root.textContent = '';
      };
    },
  };
}

// ---------------------------------------------------------------------------
// 星仪 · 图表面（MVP 切片：数据驱动折线/柱状 SVG）
// ---------------------------------------------------------------------------
export function createChartApp(): AppContribution {
  return {
    id: 'starport-chart',
    title: '星仪',
    kind: 'internal',
    mount: (container: unknown) => {
      const root = container as HTMLElement;
      const doc = root.ownerDocument;
      const editor = el(doc, 'textarea', 'chart-data');
      editor.value = JSON.stringify(
        {
          labels: ['一', '二', '三', '四'],
          series: [
            {name: '流入', points: [3, 7, 5, 9]},
            {name: '流出', points: [6, 2, 8, 4]},
          ],
        },
        null,
        2,
      );
      const renderBtn = el(doc, 'button', 'chart-render', '渲染');
      const note = el(doc, 'div', 'chart-note', '数据驱动：图形由数据算出（折线/柱状同源几何）。');
      const svgHost = el(doc, 'div', 'chart-svg');
      const barBtn = el(doc, 'button', 'chart-bars', '柱状');
      const lineBtn = el(doc, 'button', 'chart-lines', '折线');
      root.append(editor, barBtn, lineBtn, renderBtn, note, svgHost);

      function render(mode: 'line' | 'bar'): void {
        try {
          const data = JSON.parse(editor.value) as ChartData;
          const frame = {width: 480, height: 220, padding: 24};
          const ns = 'http://www.w3.org/2000/svg';
          const svg = doc.createElementNS(ns, 'svg');
          svg.setAttribute('width', String(frame.width));
          svg.setAttribute('height', String(frame.height));
          if (mode === 'line') {
            const geo = computeLineGeometry(data, frame);
            for (const path of geo.paths) {
              const node = doc.createElementNS(ns, 'path');
              node.setAttribute('d', path.d);
              node.setAttribute('fill', 'none');
              node.setAttribute('stroke', '#ffd27a');
              node.setAttribute('stroke-width', '1.5');
              svg.append(node);
            }
            note.textContent = `折线：值域 [${geo.min}, ${geo.max}]，${geo.paths.length} 条序列。`;
          } else {
            const geo = computeBarGeometry(data, frame);
            for (const bar of geo.bars) {
              const node = doc.createElementNS(ns, 'rect');
              node.setAttribute('x', String(bar.x));
              node.setAttribute('y', String(bar.y));
              node.setAttribute('width', String(Math.max(0, bar.width - 1)));
              node.setAttribute('height', String(bar.height));
              node.setAttribute('fill', 'rgba(255, 210, 122, 0.6)');
              svg.append(node);
            }
            note.textContent = `柱状：值域 [${geo.min}, ${geo.max}]，${geo.bars.length} 根柱。`;
          }
          svgHost.textContent = '';
          svgHost.append(svg);
        } catch (caught) {
          note.textContent = `数据非法（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
        }
      }

      const onLine = (): void => render('line');
      const onBar = (): void => render('bar');
      lineBtn.addEventListener('click', onLine);
      barBtn.addEventListener('click', onBar);
      renderBtn.addEventListener('click', onLine);
      render('line');

      return () => {
        lineBtn.removeEventListener('click', onLine);
        barBtn.removeEventListener('click', onBar);
        root.textContent = '';
      };
    },
  };
}

// ---------------------------------------------------------------------------
// 泊位 · 收纳面（MVP 切片：分区/快捷位管理；文件映射磁贴待原生面）
// ---------------------------------------------------------------------------
export function createOrganizeApp(): AppContribution {
  return {
    id: 'starport-organize',
    title: '泊位',
    kind: 'internal',
    mount: (container: unknown) => {
      const root = container as HTMLElement;
      const doc = root.ownerDocument;
      let groups: Group[] = [createGroup('g-work', '工作'), createGroup('g-life', '生活')];
      const note = el(doc, 'div', 'organize-note', '收纳分区 MVP：真实文件夹映射磁贴待原生文件面接线（在册扩展位）。');
      const listHost = el(doc, 'div', 'organize-list');
      const addBar = el(doc, 'div', 'organize-add');
      const groupSel = el(doc, 'select', 'organize-group');
      const nameInput = el(doc, 'input', 'organize-name');
      nameInput.placeholder = '快捷位名称';
      const targetInput = el(doc, 'input', 'organize-target');
      targetInput.placeholder = '目标（路径/网址/说明）';
      const addBtn = el(doc, 'button', 'organize-add-btn', '入库');
      addBar.append(groupSel, nameInput, targetInput, addBtn);
      root.append(note, listHost, addBar);

      function render(): void {
        groupSel.textContent = '';
        for (const group of groups) {
          const option = el(doc, 'option', undefined, group.name);
          option.value = group.id;
          groupSel.append(option);
        }
        listHost.textContent = '';
        for (const group of groups) {
          const section = el(doc, 'div', 'organize-group-block');
          section.append(el(doc, 'div', 'organize-group-title', group.name));
          for (const shortcut of group.shortcuts) {
            const row = el(doc, 'div', 'organize-row', `${shortcut.name} → ${shortcut.target}`);
            const del = el(doc, 'button', 'organize-del', '删除');
            del.addEventListener('click', () => {
              try {
                groups = removeShortcut(groups, group.id, shortcut.id);
                render();
              } catch (caught) {
                note.textContent = `操作失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
              }
            });
            row.append(del);
            section.append(row);
          }
          listHost.append(section);
        }
      }

      addBtn.addEventListener('click', () => {
        try {
          groups = addShortcut(groups, groupSel.value, {
            id: `s-${Date.now()}-${Math.floor(Math.random() * 1000)}`,
            name: nameInput.value,
            target: targetInput.value,
          });
          nameInput.value = '';
          targetInput.value = '';
          note.textContent = '收纳分区 MVP：真实文件夹映射磁贴待原生文件面接线（在册扩展位）。';
          render();
        } catch (caught) {
          note.textContent = `入库失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
        }
      });
      render();
      // 跨组移动 API（organize-store.moveShortcut）在册且纯件测试钉死，
      // UI 手势（拖拽/菜单）为后续批扩展位。
      return () => {
        root.textContent = '';
      };
    },
  };
}

// ---------------------------------------------------------------------------
// 信风 · 生活信息流（J MVP 切片：源登记 + 条目流 + 未读计数）
// ---------------------------------------------------------------------------
export function createLifeflowApp(): AppContribution {
  return {
    id: 'starport-lifeflow',
    title: '信风',
    kind: 'internal',
    mount: (container: unknown) => {
      const root = container as HTMLElement;
      const doc = root.ownerDocument;
      let sources: FeedSource[] = [];
      let items: FeedItem[] = [];
      const note = el(doc, 'div', 'lifeflow-note', '信风 MVP：源登记与条目流；多源抓取适配待深望抓取面扩批。');
      const sourceBar = el(doc, 'div', 'lifeflow-sources');
      const nameInput = el(doc, 'input', 'lifeflow-name');
      nameInput.placeholder = '信息源名称';
      const urlInput = el(doc, 'input', 'lifeflow-url');
      urlInput.placeholder = 'https://…（只收 HTTP/HTTPS）';
      const addBtn = el(doc, 'button', 'lifeflow-add', '登记信息源');
      const itemBar = el(doc, 'div', 'lifeflow-items-add');
      const itemTitle = el(doc, 'input', 'lifeflow-item-title');
      itemTitle.placeholder = '条目标题（手动录入位；自动抓取待扩批）';
      const addItemBtn = el(doc, 'button', 'lifeflow-item-add', '入库条目');
      const listHost = el(doc, 'div', 'lifeflow-list');
      sourceBar.append(nameInput, urlInput, addBtn);
      itemBar.append(itemTitle, addItemBtn);
      root.append(note, sourceBar, itemBar, listHost);

      function render(): void {
        listHost.textContent = '';
        listHost.append(el(doc, 'div', 'lifeflow-count', `未读 ${countUnread(items)} 条 / 源 ${sources.length} 个`));
        for (const item of listItems(items)) {
          const row = el(doc, 'div', 'lifeflow-row', `${item.read ? '✓ ' : '● '}${item.title}`);
          row.addEventListener('click', () => {
            items = markRead(items, item.id, !item.read);
            render();
          });
          listHost.append(row);
        }
      }

      addBtn.addEventListener('click', () => {
        try {
          sources = addSource(sources, {
            id: `src-${Date.now()}`,
            name: nameInput.value,
            url: urlInput.value,
          });
          nameInput.value = '';
          urlInput.value = '';
          render();
        } catch (caught) {
          note.textContent = `登记失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
        }
      });
      addItemBtn.addEventListener('click', () => {
        try {
          if (!sources.length) throw new Error('先登记信息源');
          items = addItems(items, [
            {
              id: `item-${Date.now()}`,
              sourceId: sources[0].id,
              title: itemTitle.value,
              summary: null,
              at: Date.now(),
              read: true,
            },
          ]);
          itemTitle.value = '';
          render();
        } catch (caught) {
          note.textContent = `入库失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
        }
      });
      render();
      return () => {
        root.textContent = '';
      };
    },
  };
}

// ---------------------------------------------------------------------------
// 通译 · 翻译面（MVP 切片：真实模型通道直译）
// ---------------------------------------------------------------------------
export function createTranslateApp(options: {config: ApeirethConfig}): AppContribution {
  return {
    id: 'starport-translate',
    title: '通译',
    kind: 'internal',
    mount: (container: unknown) => {
      const root = container as HTMLElement;
      const doc = root.ownerDocument;
      const input = el(doc, 'textarea', 'translate-input');
      input.placeholder = '待译文本';
      const langInput = el(doc, 'input', 'translate-lang');
      langInput.value = '英文';
      const runBtn = el(doc, 'button', 'translate-run', '翻译');
      const note = el(doc, 'div', 'translate-note', '走真实模型通道；失败如实外显，不假成功。');
      const output = el(doc, 'pre', 'translate-output');
      root.append(langInput, input, runBtn, note, output);

      const controller = new AbortController();
      const run = async (): Promise<void> => {
        const text = input.value.trim();
        if (!text) {
          note.textContent = '待译文本为空。';
          return;
        }
        note.textContent = '翻译中…';
        output.textContent = '';
        try {
          const result = await streamChat(
            options.config,
            [
              {
                role: 'system',
                content: `你是翻译器。把用户输入翻译成${langInput.value || '目标语言'}，只输出译文。`,
              },
              {role: 'user', content: text},
            ],
            {onDelta: (delta) => (output.textContent = (output.textContent ?? '') + delta)},
            controller.signal,
          );
          if (!output.textContent) output.textContent = result;
          note.textContent = '完成。';
        } catch (caught) {
          note.textContent = `翻译失败（如实报）：${caught instanceof Error ? caught.message : String(caught)}`;
        }
      };
      runBtn.addEventListener('click', () => void run());
      return () => {
        controller.abort();
        root.textContent = '';
      };
    },
  };
}

// ---------------------------------------------------------------------------
// 船坞 · 生态分发面（MVP 切片：已装贡献点清单 = 注册表真值）
// ---------------------------------------------------------------------------
export function createEcosystemApp(options: {registry: ContributionRegistry}): AppContribution {
  return {
    id: 'starport-ecosystem',
    title: '船坞',
    kind: 'internal',
    mount: (container: unknown) => {
      const root = container as HTMLElement;
      const doc = root.ownerDocument;
      const note = el(doc, 'div', 'ecosystem-note', '已装贡献点清单（注册表真值）；插件/技能/主题市场待后端分发面。');
      const listHost = el(doc, 'div', 'ecosystem-list');
      root.append(note, listHost);
      const refresh = (): void => {
        listHost.textContent = '';
        for (const app of options.registry.listApps()) {
          listHost.append(el(doc, 'div', 'ecosystem-app', `应用：${app.title}（${app.id}）`));
        }
        for (const cmd of options.registry.listCommands()) {
          listHost.append(el(doc, 'div', 'ecosystem-cmd', `命令：${cmd.title}（${cmd.id}）`));
        }
      };
      refresh();
      return () => {
        root.textContent = '';
      };
    },
  };
}
