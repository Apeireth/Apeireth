<!-- 性格卡（R2 性格锁定上屏）：四滑杆现值 + 锁定态 + 最近移动 +「本周性格周报」
     区块。锁定/解锁按钮 = CLI 指引（trait 管理面无 HTTP 面，读码定：只读 + 指引，
     界面不代跑）；周报区块 = 成长计既有账（tuning-log.jsonl）7 天窗投影，与
     tool.trait_report 同账同语义（详见 trait-card.ts 头注）。 -->
<script lang="ts">
  import {SlidersHorizontal, Lock, Unlock} from 'lucide-svelte';
  import {
    traitCardView,
    traitLockCommand,
    type TraitLocksLike,
    type TraitValuesLike,
    type TuningLogEntryLike,
  } from '../trait-card';

  interface Props {
    /** 四滑杆现值（本机下发旋钮值，APEIRETH_TUNE_* 同源）；null = 缺源显「—」。 */
    values?: TraitValuesLike | null;
    /** 锁定态（trait-locks.json）；当前无面板读面 → null = 「—」（不冒充未锁定）。 */
    locks?: TraitLocksLike | null;
    /** 成长计既有账取数缝（桌面 IPC readTuningLog 真源）；null = 读不到账。 */
    loadMoves?: () => Promise<TuningLogEntryLike[] | null>;
    /** 账面坏行跳过计数；读面缺席 = null → 「—」。 */
    skippedRecords?: number | null;
  }
  let {values = null, locks = null, loadMoves, skippedRecords = null}: Props = $props();

  let moves = $state<readonly TuningLogEntryLike[] | null>(null);
  let errorText = $state('');
  let loading = $state(false);
  let guideLine = $state('');

  async function refresh(): Promise<void> {
    if (!loadMoves) return;
    loading = true;
    try {
      moves = await loadMoves();
      errorText = '';
    } catch (error) {
      errorText = error instanceof Error ? error.message : String(error);
    } finally {
      loading = false;
    }
  }

  const view = $derived(traitCardView({values, locks, moves, skippedRecords}));

  $effect(() => {
    void refresh();
  });
</script>

<div class="tc-card">
  <div class="tc-head">
    <span class="tc-title"><SlidersHorizontal size={13} /> 性格</span>
    <span class="tc-count">四滑杆 · 只读</span>
  </div>

  <p class="tc-note">{view.note}</p>

  {#if loading}
    <p class="tc-empty">读取中…</p>
  {:else if errorText}
    <p class="tc-error">{errorText}</p>
  {:else}
    <div class="tc-sec-head">四滑杆现值 / 锁定态 / 最近移动</div>
    <div class="tc-list">
      {#each view.sliderRows as row (row.param)}
        <div class="tc-slider">
          <div class="tc-slider-top">
            <span class="tc-slider-label">{row.label}</span>
            <span class="tc-slider-value">{row.value}</span>
            <span class="tc-slider-lock" class:locked={row.locked === '已锁定'}>{row.locked}</span>
            <span class="tc-slider-btns">
              <button
                class="tc-mini"
                title="锁定走 CLI（显式动作）"
                onclick={() => (guideLine = traitLockCommand('lock', row.param))}
              >
                <Lock size={11} /> 锁定
              </button>
              <button
                class="tc-mini"
                title="解锁走 CLI（显式动作）"
                onclick={() => (guideLine = traitLockCommand('unlock', row.param))}
              >
                <Unlock size={11} /> 解锁
              </button>
            </span>
          </div>
          <div class="tc-slider-move">最近移动 {row.lastMove}</div>
          {#if row.lockedNote || row.lastMoveNote}
            <div class="tc-slider-why">{[row.lockedNote, row.lastMoveNote].filter(Boolean).join(' · ')}</div>
          {/if}
        </div>
      {/each}
    </div>

    {#if guideLine}
      <p class="tc-guide-line">CLI 指引（复制执行，界面不代跑）：<code>{guideLine}</code></p>
    {/if}

    <div class="tc-sec-head">最近移动（新 → 旧，零移动留痕不计）</div>
    <div class="tc-list">
      {#each view.recentMoveRows as row, index (`${row.param}-${row.at}-${index}`)}
        <div class="tc-move">
          <span class="tc-move-at">{row.at}</span>
          <span class="tc-move-label">{row.label}</span>
          <span class="tc-move-delta">{row.delta}</span>
          <span class="tc-move-reason">{row.reason}</span>
        </div>
      {:else}
        <p class="tc-empty">暂无实际移动记录（如实空态，零移动留痕不计）</p>
      {/each}
    </div>

    <div class="tc-sec-head">{view.week.title}</div>
    <p class="tc-week-window">窗口 {view.week.window}</p>
    <div class="tc-list">
      <div class="tc-week-row tc-week-head">
        <span>滑杆</span><span>净移动</span><span>移动次数</span><span>锁定跳过</span><span>最近移动</span>
      </div>
      {#each view.week.rows as row (row.param)}
        <div class="tc-week-row">
          <span>{row.label}</span>
          <span>{row.netDelta}</span>
          <span>{row.moveCount}</span>
          <span>{row.lockedSkipped}</span>
          <span>{row.lastMove}</span>
        </div>
      {/each}
    </div>
    {#if view.week.topReasons.length > 0}
      <div class="tc-week-reasons">
        移动原因 top：
        {#each view.week.topReasons as row, index (row.reason)}
          <span class="tc-week-reason">{index + 1}. {row.reason}（{row.count}）</span>
        {/each}
      </div>
    {/if}
    <div class="tc-rows">
      {#each view.week.totals as row (row.key)}
        <div class="tc-row">
          <span class="tc-row-label">{row.label}</span>
          <span class="tc-row-value">{row.value}</span>
          {#if row.note}<span class="tc-row-why">{row.note}</span>{/if}
        </div>
      {/each}
    </div>
    <p class="tc-week-note">{view.week.note}</p>

    <div class="tc-sec-head">锁定/解锁指引（显式动作，CLI；界面不代跑）</div>
    <div class="tc-list">
      {#each view.lockGuidance as row (row.key)}
        <div class="tc-guide">
          <div class="tc-guide-label">{row.label}</div>
          <code class="tc-guide-cmd">{row.command}</code>
          <div class="tc-guide-desc">{row.desc}</div>
        </div>
      {/each}
    </div>
    <p class="tc-foot">只读纪律：本卡零写面——锁定/解锁是 CLI 显式动作（trait 管理面无 HTTP 面，读码定）</p>
  {/if}
</div>

<style>
  .tc-card {
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .tc-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--line);
    background: var(--surface);
  }
  .tc-title {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .tc-count {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--faint);
  }
  .tc-note,
  .tc-week-note {
    margin: 0;
    padding: 6px 14px;
    font-size: 11px;
    color: var(--faint);
    border-bottom: 1px solid var(--line);
    overflow-wrap: anywhere;
  }
  .tc-week-note {
    border-bottom: none;
    border-top: 1px solid var(--line);
  }
  .tc-sec-head {
    padding: 8px 14px 4px;
    border-top: 1px solid var(--line);
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
  }
  .tc-list {
    display: flex;
    flex-direction: column;
  }
  .tc-slider {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--line);
  }
  .tc-slider:last-child {
    border-bottom: none;
  }
  .tc-slider-top {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .tc-slider-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text);
  }
  .tc-slider-value {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--text);
  }
  .tc-slider-lock {
    padding: 1px 6px;
    border: 1px solid var(--line);
    border-radius: 999px;
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
    white-space: nowrap;
  }
  .tc-slider-lock.locked {
    color: var(--amber);
    border-color: var(--amber);
  }
  .tc-slider-btns {
    margin-left: auto;
    display: flex;
    gap: 6px;
  }
  .tc-mini {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--text);
    font-size: 10px;
    cursor: pointer;
    white-space: nowrap;
  }
  .tc-slider-move {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
    overflow-wrap: anywhere;
  }
  .tc-slider-why {
    font-size: 10px;
    color: var(--faint);
    overflow-wrap: anywhere;
  }
  .tc-guide-line {
    margin: 0;
    padding: 8px 14px;
    border-top: 1px solid var(--line);
    font-size: 10px;
    color: var(--faint);
    overflow-wrap: anywhere;
  }
  .tc-guide-line code {
    font-family: var(--mono);
    color: var(--text);
  }
  .tc-move {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 6px 14px;
    border-bottom: 1px solid var(--line);
    font-size: 11px;
  }
  .tc-move:last-child {
    border-bottom: none;
  }
  .tc-move-at {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
    white-space: nowrap;
  }
  .tc-move-label {
    color: var(--muted);
    white-space: nowrap;
  }
  .tc-move-delta {
    font-family: var(--mono);
    color: var(--text);
    white-space: nowrap;
  }
  .tc-move-reason {
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .tc-week-window {
    margin: 0;
    padding: 4px 14px;
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
    overflow-wrap: anywhere;
  }
  .tc-week-row {
    display: grid;
    grid-template-columns: 1.4fr 0.8fr 0.8fr 0.8fr 1.2fr;
    gap: 8px;
    padding: 6px 14px;
    border-bottom: 1px solid var(--line);
    font-family: var(--mono);
    font-size: 10px;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .tc-week-row:last-child {
    border-bottom: none;
  }
  .tc-week-head {
    color: var(--muted);
  }
  .tc-week-reasons {
    padding: 6px 14px;
    border-top: 1px solid var(--line);
    font-size: 10px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .tc-week-reason {
    margin-right: 8px;
    font-family: var(--mono);
    color: var(--text);
  }
  .tc-rows {
    display: flex;
    flex-direction: column;
  }
  .tc-row {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 6px 14px;
    border-top: 1px solid var(--line);
    font-size: 11px;
  }
  .tc-row-label {
    flex: 0 0 118px;
    color: var(--muted);
  }
  .tc-row-value {
    font-family: var(--mono);
    color: var(--text);
  }
  .tc-row-why {
    color: var(--faint);
    overflow-wrap: anywhere;
  }
  .tc-guide {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--line);
  }
  .tc-guide:last-child {
    border-bottom: none;
  }
  .tc-guide-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
  }
  .tc-guide-cmd {
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
  .tc-guide-desc {
    font-size: 10px;
    color: var(--faint);
    overflow-wrap: anywhere;
  }
  .tc-empty {
    margin: 0;
    padding: 10px 14px;
    font-size: 11px;
    color: var(--faint);
  }
  .tc-error {
    margin: 0;
    padding: 10px 14px 12px;
    font-size: 11px;
    color: var(--amber);
    overflow-wrap: anywhere;
  }
  .tc-foot {
    margin: 0;
    padding: 8px 14px 10px;
    border-top: 1px solid var(--line);
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
  }
</style>
