<!-- 工作台卡片区（V1-b/d 上屏）：住户 / 技能 / 随手记 三卡并列。
     住户卡 = UserAdminPanel（用户列表 + owner 管理面 + 当前域节 + CLI 指引）；
     技能卡 = SkillShelfPanel（技能库同源只读 + 展开卡体 + L2 封套警示头）；
     随手记卡 = 最近 50 条账本（与快捷窗回看同源：CaptureLedger）+ 搜索过滤。
     关怀/性格/提案三卡（关怀·性格上屏批 + 面板读面微批）：关怀卡（规则/打扰纪律/
     心跳态只读，真源 GET /v1/panel/care）、性格卡（四滑杆现值 + 锁定态 + 最近移动 +
     本周性格周报 + 锁定解锁 CLI 指引，真源 GET /v1/panel/traits）、提案卡（提案账
     最近 N 条 IM 卡风格，「建议你亲自确认」行）。
     本文件是工作台卡片区容器（组件本体各自独立），接线处见
     components/Workbench.svelte 的挂载点。 -->
<script lang="ts">
  import {StickyNote, Search} from 'lucide-svelte';
  import type {ApeirethConfig} from '../types';
  import UserAdminPanel from '../components/UserAdminPanel.svelte';
  import SkillShelfPanel from '../components/SkillShelfPanel.svelte';
  import CareCard from '../components/CareCard.svelte';
  import TraitCard from '../components/TraitCard.svelte';
  import ProposalCard from '../components/ProposalCard.svelte';
  import {fetchSkills, fetchProposalLedger, fetchCare, fetchTraits} from '../runtime';
  import {
    CAPTURE_REVIEW_LIMIT,
    captureReviewRows,
    filterCaptureRows,
  } from '../capture-review';
  import {CaptureLedger} from '../../features/quick/quick-capture.ts';

  let {config}: {config: ApeirethConfig} = $props();

  /** 随手记账本（与快捷窗回看同一本账，只读消费；账本是回看缓存，记忆流是真源）。 */
  const ledger = new CaptureLedger();
  let query = $state('');
  let tick = $state(0);

  // 账本可能被快捷窗追加（同 localStorage），挂载后取一次 + 刷新按钮重取。
  const rows = $derived.by(() => {
    void tick;
    return captureReviewRows(ledger.records(), CAPTURE_REVIEW_LIMIT);
  });
  const shown = $derived(filterCaptureRows(rows, query));

  function reload(): void {
    tick += 1;
  }
</script>

<div class="wbv-cards">
  <UserAdminPanel {config} />
  <SkillShelfPanel loadCards={() => fetchSkills(config)} />
  <!-- 关怀/性格注入缝一行换源（面板读面微批）：真源 = runtime 只读 fetch；
       读面缺席（501/404）= 卡内诚实空态（CARE_NO_FACE / 缺源「—」），不硬造端点。 -->
  <CareCard loadCare={() => fetchCare(config)} />
  <TraitCard loadTraits={() => fetchTraits(config)} />
  <ProposalCard loadLedger={() => fetchProposalLedger(config)} />

  <div class="cp-card">
    <div class="cp-head">
      <span class="cp-title"><StickyNote size={13} /> 随手记</span>
      <span class="cp-tools">
        <span class="cp-count">{shown.length} / {rows.length} 条（账本上限 {CAPTURE_REVIEW_LIMIT}）</span>
        <button class="cp-reload" onclick={reload}>刷新</button>
      </span>
    </div>
    <div class="cp-search">
      <span class="cp-search-ico"><Search size={12} /></span>
      <input class="cp-search-input" type="search" placeholder="过滤（正文或序号，如 #3）" bind:value={query} />
    </div>
    <div class="cp-list">
      {#each shown as row (row.seq)}
        <div class="cp-row">
          <span class="cp-seq">#{row.seq}</span>
          <span class="cp-time">{row.time}</span>
          <span class="cp-text">{row.text}</span>
        </div>
      {:else}
        <p class="cp-empty">{rows.length === 0 ? '账本暂无随手记（快捷窗 # 开头即记）' : '无匹配条目'}</p>
      {/each}
    </div>
    <p class="cp-foot">与快捷窗回看同源（本地账本只读缓存，记忆流是真源）</p>
  </div>
</div>

<style>
  .wbv-cards {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-width: 0;
  }
  /* 2026-10-18 美术走查：工作台卡片与工具卡/指标卡统一「浮起」语汇；
     卡头原来是 --surface 实心块 + 硬边，读作表格表头，改为留空的卡头。 */
  .cp-card {
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface-2);
    box-shadow: inset 0 1px 0 var(--ap-hairline-top), var(--ap-shadow-card);
    overflow: hidden;
  }
  .cp-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 11px 14px;
    border-bottom: 1px solid var(--line);
  }
  .cp-title {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--text);
  }
  .cp-tools {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .cp-count {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--faint);
  }
  .cp-reload {
    padding: 4px 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--text);
    font-size: 11px;
    cursor: pointer;
  }
  .cp-search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--line);
  }
  .cp-search-ico {
    display: inline-flex;
    color: var(--faint);
  }
  .cp-search-input {
    flex: 1;
    min-width: 0;
    padding: 4px 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
    font-family: var(--mono);
    font-size: 11px;
  }
  .cp-list {
    display: flex;
    flex-direction: column;
    max-height: 280px;
    overflow: auto;
  }
  .cp-row {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 6px 14px;
    border-bottom: 1px solid var(--line);
  }
  .cp-row:last-child {
    border-bottom: none;
  }
  .cp-seq {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
    white-space: nowrap;
  }
  .cp-time {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
    white-space: nowrap;
  }
  .cp-text {
    font-size: 12px;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .cp-empty {
    margin: 0;
    padding: 10px 14px;
    font-size: 11px;
    color: var(--faint);
  }
  .cp-foot {
    margin: 0;
    padding: 8px 14px 10px;
    border-top: 1px solid var(--line);
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
  }
</style>
