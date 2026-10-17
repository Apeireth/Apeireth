<!-- 提案卡（提案账上屏）：提案账最近 N 条只读列表，IM 卡风格呈现——每张卡带
     「建议你亲自确认」行（提案是否采纳由你定夺，本卡不代决、不代批、不回写）。
     取数缝（loadLedger）← runtime.ts fetchProposalLedger（GET
     /v1/panel/memory/streams?kind=proposal，只读）；读面缺席 = 「暂无接口」诚实
     空态，不硬造端点、不摆假数据（详见 proposal-card.ts 头注）。 -->
<script lang="ts">
  import {MessageSquareQuote} from 'lucide-svelte';
  import {proposalCardView, type ProposalLedger} from '../proposal-card';

  interface Props {
    /** 取数缝：提案账注入（runtime.ts fetchProposalLedger 一行换源）；null = 读面缺席。 */
    loadLedger?: () => Promise<ProposalLedger | null>;
    /** 最近 N 条上屏上限（默认 8）。 */
    limit?: number;
  }
  let {loadLedger, limit}: Props = $props();

  let ledger = $state<ProposalLedger | null>(null);
  let errorText = $state('');
  let loading = $state(false);

  async function refresh(): Promise<void> {
    if (!loadLedger) return;
    loading = true;
    try {
      ledger = await loadLedger();
      errorText = '';
    } catch (error) {
      errorText = error instanceof Error ? error.message : String(error);
    } finally {
      loading = false;
    }
  }

  const view = $derived(proposalCardView(ledger, limit));

  $effect(() => {
    void refresh();
  });
</script>

<div class="pc-card">
  <div class="pc-head">
    <span class="pc-title"><MessageSquareQuote size={13} /> 提案</span>
    <span class="pc-count">{view.available ? `${view.rows.length} 条（最近）` : '暂无接口'}</span>
  </div>

  <p class="pc-note">{view.note}</p>

  {#if loading}
    <p class="pc-empty">读取中…</p>
  {:else if errorText}
    <p class="pc-error">{errorText}</p>
  {:else}
    <div class="pc-list">
      {#each view.rows as row (row.id)}
        <div class="pc-item">
          <div class="pc-item-head">
            <span class="pc-item-title">{row.title}</span>
            <span class="pc-item-at">{row.at}</span>
          </div>
          <div class="pc-item-body">{row.body}</div>
          <div class="pc-item-meta">
            <span>主体 {row.subject}</span>
            <span>来源 {row.source}</span>
            <span>标签 {row.tags}</span>
          </div>
          <div class="pc-item-confirm">{row.confirmLine}</div>
        </div>
      {:else}
        <p class="pc-empty">{view.available ? '提案账暂无条目（如实空态）' : '暂无接口'}</p>
      {/each}
    </div>
    <p class="pc-foot">只读纪律：提案账 append-only，本卡零写面（不代决、不代批、不回写）</p>
  {/if}
</div>

<style>
  .pc-card {
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .pc-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--line);
    background: var(--surface);
  }
  .pc-title {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .pc-count {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--faint);
  }
  .pc-note {
    margin: 0;
    padding: 6px 14px;
    font-size: 11px;
    color: var(--faint);
    border-bottom: 1px solid var(--line);
    overflow-wrap: anywhere;
  }
  .pc-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 10px 14px;
    max-height: 420px;
    overflow: auto;
  }
  .pc-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--surface);
  }
  .pc-item-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
  }
  .pc-item-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .pc-item-at {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
    white-space: nowrap;
  }
  .pc-item-body {
    font-size: 11px;
    line-height: 1.6;
    color: var(--text);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .pc-item-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
    overflow-wrap: anywhere;
  }
  .pc-item-confirm {
    margin-top: 2px;
    padding: 4px 8px;
    border: 1px dashed var(--line);
    border-radius: 8px;
    font-size: 11px;
    font-weight: 600;
    color: var(--amber);
    text-align: center;
  }
  .pc-empty {
    margin: 0;
    padding: 10px 14px;
    font-size: 11px;
    color: var(--faint);
  }
  .pc-error {
    margin: 0;
    padding: 10px 14px 12px;
    font-size: 11px;
    color: var(--amber);
    overflow-wrap: anywhere;
  }
  .pc-foot {
    margin: 0;
    padding: 8px 14px 10px;
    border-top: 1px solid var(--line);
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
  }
</style>
