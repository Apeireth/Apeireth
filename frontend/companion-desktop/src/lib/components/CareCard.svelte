<!-- 关怀卡（V3-a 双引擎关怀上屏）：关怀规则列表（四类触发器/参数/最近触发/下次
     触发）+ 打扰纪律（勿扰窗口/频率上限/专注抑制）只读展示 + 心跳态两列（下次
     poll / 最近命中）。只读纪律：零动作面——配置改写走 tool.care_schedule（停人工
     审批），面板写面可后续，指引只给命令文本不代跑。
     取数（诚实边界）：关怀读面当前只在工具面自述（无 HTTP/IPC 读面）——卡片经
     注入缝（loadCare）收快照；缺席 = 「暂无接口」诚实空态，不硬造端点、不摆假
     数据（详见 care-card.ts 头注）。 -->
<script lang="ts">
  import {HeartPulse} from 'lucide-svelte';
  import {careCardView, type CareSnapshot} from '../care-card';

  interface Props {
    /** 取数缝：关怀快照注入（读面落地后一行换源）；返回 null = 读面缺席（诚实空态）。 */
    loadCare?: () => Promise<CareSnapshot | null>;
  }
  let {loadCare}: Props = $props();

  let snapshot = $state<CareSnapshot | null>(null);
  let errorText = $state('');
  let loading = $state(false);

  async function refresh(): Promise<void> {
    if (!loadCare) return;
    loading = true;
    try {
      snapshot = await loadCare();
      errorText = '';
    } catch (error) {
      errorText = error instanceof Error ? error.message : String(error);
    } finally {
      loading = false;
    }
  }

  const view = $derived(careCardView(snapshot));

  $effect(() => {
    void refresh();
  });
</script>

<div class="cc-card">
  <div class="cc-head">
    <span class="cc-title"><HeartPulse size={13} /> 关怀</span>
    <span class="cc-count">{view.available ? `${view.ruleRows.length} 条规则` : '暂无接口'}</span>
  </div>

  <p class="cc-note">{view.note}</p>

  {#if loading}
    <p class="cc-empty">读取中…</p>
  {:else if errorText}
    <p class="cc-error">{errorText}</p>
  {:else}
    <div class="cc-rows">
      {#each view.summaryRows as row (row.key)}
        <div class="cc-row">
          <span class="cc-label">{row.label}</span>
          <span class="cc-value">{row.value}</span>
          {#if row.note}<span class="cc-why">{row.note}</span>{/if}
        </div>
      {/each}
    </div>

    <div class="cc-sec-head">关怀规则（四类触发器闭合词表）</div>
    <div class="cc-list">
      {#each view.ruleRows as row (row.id)}
        <div class="cc-rule">
          <div class="cc-rule-top">
            <span class="cc-rule-id">{row.id}</span>
            <span class="cc-rule-kind">{row.kindLabel}</span>
            <span class="cc-rule-enabled">{row.enabled}</span>
          </div>
          <div class="cc-rule-trigger">{row.trigger}</div>
          <div class="cc-rule-meta">
            <span>素材 {row.material}</span>
            <span>最近触发 {row.lastFired}</span>
            <span>下次触发 {row.nextTrigger}</span>
          </div>
          {#if row.lastFiredNote || row.nextTriggerNote}
            <div class="cc-rule-why">{[row.lastFiredNote, row.nextTriggerNote].filter(Boolean).join(' · ')}</div>
          {/if}
        </div>
      {:else}
        <p class="cc-empty">{view.available ? '暂无关怀规则（care_rules.json 规则表为空）' : '暂无接口'}</p>
      {/each}
    </div>

    <div class="cc-sec-head">打扰纪律（只读展示；改写走 care_schedule 工具/CLI，面板可后续）</div>
    <div class="cc-rows">
      {#each view.disciplineRows as row (row.key)}
        <div class="cc-row">
          <span class="cc-label">{row.label}</span>
          <span class="cc-value">{row.value}</span>
          {#if row.note}<span class="cc-why">{row.note}</span>{/if}
        </div>
      {/each}
    </div>

    <div class="cc-sec-head">心跳态（常驻关怀生产者）</div>
    <div class="cc-rows">
      {#each view.heartbeatRows as row (row.key)}
        <div class="cc-row">
          <span class="cc-label">{row.label}</span>
          <span class="cc-value">{row.value}</span>
          {#if row.note}<span class="cc-why">{row.note}</span>{/if}
        </div>
      {/each}
    </div>

    <div class="cc-sec-head">配置改写指引（命令可复制；界面不代跑）</div>
    <div class="cc-list">
      {#each view.writeGuidance as row (row.key)}
        <div class="cc-guide">
          <div class="cc-guide-label">{row.label}</div>
          <code class="cc-guide-cmd">{row.command}</code>
          <div class="cc-guide-desc">{row.desc}</div>
        </div>
      {/each}
    </div>
    <p class="cc-foot">只读纪律：本卡零写面（care_add/care_remove 每调用停人工审批，面板写面可后续）</p>
  {/if}
</div>

<style>
  .cc-card {
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .cc-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--line);
    background: var(--surface);
  }
  .cc-title {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .cc-count {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--faint);
  }
  .cc-note {
    margin: 0;
    padding: 6px 14px;
    font-size: 11px;
    color: var(--faint);
    border-bottom: 1px solid var(--line);
    overflow-wrap: anywhere;
  }
  .cc-rows {
    display: flex;
    flex-direction: column;
  }
  .cc-row {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 6px 14px;
    border-bottom: 1px solid var(--line);
    font-size: 11px;
  }
  .cc-row:last-child {
    border-bottom: none;
  }
  .cc-label {
    flex: 0 0 118px;
    color: var(--muted);
  }
  .cc-value {
    font-family: var(--mono);
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .cc-why {
    color: var(--faint);
    overflow-wrap: anywhere;
  }
  .cc-sec-head {
    padding: 8px 14px 4px;
    border-top: 1px solid var(--line);
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
  }
  .cc-list {
    display: flex;
    flex-direction: column;
  }
  .cc-rule {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--line);
  }
  .cc-rule:last-child {
    border-bottom: none;
  }
  .cc-rule-top {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .cc-rule-id {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .cc-rule-kind {
    padding: 1px 6px;
    border: 1px solid var(--line);
    border-radius: 999px;
    font-size: 10px;
    color: var(--muted);
    white-space: nowrap;
  }
  .cc-rule-enabled {
    margin-left: auto;
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
    white-space: nowrap;
  }
  .cc-rule-trigger {
    font-size: 11px;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .cc-rule-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
    overflow-wrap: anywhere;
  }
  .cc-rule-why {
    font-size: 10px;
    color: var(--faint);
    overflow-wrap: anywhere;
  }
  .cc-guide {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--line);
  }
  .cc-guide:last-child {
    border-bottom: none;
  }
  .cc-guide-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
  }
  .cc-guide-cmd {
    display: block;
    padding: 6px 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
    font-family: var(--mono);
    font-size: 10px;
    line-height: 1.6;
    color: var(--text);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .cc-guide-desc {
    font-size: 10px;
    color: var(--faint);
    overflow-wrap: anywhere;
  }
  .cc-empty {
    margin: 0;
    padding: 10px 14px;
    font-size: 11px;
    color: var(--faint);
  }
  .cc-error {
    margin: 0;
    padding: 10px 14px 12px;
    font-size: 11px;
    color: var(--amber);
    overflow-wrap: anywhere;
  }
  .cc-foot {
    margin: 0;
    padding: 8px 14px 10px;
    border-top: 1px solid var(--line);
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
  }
</style>
