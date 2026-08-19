<script lang="ts">
  import {onMount} from 'svelte';
  import {Layers3, Wrench, BrainCircuit, RefreshCw} from 'lucide-svelte';
  import PageHeader from '../../components/PageHeader.svelte';
  import type {ApeirethConfig, MemoryCategory} from '../../lib/types';
  import {categoryFromWire, categoryToWire} from '../../lib/types';
  import {fetchEpisodes, fetchOrgans, fetchTools, type MemoryEpisode, type ToolInfo} from '../../lib/runtime';

  let {
    config,
  }: {
    config: ApeirethConfig;
  } = $props();

  type Tab = 'memory' | 'tools' | 'organs';

  let activeTab = $state<Tab>('memory');
  let episodes = $state<MemoryEpisode[]>([]);
  let tools = $state<ToolInfo[]>([]);
  let organs = $state<unknown[]>([]);
  let loading = $state(false);
  let error = $state('');
  let newMemory = $state('');
  let newCategory = $state<MemoryCategory>('事实');
  let appended = $state(false);

  async function reload(): Promise<void> {
    loading = true;
    error = '';
    try {
      if (activeTab === 'memory') {
        episodes = await fetchEpisodes(config, 100);
      } else if (activeTab === 'tools') {
        tools = await fetchTools(config);
      } else {
        organs = await fetchOrgans(config);
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
    return new Date(ts * 1000).toLocaleString('zh-CN', {month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit'});
  }

  onMount(() => {
    void reload();
  });
</script>

<section class="view">
  <PageHeader eyebrow="认知" title="记忆与能力" subtitle="Apeireth 后端记忆 (episodes)、工具注册表与器官状态。">
    <button class="quiet-button" onclick={reload} disabled={loading}><RefreshCw size={14}/>刷新</button>
  </PageHeader>

  <div class="memory-tabs">
    <button class:active={activeTab === 'memory'} onclick={() => { activeTab = 'memory'; void reload(); }}><Layers3 size={15}/>记忆</button>
    <button class:active={activeTab === 'tools'} onclick={() => { activeTab = 'tools'; void reload(); }}><Wrench size={15}/>工具</button>
    <button class:active={activeTab === 'organs'} onclick={() => { activeTab = 'organs'; void reload(); }}><BrainCircuit size={15}/>器官</button>
  </div>

  {#if error}
    <p class="error-banner" role="alert">{error}</p>
  {/if}

  {#if activeTab === 'memory'}
    <div class="memory-appender">
      <textarea bind:value={newMemory} rows="2" placeholder="追加一条记忆（写入后端 episodes）"></textarea>
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
        <p class="dim-hint">加载中…</p>
      {:else if !episodes.length}
        <div class="blank-state">
          <div class="blank-mark">⌁</div>
          <h3>暂无记忆条目</h3>
          <p>对话中的重要信息会被后端自动提取并持久化到 SQLite。</p>
        </div>
      {:else}
        {#each episodes as episode}
          <article class="episode-card">
            <div class="episode-head">
              <span class="badge blue">{episode.role}</span>
              <time>{formatTime(episode.timestamp)}</time>
              <small>{episode.id.slice(0, 10)}</small>
            </div>
            <p>{episode.content}</p>
          </article>
        {/each}
      {/if}
    </div>
  {:else if activeTab === 'tools'}
    <div class="tool-list">
      {#if loading && !tools.length}
        <p class="dim-hint">加载中…</p>
      {:else if !tools.length}
        <div class="blank-state">
          <div class="blank-mark">⌁</div>
          <h3>未发现工具</h3>
          <p>后端 tool_registry 动态加载的工具会列在此处。</p>
        </div>
      {:else}
        {#each tools as tool}
          <article class="tool-card">
            <div class="tool-head">
              <Wrench size={14} />
              <strong>{tool.name}</strong>
              {#if tool.tier}
                <span class="badge amber">Tier {tool.tier}</span>
              {/if}
            </div>
            <p>{tool.description}</p>
          </article>
        {/each}
      {/if}
    </div>
  {:else}
    <div class="organ-list">
      {#if loading && !organs.length}
        <p class="dim-hint">加载中…</p>
      {:else if !organs.length}
        <div class="blank-state">
          <div class="blank-mark">⌁</div>
          <h3>器官状态</h3>
          <p>Apeireth 9+1 器官运行指标。</p>
        </div>
      {/if}
    </div>
  {/if}
</section>
