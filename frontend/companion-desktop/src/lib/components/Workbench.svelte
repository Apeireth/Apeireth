<script lang="ts">
  import {ChevronRight, FileText} from 'lucide-svelte';
  import type {
    ApeirethConfig,
    CapabilityManifest,
    Conversation,
    ToolCallDetails,
    WorkbenchTurn,
  } from '../types';
  import MemoryView from '../MemoryView.svelte';
  import DiaryView from '../views/DiaryView.svelte';
  import {WORKBENCH_CARDS, type WorkbenchSection} from '../shell-nav';

  let {
    conversation = null,
    busy = false,
    closed = false,
    turn = null,
    config,
    capabilities = null,
    section = 'turn',
    onSelectSection,
    onClose,
  }: {
    conversation: Conversation | null;
    busy?: boolean;
    closed?: boolean;
    /** 后端 /v1/workbench/turn 真值：修复前持久化的 toolCall 卡在「运行中」，
     *  用后端状态按名称对位翻面，历史会话也能显示真实终态。 */
    turn?: WorkbenchTurn | null;
    /** 记忆卷宗分区取数用（侧栏收纳批：记忆/日记移入工作台）。 */
    config: ApeirethConfig;
    capabilities?: CapabilityManifest | null;
    /** 工作台分区：turn=回合视图（默认）/ memory=记忆卷宗 / diary=他的日记。 */
    section?: WorkbenchSection;
    onSelectSection?: (next: WorkbenchSection) => void;
    onClose: () => void;
  } = $props();

  let open = $state<Record<string, boolean>>({
    goal: true,
    agent: true,
    tools: true,
    files: true,
    mem: true,
  });

  const lastAssistant = $derived(
    conversation?.messages.filter((m) => m.role === 'assistant').at(-1) ?? null,
  );
  /** 后端状态词汇 → 本地 ToolCallDetails 状态；认不出的返回 null（保留本地值）。 */
  function mapBackendStatus(s: string): ToolCallDetails['status'] | null {
    const v = s.toLowerCase();
    if (v.includes('fail') || v.includes('error')) return 'failed';
    if (v.includes('cancel')) return 'cancelled';
    if (v.includes('complete') || v.includes('success') || v.includes('done')) return 'succeeded';
    if (v.includes('run')) return 'running';
    if (v.includes('pend') || v.includes('approv') || v.includes('wait')) return 'pending';
    return null;
  }
  const toolCalls = $derived.by(() => {
    const local = lastAssistant?.toolCalls ?? [];
    const backend = turn?.tools;
    if (!backend?.length || !local.length) return local;
    // 按名称排队对位（同名工具多次调用按先后次序配对），只翻面、不增删。
    const queues = new Map<string, (typeof backend)[number][]>();
    for (const t of backend) {
      const q = queues.get(t.name) ?? [];
      q.push(t);
      queues.set(t.name, q);
    }
    return local.map((tc) => {
      const bt = queues.get(tc.name)?.shift();
      const st = bt ? mapBackendStatus(bt.status) : null;
      if (!st || st === tc.status) return tc;
      return {...tc, status: st, ...(bt?.latency_ms ? {durationMs: bt.latency_ms} : {})};
    });
  });
  const provenance = $derived(lastAssistant?.provenance);
  const memories = $derived(provenance?.memories ?? []);
  const liveTools = $derived(
    toolCalls.filter((t) => t.status === 'pending' || t.status === 'running'),
  );
  const doneTools = $derived(toolCalls.filter((t) => t.status === 'succeeded'));
  /** 回合仍在流式进行：pending/running 是真「运行中」；回合已收尾还卡着 =
   *  修复前持久化的历史数据（当时 completed 事件被前端吞掉），而后端读模型
   *  不存工具名、无法逐条回溯成败——诚实标「已结束」，不谎称「完成」。 */
  const turnLive = $derived(lastAssistant?.streaming === true);
  function displayOf(t: ToolCallDetails): {label: string; run: boolean; bad: boolean} {
    if (t.status === 'succeeded') return {label: '完成', run: false, bad: false};
    if (t.status === 'failed') return {label: '失败', run: false, bad: true};
    if (t.status === 'cancelled') return {label: '已取消', run: false, bad: false};
    if (turnLive) return {label: '运行中', run: true, bad: false};
    return {label: '已结束', run: false, bad: false};
  }
  const blockCount = $derived(
    (conversation ? 1 : 0) +
      (toolCalls.length ? 1 : 0) +
      (doneTools.length || lastAssistant?.toolCalls?.length ? 1 : 0) +
      (memories.length ? 1 : 0),
  );

  function toggle(id: string): void {
    open = {...open, [id]: !open[id]};
  }
</script>

<aside class="wb" class:closed aria-label="工作台">
  <div class="wb-head">
    <h2>工作台</h2>
    <span class="count">{blockCount} 区块</span>
    <button class="mini" onclick={onClose} aria-label="收起">
      <ChevronRight size={13} class="shell-icon-sm" />
    </button>
  </div>
  <div class="wb-body">
    <!-- 侧栏收纳批：记忆 / 日记两张卡片入口（面板组件原样复用，作为工作台分区渲染；
         再点一次回到回合视图）。 -->
    <div class="wb-cards">
      {#each WORKBENCH_CARDS as card (card.section)}
        <button
          class="wb-card"
          class:active={section === card.section}
          aria-pressed={section === card.section}
          title={section === card.section ? '再点回到回合视图' : `打开${card.title}`}
          onclick={() => onSelectSection?.(section === card.section ? 'turn' : card.section)}
        >
          <span class="wb-card-title">{card.title}</span>
          <span class="wb-card-sub">{card.sub}</span>
        </button>
      {/each}
    </div>
    {#if section === 'memory'}
      <div class="wb-panel">
        <MemoryView {config} {capabilities} />
      </div>
    {:else if section === 'diary'}
      <div class="wb-panel">
        <DiaryView />
      </div>
    {:else if !conversation}
      <p class="wb-empty">开始对话后，目标、工具轨迹与本轮记忆会显示在这里。</p>
    {:else}
      <div class="blk">
        <button
          class="blk-head"
          aria-expanded={open.goal}
          onclick={() => toggle('goal')}
        >
          目标
          <ChevronRight size={13} class="shell-icon-sm caret" />
        </button>
        {#if open.goal}
          <div class="blk-body show">
            <p class="goal">{conversation.title || '新对话'}</p>
          </div>
        {/if}
      </div>

      <div class="blk">
        <button
          class="blk-head"
          aria-expanded={open.agent}
          onclick={() => toggle('agent')}
        >
          代理
          <ChevronRight size={13} class="shell-icon-sm caret" />
        </button>
        {#if open.agent}
          <div class="blk-body show">
            <button class="agent" type="button">
              <span class="st" class:run={busy}></span>
              主 Agent
              <span class="tag">{busy ? '运行中' : '空闲'}</span>
            </button>
          </div>
        {/if}
      </div>

      <div class="blk">
        <button
          class="blk-head"
          aria-expanded={open.tools}
          onclick={() => toggle('tools')}
        >
          工具轨迹
          {#if toolCalls.length}
            <span class="tagn">{turnLive && liveTools.length ? `${liveTools.length} 运行中` : `${toolCalls.length} 次调用`}</span>
          {/if}
          <ChevronRight size={13} class="shell-icon-sm caret" />
        </button>
        {#if open.tools}
          <div class="blk-body show">
            {#if toolCalls.length}
              {#each toolCalls as tool (tool.id)}
                {@const d = displayOf(tool)}
                <button class="agent" type="button">
                  <span class="st" class:run={d.run}></span>
                  {tool.name}
                  <span class="tag" class:bad={d.bad}>{d.label}</span>
                </button>
              {/each}
            {:else}
              <p class="wb-empty">本轮尚无工具调用。</p>
            {/if}
          </div>
        {/if}
      </div>

      {#if doneTools.length || lastAssistant?.toolCalls?.length}
        <div class="blk">
          <button
            class="blk-head"
            aria-expanded={open.files}
            onclick={() => toggle('files')}
          >
            引用的工具
            <span class="tagn">{toolCalls.length}</span>
            <ChevronRight size={13} class="shell-icon-sm caret" />
          </button>
          {#if open.files}
            <div class="blk-body show">
              {#each toolCalls as tool (tool.id)}
                <button class="file" type="button">
                  <FileText size={13} class="shell-icon-sm" />
                  {tool.name}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/if}

      <div class="blk">
        <button
          class="blk-head"
          aria-expanded={open.mem}
          onclick={() => toggle('mem')}
        >
          本轮记忆
          {#if memories.length}
            <span class="tagn">{memories.length}</span>
          {:else if provenance?.count}
            <span class="tagn">{provenance.count}</span>
          {/if}
          <ChevronRight size={13} class="shell-icon-sm caret" />
        </button>
        {#if open.mem}
          <div class="blk-body show">
            {#if memories.length}
              {#each memories as mem, i (i)}
                <div class="mem"><span class="sp">✦</span><span>{mem}</span></div>
              {/each}
            {:else if provenance?.count}
              <div class="mem">
                <span class="sp">✦</span>
                <span>他想起了 {provenance.count} 段记忆</span>
              </div>
            {:else}
              <p class="wb-empty">本轮尚未召回记忆。</p>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</aside>

<style>
  /* 工作台卡片入口（侧栏收纳批）：圆角深底 + 标题/副题，hover 微亮 */
  .wb-cards {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 12px;
  }
  .wb-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 3px;
    width: 100%;
    padding: 10px 12px;
    border: 1px solid var(--ap-line);
    border-radius: 10px;
    background: var(--ap-panel);
    color: var(--ap-bone);
    text-align: left;
    cursor: pointer;
    transition: border-color 0.2s ease, background 0.2s ease, box-shadow 0.2s ease;
  }
  .wb-card:hover {
    border-color: rgba(255, 210, 122, 0.45);
    background: var(--ap-shell-chip);
    box-shadow: 0 0 18px -8px rgba(255, 210, 122, 0.3);
  }
  .wb-card.active {
    border-color: rgba(255, 210, 122, 0.55);
    box-shadow: inset 2px 0 0 var(--ap-gold);
  }
  .wb-card-title {
    font-family: var(--ap-font-voice);
    font-size: 13px;
    letter-spacing: 0.12em;
  }
  .wb-card-sub {
    font-family: var(--ap-font-mono);
    font-size: 9.5px;
    letter-spacing: 0.06em;
    line-height: 1.7;
    color: var(--ap-bone-42);
  }
  /* 分区面板容器：原面板组件原样复用，只做容器级滚动 */
  .wb-panel {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
</style>
