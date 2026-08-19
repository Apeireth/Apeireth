<script lang="ts">
  import {onMount} from 'svelte';
  import {
    BrainCircuit,
    GitBranch,
    Layers3,
    Network,
    RefreshCw,
    Search,
    Sparkles,
    Star,
  } from 'lucide-svelte';
  import PageHeader from '../../components/PageHeader.svelte';
  import type {ApeirethConfig, MemoryCategory} from '../../lib/types';
  import {categoryToWire} from '../../lib/types';
  import {
    fetchGraphData,
    fetchMemoryStreams,
    fetchPanelEpisodes,
    type GraphData,
    type MemoryEpisode,
    type StreamEntry,
  } from '../../lib/runtime';

  let {
    config,
  }: {
    config: ApeirethConfig;
  } = $props();

  type Tab = 'memory' | 'graph' | 'reflection';

  let activeTab = $state<Tab>('memory');
  let episodes = $state<MemoryEpisode[]>([]);
  let graphData = $state<GraphData>({facts_count: 0, links_count: 0, facts: [], links: []});
  let reflectionStreams = $state<StreamEntry[]>([]);
  let loading = $state(false);
  let error = $state('');
  let searchQuery = $state('');
  let selectedCategory = $state<'全部' | MemoryCategory>('全部');
  let newMemory = $state('');
  let newCategory = $state<MemoryCategory>('事实');
  let appended = $state(false);

  // Graph search params
  let graphSubject = $state('');

  async function reload(): Promise<void> {
    loading = true;
    error = '';
    try {
      if (activeTab === 'memory') {
        episodes = await fetchPanelEpisodes(config, searchQuery || undefined, 100);
      } else if (activeTab === 'graph') {
        graphData = await fetchGraphData(config, {
          subject: graphSubject || undefined,
          limit: 100,
        });
      } else {
        reflectionStreams = await fetchMemoryStreams(config, 'reflection', 50);
      }
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
    } finally {
      loading = false;
    }
  }

  async function appendMemory(): Promise<void> {
    const text = newMemory.trim();
    if (!text) return;
    appended = false;
    try {
      const response = await fetch(
        `${config.baseUrl.replace(/\/+$/, '')}/v1/memory/append`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${config.apiKey}`,
          },
          body: JSON.stringify({
            session_id: 'companion-main',
            role: 'user',
            content: `[${categoryToWire(newCategory)}] ${text}`,
          }),
        },
      );
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      newMemory = '';
      appended = true;
      await reload();
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
    }
  }

  function formatTime(ts: number): string {
    const d = new Date(ts * 1000);
    return d.toLocaleString('zh-CN', {month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit'});
  }

  const filteredEpisodes = $derived(
    episodes.filter((e) => {
      if (selectedCategory === '全部') return true;
      const wire = categoryToWire(selectedCategory);
      return e.content.includes(`[${wire}]`) || e.content.includes(selectedCategory);
    }),
  );

  onMount(() => {
    void reload();
  });
</script>

<section class="view">
  <PageHeader
    eyebrow="认知"
    title="记忆与知识图谱"
    subtitle="Apeireth 原生记忆库 (episodes)、实体关系网络 (Knowledge Graph) 与自成长反思日志。"
  >
    <button class="quiet-button" onclick={reload} disabled={loading}>
      <RefreshCw size={14} />刷新
    </button>
  </PageHeader>

  <div class="memory-tabs">
    <button
      class:active={activeTab === 'memory'}
      onclick={() => { activeTab = 'memory'; void reload(); }}
    >
      <Layers3 size={15} />记忆条目
    </button>
    <button
      class:active={activeTab === 'graph'}
      onclick={() => { activeTab = 'graph'; void reload(); }}
    >
      <Network size={15} />知识图谱
    </button>
    <button
      class:active={activeTab === 'reflection'}
      onclick={() => { activeTab = 'reflection'; void reload(); }}
    >
      <Sparkles size={15} />自成长反思
    </button>
  </div>

  {#if error}
    <p class="error-banner" role="alert">{error}</p>
  {/if}

  {#if activeTab === 'memory'}
    <div class="memory-top-bar">
      <div class="search-box">
        <Search size={14} />
        <input
          bind:value={searchQuery}
          placeholder="搜索记忆关键词…"
          onkeydown={(e) => { if (e.key === 'Enter') void reload(); }}
        />
      </div>
      <div class="filters">
        {#each ['全部', '事实', '偏好', '事件', '反馈', '参考'] as cat}
          <button
            class:active={selectedCategory === cat}
            onclick={() => selectedCategory = cat as '全部' | MemoryCategory}
          >
            {cat}
          </button>
        {/each}
      </div>
    </div>

    <div class="memory-appender">
      <textarea bind:value={newMemory} rows="2" placeholder="追加一条记忆（写入后端 memory_store）"></textarea>
      <div class="memory-appender-row">
        <select bind:value={newCategory}>
          <option value="事实">事实</option>
          <option value="偏好">偏好</option>
          <option value="事件">事件</option>
          <option value="反馈">反馈</option>
          <option value="参考">参考</option>
        </select>
        <button class="primary-button" onclick={appendMemory} disabled={!newMemory.trim()}>追加</button>
      </div>
      {#if appended}
        <small class="appended-hint">已写入后端 memory_store。</small>
      {/if}
    </div>

    <div class="episode-list">
      {#if loading && !episodes.length}
        <p class="dim-hint">正在加载记忆…</p>
      {:else if !filteredEpisodes.length}
        <div class="blank-state">
          <div class="blank-mark">⌁</div>
          <h3>暂无记忆条目</h3>
          <p>对话中的重要信息会被后端自动提取并持久化到 SQLite。</p>
        </div>
      {:else}
        {#each filteredEpisodes as episode}
          <article class="episode-card">
            <div class="episode-head">
              <span class="badge blue">{episode.role}</span>
              <time>{formatTime(episode.timestamp)}</time>
              <small>{episode.id.slice(0, 14)}</small>
            </div>
            <p>{episode.content}</p>
          </article>
        {/each}
      {/if}
    </div>
  {:else if activeTab === 'graph'}
    <div class="graph-top-bar">
      <div class="search-box">
        <Search size={14} />
        <input
          bind:value={graphSubject}
          placeholder="按主体过滤实体关系 (例如: 主人, 阿佩瑞斯)…"
          onkeydown={(e) => { if (e.key === 'Enter') void reload(); }}
        />
      </div>
      <button class="quiet-button" onclick={reload}>查询图谱</button>
      <span class="graph-counts-badge">
        {graphData.facts_count} 事实 · {graphData.links_count} 关系链接
      </span>
    </div>

    <div class="graph-view-container">
      {#if loading && !graphData.facts.length}
        <p class="dim-hint">正在探索知识图谱网络…</p>
      {:else if !graphData.facts.length && !graphData.links.length}
        <div class="blank-state">
          <div class="blank-mark">⌁</div>
          <h3>知识图谱尚未形成实体关系</h3>
          <p>伴随体在与您的长期对话和反思中，会自动构建和关联概念图谱。</p>
        </div>
      {:else}
        <div class="graph-triples-grid">
          {#each graphData.facts as fact}
            <article class="graph-triple-card">
              <div class="triple-node subject-node">
                <span>主体</span>
                <strong>{fact.subject}</strong>
              </div>
              <div class="triple-edge">
                <GitBranch size={13} />
                <code>{fact.predicate}</code>
              </div>
              <div class="triple-node object-node">
                <span>客体</span>
                <strong>{fact.object}</strong>
              </div>
            </article>
          {/each}
        </div>
      {/if}
    </div>
  {:else}
    <div class="reflection-list">
      {#if loading && !reflectionStreams.length}
        <p class="dim-hint">正在加载自成长日志…</p>
      {:else if !reflectionStreams.length}
        <div class="blank-state">
          <div class="blank-mark">⌁</div>
          <h3>暂无自成长与反思日志</h3>
          <p>后端常驻守护进程 (CompanionDaemon) 会在安静期自主执行周期深度反思与做梦提炼。</p>
        </div>
      {:else}
        {#each reflectionStreams as stream}
          <article class="reflection-card">
            <div class="reflection-head">
              <div class="reflection-title-box">
                <Sparkles size={14} class="exec-icon-accent" />
                <strong>周期成长反思</strong>
              </div>
              <time>{formatTime(stream.created_at)}</time>
            </div>
            <p class="reflection-payload">{stream.payload}</p>
            <div class="reflection-footer">
              <small>来源: {stream.source || 'CompanionDaemon'}</small>
              <small>ID: {stream.id.slice(0, 12)}</small>
            </div>
          </article>
        {/each}
      {/if}
    </div>
  {/if}
</section>
