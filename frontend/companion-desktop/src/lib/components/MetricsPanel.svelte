<!-- 运行指标卡（D9 上屏）：消费 `GET /v1/panel/metrics`（契约 §10.1，已过
     令牌门）。延迟 P50/P95/均值、token 流向、缓存命中率、成功/失败/待批计数
     ——空态「—」语义照契约（null 不冒充 0）；刷新按钮 + 30s 自动刷新。
     只读展示：无保存按钮、无写面（即效/读取语义纪律）。 -->
<script lang="ts">
  import {Activity, RefreshCcw} from 'lucide-svelte';
  import type {ApeirethConfig} from '../types';
  import {describeCaught, normalizeBaseUrl} from '../runtime';
  import {gatewayAuthHeaders} from '../tauri-bridge';
  import {
    METRICS_AUTO_REFRESH_MS,
    fetchPanelMetricsSnapshot,
    metricsPanelView,
    startMetricsAutoRefresh,
  } from '../metrics';
  import type {PanelMetricsSnapshot} from '../metrics';

  let {config}: {config: ApeirethConfig} = $props();

  /** 最近一帧真值（取数失败保留上一帧 + 报真因，不静默降级成空态）。 */
  let snapshot = $state<PanelMetricsSnapshot | null>(null);
  let errorText = $state('');
  let loading = $state(false);
  let refreshedAt = $state<number | null>(null);

  /** 一次刷新（手动按钮与自动刷新走同一条取数缝）。 */
  async function refresh(): Promise<void> {
    loading = true;
    try {
      snapshot = await fetchPanelMetricsSnapshot(
        {baseUrl: normalizeBaseUrl(config.baseUrl)},
        {
          // 令牌面单源：每个网关请求都走 tauri-bridge 鉴权头缝。
          authHeaders: () => gatewayAuthHeaders(),
          fetchJson: (url, init) => fetch(url, init),
        },
      );
      errorText = '';
      refreshedAt = Date.now();
    } catch (error) {
      // 错误即帧：真因上屏（HTTP 状态/信封形状/网络），不编空态。
      errorText = describeCaught(error);
    } finally {
      loading = false;
    }
  }

  const view = $derived(metricsPanelView(snapshot));

  // 首挂即取数 + 30s 自动刷新；换端点/卸载即停（$effect 清理，不泄漏定时器）。
  $effect(() => {
    void refresh();
    const auto = startMetricsAutoRefresh(() => void refresh());
    return () => auto.stop();
  });

  function formatClock(epochMs: number): string {
    return new Date(epochMs).toLocaleTimeString('zh-CN');
  }
</script>

<div class="metrics-card">
  <div class="metrics-head">
    <span class="metrics-title"><Activity size={13} /> 运行指标</span>
    <span class="metrics-tools">
      <span class="metrics-auto">自动刷新 {METRICS_AUTO_REFRESH_MS / 1000}s</span>
      <button class="metrics-refresh" onclick={() => void refresh()} disabled={loading}>
        <span class="metrics-ico" class:spin={loading}><RefreshCcw size={13} /></span>
        <span>{loading ? '刷新中…' : '刷新'}</span>
      </button>
    </span>
  </div>

  <div class="metrics-grid">
    {#each view.cells as cell (cell.key)}
      <div class="metrics-cell">
        <small>{cell.label}</small>
        <b>{cell.value}</b>
      </div>
    {/each}
  </div>

  <div class="metrics-foot">
    <span class="metrics-caption">{view.caption}</span>
    {#if refreshedAt !== null}
      <span class="metrics-clock">最后刷新 {formatClock(refreshedAt)}</span>
    {/if}
  </div>

  {#if errorText}
    <p class="metrics-error">{errorText}</p>
  {/if}
</div>

<style>
  .metrics-card {
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .metrics-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--line);
    background: var(--surface);
  }
  .metrics-title {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .metrics-tools {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .metrics-auto {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--faint);
  }
  .metrics-refresh {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--text);
    font-size: 11px;
    cursor: pointer;
  }
  .metrics-refresh:disabled {
    cursor: default;
    opacity: 0.7;
  }
  .metrics-ico {
    display: inline-flex;
  }
  .spin {
    animation: metrics-spin 0.9s linear infinite;
  }
  @keyframes metrics-spin {
    to {
      transform: rotate(360deg);
    }
  }
  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--line);
  }
  .metrics-cell {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .metrics-cell small {
    font-size: 11px;
    color: var(--faint);
  }
  .metrics-cell b {
    font-family: var(--mono);
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .metrics-foot {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 14px;
  }
  .metrics-caption {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--muted);
  }
  .metrics-clock {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--faint);
    white-space: nowrap;
  }
  .metrics-error {
    margin: 0;
    padding: 8px 14px 12px;
    font-size: 11px;
    color: var(--amber);
  }
</style>
