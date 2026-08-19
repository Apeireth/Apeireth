<script lang="ts">
  import {onMount} from 'svelte';
  import {Layers3, Wrench, Cpu, RefreshCw, BrainCircuit} from 'lucide-svelte';
  import PageHeader from './PageHeader.svelte';
  import type {ApeirethConfig, MemoryCategory} from './types';
  import {categoryFromWire, categoryToWire} from './types';
  import {fetchEpisodes, fetchOrgans, fetchTools, type MemoryEpisode, type ToolInfo} from './runtime';

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
        <button class="primary-button" onclick={appendMemory} disabled={!newMemory.trim()}>写入记忆</button>
      </div>
      {#if appended}
        <p class="ok-hint">已写入后端。</p>
      {/if}
    </div>

    <div class="memory-list">
      {#if !episodes.length && !loading}
        <div class="blank-state"><p>还没有记忆。先对话或写入一条。</p></div>
      {:else}
        {#each episodes as ep (ep.id)}
          <article class="memory-item">
            <div class="memory-item-meta">
              <span class="badge" class:blue={ep.role === 'user'} class:amber={ep.role === 'assistant'}>{ep.role}</span>
              <span>{formatTime(ep.timestamp)}</span>
              <small>{ep.session_id}</small>
            </div>
            <p>{ep.content}</p>
          </article>
        {/each}
      {/if}
    </div>
  {:else if activeTab === 'tools'}
    <div class="tool-list">
      {#if !tools.length && !loading}
        <div class="blank-state"><p>工具注册表为空。后端未注册工具，或未连接。</p></div>
      {:else}
        {#each tools as tool}
          <article class="tool-item">
            <div class="tool-item-head">
              <Cpu size={15} />
              <strong>{tool.name}</strong>
            </div>
            {#if tool.description}
              <p>{tool.description}</p>
            {/if}
          </article>
        {/each}
      {/if}
    </div>
  {:else}
    <div class="organ-list">
      {#if !organs.length && !loading}
        <div class="blank-state"><p>器官状态为空。后端未连接，或未暴露器官。</p></div>
      {:else}
        {#each organs as organ, index}
          <article class="organ-item">
            <strong>器官 {index + 1}</strong>
            <pre>{JSON.stringify(organ, null, 2)}</pre>
          </article>
        {/each}
      {/if}
    </div>
  {/if}
</section>
