<script lang="ts">
  import {onMount} from 'svelte';
  import {ArrowUp, Loader2, PenLine, Sparkles, X} from 'lucide-svelte';
  import StatusDot from '../../components/StatusDot.svelte';
  import {renderMarkdown} from '../../lib/markdown';
  import {checkHealthDetailed, createAgentRuntime, loadConfig} from '../../lib/runtime';
  import type {ApeirethConfig, HealthState} from '../../lib/types';
  import {
    CAPTURE_RECENT_VIEW,
    CaptureLedger,
    captureToMemoryStream,
    detectQuickMode,
    formatCaptureTime,
    type CaptureRecord,
  } from './quick-capture';

  let config = $state<ApeirethConfig>(loadConfig());
  let query = $state('');
  let responseText = $state('');
  let busy = $state(false);
  let error = $state('');
  let inputEl = $state<HTMLInputElement | null>(null);
  let health = $state<HealthState>('connecting');

  // ---- 随手记（Quick Capture，C 轴入口面）：纯采集零 token，不进对话回合 ----
  // 模式随输入即时切换：`#` 前缀 = 随手记（详见 features/quick/quick-capture.ts）。
  const captureLedger = new CaptureLedger();
  const mode = $derived(detectQuickMode(query));
  let capturing = $state(false);
  let captureFeedback = $state('');
  let captureError = $state('');
  let recentCaptures = $state<CaptureRecord[]>([]);

  // Generate unique session ID per quick-window instance (persisted in localStorage)
  const QUICK_SESSION_KEY = 'apeireth:quick-window:session-id';
  let quickSessionId = $state<string>('');

  function ensureQuickSession(): string {
    if (quickSessionId) return quickSessionId;
    const stored = localStorage.getItem(QUICK_SESSION_KEY);
    if (stored) {
      quickSessionId = stored;
      return stored;
    }
    const newId = `quick-${crypto.randomUUID()}`;
    localStorage.setItem(QUICK_SESSION_KEY, newId);
    quickSessionId = newId;
    return newId;
  }

  const runtime = createAgentRuntime(config);
  const responseHtml = $derived(responseText ? renderMarkdown(responseText) : '');
  const healthText = $derived(
    busy
      ? '生成中'
      : health === 'online' || health === 'ready'
        ? '已连接'
        : health === 'connecting'
          ? '连接中…'
          : health === 'degraded'
            ? '部分可用'
            : '未连接',
  );
  const dotOff = $derived(!busy && (health === 'offline' || health === 'error'));
  const dotActive = $derived(
    busy || health === 'connecting' || health === 'generating' || health === 'degraded',
  );

  async function probeHealth(): Promise<void> {
    try {
      const report = await checkHealthDetailed(config.baseUrl, config.apiKey);
      health = report.overall;
    } catch {
      health = 'offline';
    }
  }

  async function handleClose(): Promise<void> {
    try {
      // × 不自毁：webview 调用自身 destroy() 会自毁挂死（Windows 鬼影窗）。
      // 发命令请 Rust 从外部「先隐身影、再销毁」，确定性拆除。
      const {invoke} = await import('@tauri-apps/api/core');
      await invoke('close_quick_window');
    } catch {
      try {
        const {getCurrentWebviewWindow} = await import('@tauri-apps/api/webviewWindow');
        const w = getCurrentWebviewWindow();
        await w.hide().catch(() => undefined);
        await w.destroy();
      } catch {
        window.close();
      }
    }
  }

  async function submit(): Promise<void> {
    const text = query.trim();
    if (!text || busy) return;
    busy = true;
    error = '';
    responseText = '';
    captureFeedback = '';
    captureError = '';

    try {
      await runtime.run(
        {
          messages: [{role: 'user', content: text}],
          model: {id: config.model, provider: 'apeireth'},
          sessionId: ensureQuickSession(),
          context: {user: '主人'},
        },
        (event) => {
          if (event.type === 'text-delta') {
            responseText += event.text;
          }
        },
      );
    } catch (caught) {
      error = caught instanceof Error
        ? caught.message
        : (typeof caught === 'object' && caught !== null && 'message' in caught)
          ? String(caught.message)
          : String(caught);
    } finally {
      busy = false;
    }
  }

  /**
   * 随手记一键记：直接入记忆流（appendMemoryEpisode 同源），
   * 不走对话回合、不调模型、不发通知（纯采集零 token）。
   * 落地反馈「已记下 ✓」；拒绝/失败走错误帧，不谎报成功。
   */
  async function handleCapture(): Promise<void> {
    if (capturing) return;
    capturing = true;
    captureFeedback = '';
    captureError = '';
    try {
      const outcome = await captureToMemoryStream(config, captureLedger, query);
      if (outcome.ok) {
        query = '';
        captureFeedback = '已记下 ✓';
        recentCaptures = captureLedger.recent(CAPTURE_RECENT_VIEW);
      } else {
        captureError = outcome.message;
      }
    } catch (caught) {
      captureError = caught instanceof Error ? `没记下：${caught.message}` : `没记下：${String(caught)}`;
    } finally {
      capturing = false;
    }
  }

  onMount(() => {
    // 透明窗钩子: html/body 背景透明, 配壳的 transparent 窗 (base.css html.companion-bg)
    document.documentElement.classList.add('companion-bg');
    recentCaptures = captureLedger.recent(CAPTURE_RECENT_VIEW);
    inputEl?.focus();
    void probeHealth();
    const probeTimer = window.setInterval(() => void probeHealth(), 15_000);
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        void handleClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      document.documentElement.classList.remove('companion-bg');
      window.clearInterval(probeTimer);
      window.removeEventListener('keydown', handleKeyDown);
    };
  });
</script>

<div class="quick-window-shell">
  <div class="quick-header" data-tauri-drag-region>
    <div class="quick-brand" title="后端：{healthText}">
      <StatusDot size="tiny" off={dotOff} active={dotActive} />
      <Sparkles size={14} class="exec-icon-accent" />
      <span>Apeireth 快捷助手</span>
    </div>
    <button type="button" class="quick-close-btn" onclick={handleClose} aria-label="关闭">
      <X size={13} />
    </button>
  </div>

  <div class="quick-input-container">
    <input
      bind:this={inputEl}
      bind:value={query}
      placeholder={mode === 'capture'
        ? '随手记：直接记入记忆流，不走对话…'
        : '快速向阿佩瑞斯提问、记录想法或下达指令…（# 开头 = 随手记）'}
      oninput={() => {
        captureFeedback = '';
        captureError = '';
      }}
      onkeydown={(e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
          e.preventDefault();
          if (mode === 'capture') void handleCapture();
          else void submit();
        }
      }}
    />
    <button
      type="button"
      class="primary-button quick-send-btn"
      disabled={mode === 'capture' ? capturing || !query.trim() : busy || !query.trim()}
      onclick={() => (mode === 'capture' ? void handleCapture() : void submit())}
      aria-label={mode === 'capture' ? '记入记忆流' : '发送'}
    >
      {#if mode === 'capture'}
        <PenLine size={14} />
      {:else if busy}
        <Loader2 size={14} class="spinner" />
      {:else}
        <ArrowUp size={14} />
      {/if}
    </button>
  </div>

  {#if captureFeedback}
    <p class="capture-feedback" role="status">{captureFeedback}</p>
  {/if}
  {#if captureError}
    <p class="error-banner" role="alert">{captureError}</p>
  {/if}

  <div class="quick-tags-row">
    <button type="button" class="quick-tag" onclick={() => { query = '总结今天的核心记忆与进展'; void submit(); }}>
      今日记忆总结
    </button>
    <button type="button" class="quick-tag" onclick={() => { query = '检查当前正在推进的目标状态'; void submit(); }}>
      目标状态检查
    </button>
    <button type="button" class="quick-tag" onclick={() => { query = '系统与器官健康度状态'; void submit(); }}>
      系统器官体检
    </button>
  </div>

  {#if recentCaptures.length > 0}
    <div class="capture-recent" aria-label="最近随手记">
      <div class="capture-recent-title">最近随手记</div>
      {#each recentCaptures as item (item.seq)}
        <div class="capture-recent-row">
          <span class="capture-seq">#{item.seq}</span>
          <span class="capture-text" title={item.text}>{item.text}</span>
          <span class="capture-time">{formatCaptureTime(item.at)}</span>
        </div>
      {/each}
    </div>
  {/if}

  {#if responseText || busy || error}
    <div class="quick-response-area md-body">
      {#if responseText}
        {@html responseHtml}
        {#if busy}<span class="md-caret" aria-hidden="true"></span>{/if}
      {:else if busy}
        <div class="typing"><i></i><i></i><i></i></div>
      {/if}
      {#if error}
        <p class="error-banner" role="alert">{error}</p>
      {/if}
    </div>
  {/if}
</div>
