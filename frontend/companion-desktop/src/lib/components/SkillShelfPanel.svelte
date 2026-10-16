<!-- 技能书架卡（V1-b/d 上屏）：技能库列表（与 skill_search 同源只读）——
     卡名/摘要/触发词/版本；点击展开卡体，L2 封套警示头原样上屏（后端常量同源）。
     取数（诚实边界）：读面已落地（GET /v1/panel/skills，只回元数据——卡体是
     L2 封套面，读面不回卡体），真源经注入缝（loadCards ← runtime.ts fetchSkills）
     一行换源；该部署读面缺席（501/404）时如实上屏「暂无接口」空态 + 警示头
     口径预览，不硬造端点、不摆假旋钮（详见 skill-shelf.ts 头注）。 -->
<script lang="ts">
  import {BookOpen, ChevronDown, ChevronRight} from 'lucide-svelte';
  import {
    skillShelfNoFaceView,
    skillShelfView,
    toggleSkillCard,
  } from '../skill-shelf';
  import type {SkillShelfScan} from '../skill-shelf';

  interface Props {
    /** 取数缝：真源注入（runtime.ts `fetchSkills` 一行换源）；返回 null = 该部署读面缺席（诚实空态）。 */
    loadCards?: () => Promise<SkillShelfScan | null>;
  }
  let {loadCards}: Props = $props();

  let scan = $state<SkillShelfScan | null>(null);
  let expandedName = $state<string | null>(null);
  let errorText = $state('');
  let loading = $state(false);

  async function refresh(): Promise<void> {
    if (!loadCards) return;
    loading = true;
    try {
      scan = await loadCards();
      errorText = '';
    } catch (error) {
      errorText = error instanceof Error ? error.message : String(error);
    } finally {
      loading = false;
    }
  }

  const view = $derived(
    scan === null ? skillShelfNoFaceView() : skillShelfView(scan, expandedName),
  );

  $effect(() => {
    void refresh();
  });
</script>

<div class="ss-card">
  <div class="ss-head">
    <span class="ss-title"><BookOpen size={13} /> 技能</span>
    <span class="ss-count">{scan === null ? '暂无接口' : `${scan.cards.length} 张${scan.refused > 0 ? ` · 拒收 ${scan.refused}` : ''}`}</span>
  </div>

  <!-- L2 封套警示头（原样显示，卡体展开前即在案） -->
  <p class="ss-warning">{view.warningHead}</p>
  <p class="ss-note">{view.note}</p>

  {#if loading}
    <p class="ss-empty">读取中…</p>
  {:else if errorText}
    <p class="ss-error">{errorText}</p>
  {:else}
    <div class="ss-list">
      {#each view.rows as row (row.name)}
        <div class="ss-item" class:open={row.expanded}>
          <button
            class="ss-row"
            onclick={() => (expandedName = toggleSkillCard(expandedName, row.name))}
            aria-expanded={row.expanded}
          >
            <span class="ss-chev">{#if row.expanded}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}</span>
            <span class="ss-name">{row.name}</span>
            <span class="ss-ver">v{row.version}</span>
          </button>
          <div class="ss-meta">
            <span class="ss-summary">{row.summary}</span>
            <span class="ss-triggers">触发词 {row.triggers}</span>
          </div>
          {#if row.expanded && view.expanded}
            <pre class="ss-body">{view.expanded.body}</pre>
          {/if}
        </div>
      {:else}
        <p class="ss-empty">{view.note}</p>
      {/each}
    </div>
  {/if}
</div>

<style>
  .ss-card {
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .ss-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--line);
    background: var(--surface);
  }
  .ss-title {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .ss-count {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--faint);
  }
  .ss-warning {
    margin: 0;
    padding: 8px 14px;
    border-bottom: 1px solid var(--line);
    background: var(--surface);
    font-size: 11px;
    font-weight: 600;
    color: var(--amber);
  }
  .ss-note {
    margin: 0;
    padding: 6px 14px;
    font-size: 11px;
    color: var(--faint);
    border-bottom: 1px solid var(--line);
  }
  .ss-list {
    display: flex;
    flex-direction: column;
  }
  .ss-item {
    border-bottom: 1px solid var(--line);
  }
  .ss-item:last-child {
    border-bottom: none;
  }
  .ss-row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 14px 2px;
    border: none;
    background: none;
    color: var(--text);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }
  .ss-chev {
    display: inline-flex;
    color: var(--faint);
  }
  .ss-name {
    font-family: var(--mono);
    overflow-wrap: anywhere;
  }
  .ss-ver {
    margin-left: auto;
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
    white-space: nowrap;
  }
  .ss-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 14px 8px 34px;
  }
  .ss-summary {
    font-size: 11px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .ss-triggers {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
    overflow-wrap: anywhere;
  }
  .ss-body {
    margin: 0 14px 10px 34px;
    padding: 8px 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
    font-family: var(--mono);
    font-size: 11px;
    line-height: 1.6;
    color: var(--text);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 260px;
    overflow: auto;
  }
  .ss-empty {
    margin: 0;
    padding: 10px 14px;
    font-size: 11px;
    color: var(--faint);
  }
  .ss-error {
    margin: 0;
    padding: 8px 14px 12px;
    font-size: 11px;
    color: var(--amber);
  }
</style>
