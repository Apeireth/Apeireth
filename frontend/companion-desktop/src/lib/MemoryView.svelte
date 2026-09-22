<script lang="ts">
  // 记忆卷宗（Archive 纸面档案调 · §5.6② 首次实拍）——主从双栏。
  // 左：过滤器 chips（带计数，只认真实字段 role/protected）+ episodes 列表；
  // 右：详情（全文/元数据/图谱关联/治理操作 protect·unprotect·forget）。
  //
  // 诚实边界（契约 §5 rc）：
  // - schema 不存 category/importance → 缺省显「未分类」/不显示，禁编分类；
  // - 过滤器不再做 工作/近期/长期/画像 假分类 tab（旧版已拆除）；
  // - 乐观锁 expected_rev：客户端 rev 账本从 0 起、随响应 revision 更新；
  //   409 = 别处改过 → 真实冲突卡，不静默重试；
  // - episodes 列表端点不带 revision——冲突后无法自愈重试，冲突卡如实写明。
  import {
    Search,
    X,
    RotateCcw,
    Plus,
    Lock,
    LockOpen,
    Trash2,
    Share2,
  } from 'lucide-svelte';
  import EmptyState from './components/EmptyState.svelte';
  import ErrorState from './components/ErrorState.svelte';
  import LoadingState from './components/LoadingState.svelte';
  import type {ApeirethConfig, CapabilityManifest, MemoryEpisodeItem} from './types';
  import {
    appendMemoryEpisode,
    fetchGraphData,
    fetchMemoryEpisodes,
    forgetMemoryEpisode,
    protectMemoryEpisode,
    unprotectMemoryEpisode,
    capabilityAvailable,
    capabilitySupported,
    friendlyErrorMessage,
  } from './runtime';
  import {
    MEMORY_FILTERS,
    filterEpisodes,
    memoryFilterCounts,
    episodeCategoryLabel,
    episodeImportanceText,
    expectedRevOf,
    recordRevision,
    classifyMemoryMutationError,
    episodeGraphLinks,
    graphNodeLabel,
    ghostNumber,
    formatEpisodeTime,
    type MemoryFilter,
    type RevisionLedger,
    type MemoryGraphNode,
    type MemoryGraphEdge,
  } from './memory-ledger';

  let {
    config,
    capabilities = null,
  }: {
    config: ApeirethConfig;
    capabilities: CapabilityManifest | null;
  } = $props();

  /** 拉取窗口：端点不供总数，窗口口径（满窗不代表全部）。 */
  const EPISODES_WINDOW = 120;

  // ---- 能力门（不 404-probe）----
  const canRead = $derived(capabilitySupported(capabilities, 'memory.read') && capabilityAvailable(capabilities, 'memory.read'));
  const canWrite = $derived(capabilitySupported(capabilities, 'memory.write') && capabilityAvailable(capabilities, 'memory.write'));
  const canForget = $derived(capabilitySupported(capabilities, 'memory.forget') && capabilityAvailable(capabilities, 'memory.forget'));
  const canProtect = $derived(capabilitySupported(capabilities, 'memory.protect') && capabilityAvailable(capabilities, 'memory.protect'));
  const canUnprotect = $derived(capabilitySupported(capabilities, 'memory.unprotect') && capabilityAvailable(capabilities, 'memory.unprotect'));
  const canGraph = $derived(capabilitySupported(capabilities, 'memory.graph.read') && capabilityAvailable(capabilities, 'memory.graph.read'));

  // ---- 列表状态 ----
  let episodes = $state<MemoryEpisodeItem[]>([]);
  let loading = $state(false);
  let error = $state('');
  let searchQuery = $state('');
  /** 服务端搜索词（回车/清除才触发重拉；输入过程不打扰）。 */
  let appliedQuery = $state('');
  /** 会话过滤（点 session 徽章落过滤；契约 ?session= 要求 UUID）。 */
  let sessionFilter = $state('');
  let filter = $state<MemoryFilter>('all');
  let selectedId = $state<string | null>(null);

  // ---- 治理状态 ----
  let revLedger = $state<RevisionLedger>({});
  let mutatingId = $state<string | null>(null);
  let mutationError = $state('');
  /** 409 冲突的条目 id——详情区冲突卡，换条目/操作成功才清。 */
  let conflictId = $state<string | null>(null);
  /** forget 红边内联确认卡（§4.4 组件 13：不可逆操作用红边卡，不用二次弹窗）。 */
  let forgetArmedId = $state<string | null>(null);

  // ---- 图谱 ----
  let graphNodes = $state<MemoryGraphNode[] | null>(null);
  let graphEdges = $state<MemoryGraphEdge[]>([]);
  let graphNote = $state('');
  let graphLoading = $state(false);

  // ---- 写入 ----
  let showAppender = $state(false);
  let newContent = $state('');
  let appending = $state(false);
  let appendSuccess = $state(false);

  const counts = $derived(memoryFilterCounts(episodes));
  const visibleList = $derived(filterEpisodes(episodes, filter));
  const selected = $derived(episodes.find((e) => e.id === selectedId) ?? null);
  const selectedIndex = $derived(selected ? visibleList.indexOf(selected) : -1);
  const selectedGraph = $derived(
    selected && graphNodes
      ? episodeGraphLinks(selected.id, graphNodes, graphEdges)
      : null,
  );

  async function loadData(): Promise<void> {
    if (!canRead) {
      error = '记忆流未开通：当前运行时未声明 memory.read（Apeireth 2.0 canonical gateway 无此内省 API）。';
      episodes = [];
      return;
    }
    loading = true;
    error = '';
    try {
      episodes = await fetchMemoryEpisodes(config, appliedQuery, EPISODES_WINDOW, sessionFilter);
      if (selectedId && !episodes.some((e) => e.id === selectedId)) selectedId = null;
    } catch (e) {
      error = friendlyErrorMessage(e, '/v1/panel/memory/episodes');
    } finally {
      loading = false;
    }
  }

  function applySearch(): void {
    appliedQuery = searchQuery.trim();
    void loadData();
  }

  function clearSearch(): void {
    searchQuery = '';
    appliedQuery = '';
    void loadData();
  }

  function filterBySession(sessionId: string): void {
    sessionFilter = sessionId;
    void loadData();
  }

  function clearSessionFilter(): void {
    sessionFilter = '';
    void loadData();
  }

  function selectEpisode(ep: MemoryEpisodeItem): void {
    selectedId = ep.id;
    conflictId = null;
    forgetArmedId = null;
    mutationError = '';
    void ensureGraph();
  }

  /** 图谱按需拉一次：能力门内才拉；不可用/失败的口径写进 graphNote（0 装）。 */
  async function ensureGraph(): Promise<void> {
    if (graphNodes || graphLoading) return;
    if (!canGraph) {
      graphNote = '图谱未开通：当前运行时未声明 memory.graph.read——这里只显示条目本身。';
      return;
    }
    graphLoading = true;
    try {
      const g = await fetchGraphData(config);
      graphNodes = g.nodes;
      graphEdges = g.edges;
      graphNote = '';
    } catch (e) {
      graphNote = `图谱拉取失败（${friendlyErrorMessage(e, '/v1/panel/graph')}）——这里只显示条目本身。`;
    } finally {
      graphLoading = false;
    }
  }

  /** 治理操作统一入口：乐观锁 + 响应回写 + 409 真实冲突态。 */
  async function runMutation(
    ep: MemoryEpisodeItem,
    op: 'forget' | 'protect' | 'unprotect',
  ): Promise<void> {
    if (mutatingId) return;
    mutatingId = ep.id;
    mutationError = '';
    const rev = expectedRevOf(revLedger, ep.id);
    const r =
      op === 'forget'
        ? await forgetMemoryEpisode(config, ep.id, rev, '主人在记忆卷宗里手动遗忘')
        : op === 'protect'
          ? await protectMemoryEpisode(config, ep.id, rev)
          : await unprotectMemoryEpisode(config, ep.id, rev);
    mutatingId = null;
    if ('error' in r) {
      const outcome = classifyMemoryMutationError(r);
      if (outcome.kind === 'conflict') {
        conflictId = ep.id;
        await loadData(); // 重新对齐列表（端点不带 revision，无法自动重试——冲突卡如实说明）
      } else {
        mutationError = outcome.message;
      }
      return;
    }
    conflictId = null;
    forgetArmedId = null;
    revLedger = recordRevision(revLedger, ep.id, r.revision);
    if (op === 'forget') {
      episodes = episodes.filter((item) => item.id !== ep.id);
      if (selectedId === ep.id) selectedId = null;
    } else {
      episodes = episodes.map((item) =>
        item.id === ep.id ? {...item, protected: r.protected, status: r.status} : item,
      );
    }
  }

  async function handleAppend(): Promise<void> {
    const text = newContent.trim();
    if (!text || appending) return;
    appending = true;
    appendSuccess = false;
    const ok = await appendMemoryEpisode(config, text);
    appending = false;
    if (ok) {
      appendSuccess = true;
      newContent = '';
      setTimeout(() => {
        appendSuccess = false;
        showAppender = false;
      }, 900);
      void loadData();
    } else {
      mutationError = '写入失败：后端未收下这条（请确认连接正常）。';
    }
  }

  // 能力清单异步到达（App 启动节拍里拉取）：drawer 可能在 capabilities=null 时
  // 挂载，onMount 一次性早退会永久停在「未开通」误报——首载改由能力翻转到位的
  // effect 触发（一次性闸，治理卷宗 ApprovalsTab 同款先例 40f78845）。
  let loadStarted = false;
  $effect(() => {
    if (!canRead || loadStarted) return;
    loadStarted = true;
    void loadData();
  });
</script>

<section class="memory-view">
  <!-- 工具行：搜索（服务端 q）+ chips（客户端过滤计数）+ 写入/刷新 -->
  <div class="mv-toolbar">
    <div class="mv-search">
      <Search size={13} />
      <input
        type="text"
        placeholder="搜索记忆文本，回车生效…"
        bind:value={searchQuery}
        onkeydown={(e) => e.key === 'Enter' && applySearch()}
      />
      {#if appliedQuery}
        <button class="mv-icon-btn" onclick={clearSearch} aria-label="清除搜索" title="清除搜索">
          <X size={12} />
        </button>
      {/if}
    </div>

    <div class="mv-chips" role="group" aria-label="记忆过滤">
      {#each MEMORY_FILTERS as f (f.id)}
        <button
          class="mv-chip"
          class:on={filter === f.id}
          onclick={() => (filter = f.id)}
          title={`按当前拉取窗口的 ${counts.all} 条统计`}
        >
          {f.label}
          <span class="mv-chip-count">{counts[f.id]}</span>
        </button>
      {/each}
      {#if sessionFilter}
        <button class="mv-chip session-chip on" onclick={clearSessionFilter} title="清除会话过滤">
          会话 {sessionFilter.slice(0, 8)}…
          <X size={11} />
        </button>
      {/if}
    </div>

    <div class="mv-toolbar-actions">
      {#if canWrite}
        <button class="mv-btn-quiet" onclick={() => (showAppender = !showAppender)}>
          <Plus size={13} />
          <span>写入</span>
        </button>
      {/if}
      <button class="mv-btn-quiet" onclick={() => void loadData()} disabled={loading} title="重新拉取">
        <RotateCcw size={13} class={loading ? 'spin' : ''} />
        <span>刷新</span>
      </button>
    </div>
  </div>

  {#if showAppender && canWrite}
    <div class="mv-appender">
      <textarea
        bind:value={newContent}
        rows="2"
        placeholder="要他记住的事（如你的偏好、关键约定）……写进他的面板专用会话账本。"
        disabled={appending}
      ></textarea>
      <div class="mv-appender-foot">
        {#if appendSuccess}
          <span class="mv-ok">已写入他的账本</span>
        {/if}
        <button class="mv-btn-gold" onclick={() => void handleAppend()} disabled={appending || !newContent.trim()}>
          {appending ? '正在写入…' : '写入记忆'}
        </button>
      </div>
    </div>
  {/if}

  <!-- 主从双栏 -->
  <div class="mv-columns">
    <!-- 左：列表 -->
    <div class="mv-list" role="listbox" aria-label="记忆列表">
      {#if capabilities === null}
        <!-- 清单未到达 ≠ 不支持：health→capabilities 串行拉取进行中，诚实显加载 -->
        <LoadingState message="正在读取运行时能力清单…" />
      {:else if !canRead}
        <EmptyState
          icon="🧠"
          title="记忆流未开通"
          description="当前运行时未声明 memory.read——此页不编造条目；开通后这里会出现他的真实记忆账本。"
        />
      {:else if loading && !episodes.length}
        <LoadingState message="正在对齐他的记忆账本…" />
      {:else if error && !episodes.length}
        <ErrorState title="拉取记忆失败" message={error} onRetry={() => void loadData()} />
      {:else if !episodes.length}
        <EmptyState
          icon="🧠"
          title={appliedQuery || sessionFilter ? '当前窗口里没有匹配' : '记忆卷宗还是空的'}
          description={appliedQuery || sessionFilter
            ? '换个搜索词，或清除过滤条件——这里只显示当前拉取窗口内的真实条目。'
            : '当他记住什么时，这里会出现——对话里值得留住的片段，会一条一条躺在这。'}
        />
      {:else if !visibleList.length}
        <EmptyState
          icon="🧠"
          title="这个过滤桶里没有条目"
          description="计数按当前拉取窗口统计——换个过滤，或拉取更多记忆。"
        />
      {:else}
        {#each visibleList as ep, i (ep.id)}
          <button
            class="mv-row"
            class:on={selectedId === ep.id}
            role="option"
            aria-selected={selectedId === ep.id}
            onclick={() => selectEpisode(ep)}
          >
            <span class="mv-row-no">{ghostNumber(i)}</span>
            <span class="mv-diamond" class:gold={ep.role === 'assistant'} aria-hidden="true"></span>
            <span class="mv-row-body">
              <span class="mv-row-text">{ep.content}</span>
              <span class="mv-row-meta">
                <time>{formatEpisodeTime(ep.timestamp)}</time>
                {#if ep.protected}
                  <Lock size={10} />
                {/if}
                {#if ep.status === 'forgotten'}
                  <span class="mv-forgotten-tag">已遗忘</span>
                {/if}
              </span>
            </span>
          </button>
        {/each}
        <p class="mv-window-note">当前窗口 {episodes.length} 条（limit {EPISODES_WINDOW}）——端点不供总数。</p>
      {/if}
    </div>

    <!-- 右：详情 -->
    <div class="mv-detail">
      {#if selected}
        <span class="mv-ghost" aria-hidden="true">{selectedIndex >= 0 ? ghostNumber(selectedIndex) : '—'}</span>

        <header class="mv-detail-head">
          <span class="mv-detail-id">#{selected.id.slice(0, 8)}</span>
          <time class="mv-detail-time">{formatEpisodeTime(selected.timestamp)}</time>
        </header>

        <p class="mv-detail-content">{selected.content}</p>

        <div class="mv-meta-grid">
          <div>
            <span class="mv-meta-label">谁说的</span>
            <span class="mv-meta-value">{selected.role === 'assistant' ? '他说的' : selected.role === 'user' ? '主人说的' : selected.role}</span>
          </div>
          <div>
            <span class="mv-meta-label">类别</span>
            <span class="mv-meta-value" class:mv-meta-absent={!selected.category}>{episodeCategoryLabel(selected)}</span>
          </div>
          {#if episodeImportanceText(selected) !== null}
            <div>
              <span class="mv-meta-label">重要度</span>
              <span class="mv-meta-value">{episodeImportanceText(selected)}</span>
            </div>
          {/if}
          <div>
            <span class="mv-meta-label">治理状态</span>
            <span class="mv-meta-value">
              {selected.status === 'forgotten' ? '已遗忘' : '在册'}{selected.protected ? ' · 已保护' : ''}
            </span>
          </div>
          <div>
            <span class="mv-meta-label">所属会话</span>
            <button
              class="mv-meta-value mv-session-link"
              onclick={() => filterBySession(selected.sessionId)}
              title="只看这个会话的记忆"
            >
              {selected.sessionId.slice(0, 8)}…
            </button>
          </div>
        </div>

        <!-- 图谱关联（真数据：/v1/panel/graph 的 session→episode 包含边） -->
        <div class="mv-graph">
          <h4 class="mv-graph-title">
            <Share2 size={12} />
            图谱关联
          </h4>
          {#if graphNote}
            <p class="mv-graph-note">{graphNote}</p>
          {:else if graphLoading && !graphNodes}
            <p class="mv-graph-note">正在拉取图谱…</p>
          {:else if selectedGraph}
            {#if selectedGraph.edges.length === 0}
              <p class="mv-graph-note">这条记忆还没有被织进图谱——当它与会话账本连上边时，这里会出现。</p>
            {:else}
              <ul class="mv-graph-edges">
                {#each selectedGraph.edges as edge (edge.from + edge.to)}
                  <li>
                    <span class="mv-edge-node">{graphNodeLabel(edge.from, graphNodes ?? [])}</span>
                    <span class="mv-edge-arrow">→</span>
                    <span class="mv-edge-node">{graphNodeLabel(edge.to, graphNodes ?? [])}</span>
                    {#if typeof edge.weight === 'number'}
                      <span class="mv-edge-weight">权重 {edge.weight}</span>
                    {/if}
                    {#if edge.label}
                      <span class="mv-edge-label">{edge.label}</span>
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}
          {/if}
        </div>

        <!-- 治理操作 -->
        {#if conflictId === selected.id}
          <div class="mv-conflict" role="alert">
            <strong>修订冲突（409）</strong>
            <p>
              这条记忆在别处被改动过，你手里的修订号已过期。列表已重新拉取对齐；
              由于端点不在列表里带回修订号，前端无法自动重试——若再次操作仍冲突，
              说明它正被另一边持续改动。
            </p>
          </div>
        {/if}
        {#if mutationError}
          <div class="mv-error" role="alert">操作失败：{mutationError}</div>
        {/if}

        <div class="mv-actions">
          {#if selected.protected && canUnprotect}
            <button
              class="mv-btn-quiet"
              disabled={mutatingId === selected.id}
              onclick={() => void runMutation(selected, 'unprotect')}
            >
              <LockOpen size={12} />
              <span>{mutatingId === selected.id ? '操作中…' : '取消保护'}</span>
            </button>
          {:else if !selected.protected && canProtect}
            <button
              class="mv-btn-quiet"
              disabled={mutatingId === selected.id}
              onclick={() => void runMutation(selected, 'protect')}
              title="保护后不会被自动遗忘"
            >
              <Lock size={12} />
              <span>{mutatingId === selected.id ? '操作中…' : '保护'}</span>
            </button>
          {/if}
          {#if canForget && selected.status !== 'forgotten'}
            {#if forgetArmedId === selected.id}
              <div class="mv-forget-arm">
                <p>遗忘 = 软删（从检索隐藏，保留审计），不可自行撤销。确认？</p>
                <div class="mv-forget-arm-btns">
                  <button
                    class="mv-btn-danger-solid"
                    disabled={mutatingId === selected.id}
                    onclick={() => void runMutation(selected, 'forget')}
                  >
                    {mutatingId === selected.id ? '正在遗忘…' : '确认遗忘'}
                  </button>
                  <button class="mv-btn-quiet" onclick={() => (forgetArmedId = null)}>取消</button>
                </div>
              </div>
            {:else}
              <button
                class="mv-btn-danger"
                disabled={mutatingId === selected.id || !!selected.protected}
                title={selected.protected ? '已保护的条目先取消保护才能遗忘' : '软删此条（可审计）'}
                onclick={() => (forgetArmedId = selected.id)}
              >
                <Trash2 size={12} />
                <span>遗忘</span>
              </button>
            {/if}
          {/if}
          {#if !canForget && !canProtect && !canUnprotect}
            <p class="mv-actions-note">治理操作未开放：当前运行时只声明了读取——此页为只读卷宗。</p>
          {/if}
        </div>
      {:else}
        <span class="mv-ghost" aria-hidden="true">00</span>
        <div class="mv-detail-empty">
          <span class="mv-diamond lg" aria-hidden="true"></span>
          <p>从左边选一条记忆——<br />这里会出现它的全文、出处、图谱关联与治理操作。</p>
        </div>
      {/if}
    </div>
  </div>
</section>

<style>
  /* ── Archive 纸面档案调（§5.6② 首次实拍）：页面层调性，不随全局主题换。
     局部覆写中性变量 → 共享组件（EmptyState/ErrorState/LoadingState）自动墨化。 ── */
  .memory-view {
    --text: var(--ap-register-archive-ink);
    --muted: rgba(38, 38, 42, 0.6);
    --faint: rgba(38, 38, 42, 0.4);
    --line: rgba(38, 38, 42, 0.14);
    --line-strong: rgba(38, 38, 42, 0.26);
    --surface: var(--ap-register-archive-paper-hi);
    --surface-2: #ecebe9;
    --surface-3: #e5e4e2;
    /* 文字级强调：纸面可读的下沉金（实拍校准提案，tokens.css §5.6② 段注记） */
    --amber: var(--ap-register-archive-ink-gold);
    /* 语义色纸面口径（借 day/essence 已校准的深值组） */
    --ap-semantic-danger: #a84840;
    --ap-semantic-warning: #9a6f2e;

    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--ap-register-archive-paper);
    color: var(--ap-register-archive-ink);
  }

  /* ── 工具行 ── */
  .mv-toolbar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    padding: 12px 20px;
    border-bottom: 1px solid var(--line);
  }
  .mv-search {
    position: relative;
    display: flex;
    align-items: center;
    gap: 7px;
    flex: 1;
    min-width: 200px;
    max-width: 320px;
    color: var(--faint);
  }
  .mv-search input {
    width: 100%;
    padding: 6px 26px 6px 8px;
    background: transparent;
    border: 0;
    border-bottom: 1px solid var(--line-strong);
    border-radius: 0;
    color: var(--text);
    font-size: 12.5px;
    outline: 0;
  }
  .mv-search input:focus {
    border-bottom-color: var(--ap-register-archive-ink-gold);
  }
  .mv-icon-btn {
    position: absolute;
    right: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    cursor: pointer;
    padding: 3px;
  }
  .mv-chips {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .mv-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border: 1px solid var(--line-strong);
    border-radius: 999px;
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease, border-color 0.15s ease;
  }
  .mv-chip:hover {
    color: var(--text);
    border-color: var(--ap-register-archive-ink);
  }
  /* chips 选中 = 金底墨字（§4.4-7「选中 chip 用金」在纸面的可读落地：
     金字在纸白上对比不足，取实体金底 + 墨字——金色纪律的语义不变） */
  .mv-chip.on {
    background: var(--ap-register-archive-accent);
    border-color: var(--ap-register-archive-accent);
    color: #26262a;
  }
  .mv-chip-count {
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    opacity: 0.72;
  }
  .session-chip {
    font-family: var(--ap-font-mono);
    letter-spacing: 0.04em;
  }
  .mv-toolbar-actions {
    margin-left: auto;
    display: flex;
    gap: 8px;
  }

  .mv-btn-quiet {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 12px;
    border: 1px solid var(--line-strong);
    border-radius: 6px;
    background: transparent;
    color: var(--text);
    font-size: 12px;
    cursor: pointer;
  }
  .mv-btn-quiet:hover:not(:disabled) {
    border-color: var(--ap-register-archive-ink);
  }
  .mv-btn-quiet:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .mv-btn-gold {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 14px;
    border: 1px solid var(--ap-register-archive-accent);
    border-radius: 6px;
    background: var(--ap-register-archive-accent);
    color: #26262a;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .mv-btn-gold:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* ── 写入卡 ── */
  .mv-appender {
    flex: none;
    margin: 10px 20px 0;
    padding: 12px 14px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 9px;
  }
  .mv-appender textarea {
    width: 100%;
    resize: none;
    border: 1px solid var(--line-strong);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    padding: 8px 10px;
    font-size: 12.5px;
    outline: 0;
  }
  .mv-appender textarea:focus {
    border-color: var(--ap-register-archive-ink-gold);
  }
  .mv-appender-foot {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
  }
  .mv-ok {
    color: #3d7a5c;
    font-size: 11.5px;
  }

  /* ── 主从双栏 ── */
  .mv-columns {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .mv-list {
    flex: none;
    width: min(340px, 44%);
    border-right: 1px solid var(--line);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding: 8px 0 16px;
  }
  .mv-row {
    display: flex;
    align-items: flex-start;
    gap: 9px;
    padding: 10px 14px 10px 12px;
    border: 0;
    border-left: 3px solid transparent;
    background: transparent;
    text-align: left;
    cursor: pointer;
    color: var(--text);
  }
  .mv-row:hover {
    background: var(--surface-2);
  }
  /* 选中 = 金色左竖条（存在金 = 他；条目被选中的强调语义） */
  .mv-row.on {
    border-left-color: var(--ap-register-archive-accent);
    background: var(--surface-2);
  }
  .mv-row-no {
    flex: none;
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.28em;
    color: var(--faint);
    padding-top: 3px;
  }
  /* 橙菱形：条目 bullet（§5.6② 标记职能，不承担强调）；他说 = 金菱形（他在场） */
  .mv-diamond {
    flex: none;
    width: 7px;
    height: 7px;
    margin-top: 5px;
    transform: rotate(45deg);
    background: var(--ap-register-archive-marker-orange);
  }
  .mv-diamond.gold {
    background: var(--ap-register-archive-accent);
    border: 1px solid rgba(38, 38, 42, 0.35);
  }
  .mv-diamond.lg {
    width: 12px;
    height: 12px;
    margin: 0;
  }
  .mv-row-body {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .mv-row-text {
    font-size: 12.5px;
    line-height: 1.55;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .mv-row-meta {
    display: flex;
    align-items: center;
    gap: 7px;
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    color: var(--faint);
  }
  .mv-forgotten-tag {
    border: 1px solid var(--line-strong);
    border-radius: 3px;
    padding: 0 4px;
  }
  .mv-window-note {
    margin: auto 14px 0;
    padding-top: 10px;
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    color: var(--faint);
  }

  /* ── 详情 ── */
  .mv-detail {
    position: relative;
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: 22px 26px 40px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  /* 巨型幽灵编号（§5.6② archive.ghost；§4.4-4 Archive 版主从用幽灵编号背景） */
  .mv-ghost {
    position: absolute;
    top: -14px;
    right: 6px;
    font-family: var(--ap-font-mono);
    font-size: clamp(96px, 13vw, 150px);
    font-weight: 700;
    line-height: 1;
    letter-spacing: -0.04em;
    color: var(--ap-register-archive-ghost-number);
    opacity: 0.6;
    pointer-events: none;
    user-select: none;
  }
  .mv-detail-head {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  .mv-detail-id {
    font-family: var(--ap-font-mono);
    font-size: 11px;
    letter-spacing: 0.28em;
    color: var(--faint);
  }
  .mv-detail-time {
    font-family: var(--ap-font-mono);
    font-size: 11px;
    letter-spacing: 0.08em;
    color: var(--muted);
  }
  .mv-detail-content {
    position: relative;
    margin: 0;
    max-width: 58ch;
    font-size: 14px;
    line-height: 1.85;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .mv-meta-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    gap: 10px 18px;
    padding: 12px 14px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 8px;
  }
  .mv-meta-label {
    display: block;
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.28em;
    color: var(--faint);
    margin-bottom: 3px;
  }
  .mv-meta-value {
    font-size: 12.5px;
    color: var(--text);
  }
  .mv-meta-absent {
    color: var(--faint);
  }
  .mv-session-link {
    border: 0;
    background: transparent;
    padding: 0;
    cursor: pointer;
    font-family: var(--ap-font-mono);
    font-size: 12px;
    letter-spacing: 0.06em;
    color: var(--ap-register-archive-ink-gold);
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  /* ── 图谱关联 ── */
  .mv-graph {
    border-top: 1px solid var(--line);
    padding-top: 12px;
  }
  .mv-graph-title {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 8px;
    font-family: var(--ap-font-mono);
    font-size: 10px;
    font-weight: 400;
    letter-spacing: 0.28em;
    color: var(--faint);
  }
  .mv-graph-note {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
    line-height: 1.7;
  }
  .mv-graph-edges {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .mv-graph-edges li {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 12px;
    padding: 7px 10px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 6px;
  }
  .mv-edge-node {
    color: var(--text);
  }
  .mv-edge-arrow {
    color: var(--faint);
  }
  .mv-edge-weight,
  .mv-edge-label {
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    color: var(--faint);
  }

  /* ── 冲突 / 错误 / 操作 ── */
  .mv-conflict {
    border: 1px solid var(--ap-semantic-warning);
    border-left-width: 3px;
    border-radius: 6px;
    padding: 10px 14px;
    background: rgba(154, 111, 46, 0.08);
  }
  .mv-conflict strong {
    font-size: 12.5px;
    color: var(--ap-semantic-warning);
  }
  .mv-conflict p {
    margin: 5px 0 0;
    font-size: 12px;
    line-height: 1.7;
    color: var(--muted);
  }
  .mv-error {
    border: 1px solid var(--ap-semantic-danger);
    border-radius: 6px;
    padding: 9px 12px;
    font-size: 12px;
    color: var(--ap-semantic-danger);
    background: rgba(168, 72, 64, 0.07);
  }
  .mv-actions {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    flex-wrap: wrap;
    border-top: 1px solid var(--line);
    padding-top: 14px;
  }
  .mv-actions-note {
    margin: 0;
    font-size: 12px;
    color: var(--faint);
  }
  /* forget：不可逆操作 = 红色边框卡内联确认（§4.4-13，不用二次弹窗） */
  .mv-btn-danger {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 12px;
    border: 1px solid var(--ap-semantic-danger);
    border-radius: 6px;
    background: transparent;
    color: var(--ap-semantic-danger);
    font-size: 12px;
    cursor: pointer;
  }
  .mv-btn-danger:hover:not(:disabled) {
    background: rgba(168, 72, 64, 0.08);
  }
  .mv-btn-danger:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .mv-forget-arm {
    border: 1px solid var(--ap-semantic-danger);
    border-radius: 8px;
    padding: 11px 14px;
    background: rgba(168, 72, 64, 0.06);
    display: flex;
    flex-direction: column;
    gap: 9px;
  }
  .mv-forget-arm p {
    margin: 0;
    font-size: 12px;
    color: var(--text);
    line-height: 1.6;
  }
  .mv-forget-arm-btns {
    display: flex;
    gap: 8px;
  }
  .mv-btn-danger-solid {
    padding: 5px 14px;
    border: 1px solid var(--ap-semantic-danger);
    border-radius: 6px;
    background: var(--ap-semantic-danger);
    color: #f7f5f1;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .mv-btn-danger-solid:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .mv-detail-empty {
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    text-align: center;
    color: var(--muted);
    font-size: 13px;
    line-height: 1.9;
  }
  .mv-detail-empty p {
    margin: 0;
  }
  :global(.spin) {
    animation: spin 1s linear infinite;
  }
  @media (max-width: 900px) {
    .mv-columns {
      flex-direction: column;
    }
    .mv-list {
      width: 100%;
      border-right: 0;
      border-bottom: 1px solid var(--line);
      max-height: 40%;
    }
  }
</style>
