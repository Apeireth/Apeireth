<script lang="ts">
  import {onMount} from 'svelte';
  import {
    Wrench,
    Search,
    Shield,
    ShieldAlert,
    ShieldCheck,
    Cpu,
    RotateCcw,
    Check,
    X,
    ExternalLink,
    Code,
    Clock,
    Lock,
    Key,
    ChevronDown,
    ChevronRight,
  } from 'lucide-svelte';
  import PageHeader from '../PageHeader.svelte';
  import EmptyState from '../components/EmptyState.svelte';
  import ErrorState from '../components/ErrorState.svelte';
  import LoadingState from '../components/LoadingState.svelte';
  import StatusBadge from '../components/StatusBadge.svelte';
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import type {ApeirethConfig, ApprovalRequestItem, ToolItem} from '../types';
  import {fetchApprovalRequests, fetchTools, grantToolPermission} from '../runtime';

  let {
    config,
  }: {
    config: ApeirethConfig;
  } = $props();

  let tools = $state<ToolItem[]>([]);
  let approvalRequests = $state<ApprovalRequestItem[]>([]);
  let loading = $state(false);
  let error = $state('');
  let searchQuery = $state('');

  // Selected tool detail modal
  let selectedTool = $state<ToolItem | null>(null);
  let showSchema = $state(false);

  // Approval modal state
  let approvingTool = $state<ApprovalRequestItem | null>(null);
  let masterTokenDraft = $state('');
  let grantHours = $state(24);
  let grantBusy = $state(false);
  let grantError = $state('');
  let grantSuccess = $state(false);

  async function loadData() {
    loading = true;
    error = '';
    try {
      const [toolsRes, approvalsRes] = await Promise.all([
        fetchTools(config).catch(() => []),
        fetchApprovalRequests(config).catch(() => []),
      ]);
      tools = toolsRes;
      approvalRequests = approvalsRes;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function openApprovalModal(req: ApprovalRequestItem) {
    approvingTool = req;
    masterTokenDraft = '';
    grantHours = 24;
    grantError = '';
    grantSuccess = false;
  }

  function closeApprovalModal() {
    approvingTool = null;
    masterTokenDraft = '';
    grantError = '';
    grantSuccess = false;
  }

  async function executeGrant() {
    if (!approvingTool) return;
    grantBusy = true;
    grantError = '';
    grantSuccess = false;

    const res = await grantToolPermission(
      config,
      approvingTool.tool,
      grantHours,
      masterTokenDraft,
    );

    // Immediately clear in-memory token draft after transaction
    masterTokenDraft = '';
    grantBusy = false;

    if (res.ok) {
      grantSuccess = true;
      setTimeout(() => {
        closeApprovalModal();
        void loadData();
      }, 1200);
    } else {
      grantError = res.error || '授权失败，请检查 Master Token 是否有效';
    }
  }


  const filteredTools = $derived.by(() => {
    let list = [...tools];
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      list = list.filter((t) =>
        t.name.toLowerCase().includes(q) ||
        (t.description && t.description.toLowerCase().includes(q)),
      );
    }
    return list;
  });

  onMount(() => {
    void loadData();
  });
</script>

<section class="tools-view">
  <PageHeader
    eyebrow="能力"
    title="工具管理与权限"
    subtitle="查看 Apeireth 注册工具、输入输出规范及待主人批准的高危特权调用。"
  >
    <button class="quiet-button" onclick={loadData} disabled={loading}>
      <RotateCcw size={13} class={loading ? 'spin' : ''} />
      <span>刷新工具列表</span>
    </button>
  </PageHeader>

  <!-- Pending Approvals Alert Section -->
  {#if approvalRequests.length > 0}
    <div class="approval-banner">
      <div class="banner-head">
        <ShieldAlert size={16} class="alert-icon" />
        <strong>待主人审批的工具请求 ({approvalRequests.length})</strong>
      </div>
      <p class="banner-desc">高危工具（如文件读写、Shell执行）默认受权限洋葱与宪法防护，需主人授权后执行。</p>
      <div class="approval-list">
        {#each approvalRequests as req}
          <div class="approval-item">
            <div class="req-info">
              <code class="req-tool">{req.tool}</code>
              <span class="req-summary">{req.summary}</span>
              {#if req.requestedAt}
                <span class="req-time">{new Date(req.requestedAt > 1e11 ? req.requestedAt : req.requestedAt * 1000).toLocaleTimeString('zh-CN')}</span>
              {/if}
            </div>
            <button class="primary-button small-btn" onclick={() => openApprovalModal(req)}>
              <Key size={12} />
              <span>授予时效权限</span>
            </button>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Toolbar -->
  <div class="tools-toolbar">
    <div class="search-input-wrap">
      <Search size={14} class="search-icon" />
      <input
        type="text"
        placeholder="搜索工具名称或描述…"
        bind:value={searchQuery}
      />
      {#if searchQuery}
        <button class="clear-search-btn" onclick={() => searchQuery = ''} aria-label="清除搜索">
          <X size={12} />
        </button>
      {/if}
    </div>

    <div class="tools-summary-meta">
      <span>已装配工具: <b>{tools.length}</b> 个</span>
    </div>
  </div>

  <!-- Tools Grid -->
  <div class="tools-container">
    {#if loading && !tools.length}
      <LoadingState message="正在连接工具注册表…" />
    {:else if error && !tools.length}
      <ErrorState title="拉取工具列表失败" message={error} onRetry={loadData} />
    {:else if !filteredTools.length}
      <EmptyState
        icon="🔧"
        title={searchQuery ? '没有找到匹配的工具' : '工具注册表为空'}
        description="后端未注册工具或后端未连接。"
      />
    {:else}
      <div class="tools-grid">
        {#each filteredTools as tool}
          <div
            class="tool-card-box"
            role="button"
            tabindex="0"
            onclick={() => { selectedTool = tool; showSchema = false; }}
            onkeydown={(e) => e.key === 'Enter' && (selectedTool = tool)}
          >
            <div class="card-head">
              <div class="tool-id-row">
                <span class="tool-icon-box"><Cpu size={15} /></span>
                <strong class="tool-title">{tool.name}</strong>
              </div>
              <StatusBadge
                label={tool.available ? '可用' : '不可用'}
                variant={tool.available ? 'green' : 'neutral'}
                size="small"
              />
            </div>

            <p class="tool-desc">{tool.description || '内置系统工具，供 Agent 自主决策调用。'}</p>

            <div class="card-foot">
              <span class="source-pill">内置能力</span>
              <button
                class="details-link"
                onclick={(e) => { e.stopPropagation(); selectedTool = tool; showSchema = false; }}
              >
                <span>参数规范</span>
                <ExternalLink size={11} />
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</section>

<!-- Tool Details Modal -->
{#if selectedTool}
  <div class="modal-backdrop" onclick={() => selectedTool = null} role="presentation">
    <div
      class="modal-dialog"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="tool-dialog-title"
    >
      <div class="modal-header">
        <div class="modal-title-wrap">
          <Cpu size={18} class="dialog-cpu-icon" />
          <h3 id="tool-dialog-title">{selectedTool.name}</h3>
          <StatusBadge label="Built-in" variant="amber" size="small" />
        </div>
        <button class="modal-close-btn" onclick={() => selectedTool = null} aria-label="关闭">
          <X size={16} />
        </button>
      </div>

      <div class="modal-body">
        <div class="detail-section">
          <span class="detail-label">功能说明</span>
          <p class="desc-text">{selectedTool.description || '无具体说明'}</p>
        </div>

        <div class="detail-section">
          <span class="detail-label">安全策略与权限等级</span>
          <div class="security-info">
            <ShieldCheck size={14} class="sec-icon" />
            <span>执行受宪法评审 (MiniMaxConstitutionLlm) 与洋葱权限模型保护</span>
          </div>
        </div>

        {#if selectedTool.argsSchema}
          <div class="detail-section">
            <div class="schema-head" role="button" tabindex="0" onclick={() => showSchema = !showSchema} onkeydown={(e) => e.key === 'Enter' && (showSchema = !showSchema)}>
              <span class="detail-label">调用参数 Schema (JSON)</span>
              <button class="toggle-schema-btn">
                {#if showSchema}<ChevronDown size={13} />{:else}<ChevronRight size={13} />{/if}
                <span>{showSchema ? '收起' : '展开'}</span>
              </button>
            </div>
            {#if showSchema}
              <pre class="schema-pre">{JSON.stringify(selectedTool.argsSchema, null, 2)}</pre>
            {/if}
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="primary-btn" onclick={() => selectedTool = null}>完成</button>
      </div>
    </div>
  </div>
{/if}

<!-- Master Token Grant Modal -->
{#if approvingTool}
  <div class="modal-backdrop" onclick={closeApprovalModal} role="presentation">
    <div
      class="modal-dialog"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="grant-dialog-title"
    >
      <div class="modal-header">
        <div class="modal-title-wrap">
          <ShieldAlert size={18} class="dialog-shield-icon" />
          <h3 id="grant-dialog-title">主人特权授权: {approvingTool.tool}</h3>
        </div>
        <button class="modal-close-btn" onclick={closeApprovalModal} aria-label="关闭">
          <X size={16} />
        </button>
      </div>

      <div class="modal-body">
        <p class="grant-intro">
          您正在为工具 <code>{approvingTool.tool}</code> 签发时效性授权包 (PermissionPack)。
        </p>

        <div class="form-field">
          <label for="master-token-input">Master Token</label>
          <input
            id="master-token-input"
            type="password"
            placeholder="输入后端持有的 APEIRETH_MASTER_TOKEN"
            bind:value={masterTokenDraft}
          />
          <small class="field-hint">Master Token 由您在启动后端时设定，AI 本身不接触此凭据。</small>
        </div>

        <div class="form-field">
          <label for="grant-hours-input">授权有效时长 (小时)</label>
          <input
            id="grant-hours-input"
            type="number"
            min="1"
            max="720"
            bind:value={grantHours}
          />
        </div>

        {#if grantError}
          <p class="error-text" role="alert">{grantError}</p>
        {/if}
        {#if grantSuccess}
          <p class="success-text">✅ 授权成功！时效权限已生效。</p>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="quiet-button" onclick={closeApprovalModal} disabled={grantBusy}>取消</button>
        <button class="primary-button" onclick={executeGrant} disabled={grantBusy || !masterTokenDraft.trim()}>

          {#if grantBusy}
            <span>正在授权…</span>
          {:else}
            <span>确认批准</span>
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .tools-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }
  .approval-banner {
    margin: 12px 32px 0;
    padding: 14px 18px;
    background: rgba(231, 162, 59, 0.08);
    border: 1px solid var(--amber-line);
    border-radius: 9px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .banner-head {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--amber);
  }
  .banner-desc {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .approval-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 4px;
  }
  .approval-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 6px;
  }
  .req-info {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }
  .req-tool {
    font-family: var(--mono);
    color: var(--amber);
    font-weight: 600;
  }
  .req-summary {
    color: var(--text);
  }
  .req-time {
    font-size: 10px;
    color: var(--faint);
    font-family: var(--mono);
  }
  .small-btn {
    padding: 4px 10px;
    font-size: 11px;
    gap: 4px;
  }

  .tools-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 32px 14px;
    border-bottom: 1px solid var(--line);
  }
  .search-input-wrap {
    flex: 1;
    min-width: 240px;
    max-width: 380px;
    position: relative;
    display: flex;
    align-items: center;
  }
  :global(.search-icon) {
    position: absolute;
    left: 10px;
    color: var(--faint);
  }
  .search-input-wrap input {
    width: 100%;
    padding: 7px 28px 7px 30px;
    background: var(--surface-2);
    border: 1px solid var(--line-strong);
    border-radius: 7px;
    color: var(--text);
    font-size: 12px;
    outline: 0;
  }
  .search-input-wrap input:focus {
    border-color: var(--amber-line);
  }
  .clear-search-btn {
    position: absolute;
    right: 8px;
    border: 0;
    background: transparent;
    color: var(--faint);
    cursor: pointer;
    display: grid;
    place-items: center;
  }
  .tools-summary-meta {
    font-size: 12px;
    color: var(--muted);
  }
  .tools-summary-meta b {
    color: var(--amber);
    font-family: var(--mono);
  }

  .tools-container {
    flex: 1;
    overflow-y: auto;
    padding: 20px 32px 40px;
  }
  .tools-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 14px;
  }
  .tool-card-box {
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 9px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .tool-card-box:hover {
    border-color: var(--line-strong);
    background: var(--surface-3);
    transform: translateY(-1px);
  }
  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .tool-id-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .tool-icon-box {
    color: var(--amber);
    display: grid;
    place-items: center;
  }
  .tool-title {
    font-size: 13px;
    font-family: var(--mono);
    color: var(--text);
  }
  .tool-desc {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
    line-height: 1.5;
    flex: 1;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;

  }
  .card-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 8px;
    border-top: 1px solid rgba(255, 255, 255, 0.04);
  }
  .source-pill {
    font-size: 10px;
    color: var(--faint);
  }
  .details-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 0;
    background: transparent;
    color: var(--amber);
    font-size: 11px;
    cursor: pointer;
    padding: 0;
  }
  .details-link:hover {
    text-decoration: underline;
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: grid;
    place-items: center;
    z-index: 1000;
    padding: 20px;
  }
  .modal-dialog {
    width: 100%;
    max-width: 520px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: 12px;
    box-shadow: var(--shadow);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    max-height: 85vh;
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px;
    border-bottom: 1px solid var(--line);
    background: var(--surface-2);
  }
  .modal-title-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .modal-title-wrap h3 {
    margin: 0;
    font-size: 14px;
    font-family: var(--mono);
    color: var(--text);
  }
  :global(.dialog-cpu-icon) { color: var(--amber); }
  :global(.dialog-shield-icon) { color: var(--amber); }
  .modal-close-btn {
    border: 0;
    background: transparent;
    color: var(--muted);
    padding: 4px;
    border-radius: 6px;
    cursor: pointer;
    display: grid;
    place-items: center;
  }
  .modal-body {
    padding: 18px 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .detail-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .detail-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--faint);
    text-transform: uppercase;
  }
  .desc-text {
    margin: 0;
    font-size: 13px;
    color: var(--text);
    line-height: 1.6;
  }
  .security-info {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
    background: var(--surface-2);
    padding: 8px 12px;
    border-radius: 6px;
    border: 1px solid var(--line);
  }
  :global(.sec-icon) {
    color: var(--green);
  }
  .schema-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
  }
  .toggle-schema-btn {
    border: 0;
    background: transparent;
    color: var(--amber);
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    cursor: pointer;
  }
  .schema-pre {
    margin: 6px 0 0;
    padding: 10px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--muted);
    font-family: var(--mono);
    font-size: 11px;
    white-space: pre-wrap;
    word-break: break-all;
    max-height: 220px;
    overflow-y: auto;
  }
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 12px 20px;
    border-top: 1px solid var(--line);
    background: var(--surface-2);
  }
  .primary-btn {
    padding: 6px 14px;
    border-radius: 6px;
    background: var(--amber);
    border: 1px solid var(--amber);
    color: #1a1408;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .form-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .form-field label {
    font-size: 12px;
    color: var(--muted);
    font-weight: 500;
  }
  .form-field input {
    padding: 8px 12px;
    background: var(--surface-2);
    border: 1px solid var(--line-strong);
    border-radius: 6px;
    color: var(--text);
    font-size: 13px;
    outline: 0;
  }
  .form-field input:focus {
    border-color: var(--amber-line);
  }
  .field-hint {
    font-size: 11px;
    color: var(--faint);
  }
  .error-text {
    margin: 0;
    color: var(--danger);
    font-size: 12px;
  }
  .success-text {
    margin: 0;
    color: var(--green);
    font-size: 12px;
  }
  .grant-intro code {
    font-family: var(--mono);
    color: var(--amber);
  }
  :global(.spin) {
    animation: spin 1s linear infinite;
  }
</style>
