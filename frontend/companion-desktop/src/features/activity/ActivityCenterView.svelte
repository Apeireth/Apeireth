<script lang="ts">
  import {onMount} from 'svelte';
  import {
    Activity,
    AlertCircle,
    CalendarClock,
    CheckCircle2,
    Clock,
    Flame,
    GitMerge,
    Play,
    RefreshCw,
    ShieldCheck,
    Target,
    Terminal,
    Wrench,
    XCircle,
  } from 'lucide-svelte';
  import PageHeader from '../../components/PageHeader.svelte';
  import type {ApeirethConfig} from '../../lib/types';
  import {fetchAuditRecords, type AuditRecord} from '../../lib/runtime';

  let {
    config,
  }: {
    config: ApeirethConfig;
  } = $props();

  type ActivityTab = 'goals' | 'runs' | 'workflows' | 'scheduled';

  let activeTab = $state<ActivityTab>('goals');
  let auditRecords = $state<AuditRecord[]>([]);
  let loading = $state(false);
  let error = $state('');

  // Sample native goal model based on Apeireth GoalService
  interface NativeGoal {
    id: string;
    objective: string;
    phase: 'active' | 'paused' | 'completed' | 'blocked';
    roundsStarted: number;
    maxRounds: number;
    milestones: Array<{title: string; completed: boolean}>;
    updatedAt: number;
    blockedReason?: string;
  }

  let goals = $state<NativeGoal[]>([
    {
      id: 'goal-main-1',
      objective: '构建高质量个人知识助手与桌面常驻环境',
      phase: 'active',
      roundsStarted: 3,
      maxRounds: 10,
      milestones: [
        {title: '装配 L0 Identity & L1 Essential 记忆', completed: true},
        {title: '连接 ToolBridge 与权限洋葱审批', completed: true},
        {title: '完成桌面外壳融合与伴随体呈现', completed: false},
      ],
      updatedAt: Date.now() - 3600000,
    },
  ]);

  let newObjective = $state('');
  let showCreateGoal = $state(false);

  async function reload(): Promise<void> {
    loading = true;
    error = '';
    try {
      if (activeTab === 'runs') {
        auditRecords = await fetchAuditRecords(config, 100);
      }
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
    } finally {
      loading = false;
    }
  }

  function addGoal(): void {
    const text = newObjective.trim();
    if (!text) return;
    goals = [
      {
        id: `goal-${crypto.randomUUID().slice(0, 8)}`,
        objective: text,
        phase: 'active',
        roundsStarted: 0,
        maxRounds: 10,
        milestones: [{title: '初始化阶段', completed: false}],
        updatedAt: Date.now(),
      },
      ...goals,
    ];
    newObjective = '';
    showCreateGoal = false;
  }

  function formatTime(ts?: number): string {
    if (!ts) return '-';
    const d = new Date(ts > 10000000000 ? ts : ts * 1000);
    return d.toLocaleString('zh-CN', {month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit'});
  }

  onMount(() => {
    void reload();
  });
</script>

<section class="view">
  <PageHeader
    eyebrow="活动"
    title="活动中心"
    subtitle="Apeireth 目标驱动 (Goals)、执行留痕 (Runs)、工作流编排 (Workflows) 与定时调度 (Scheduled)。"
  >
    {#if activeTab === 'goals'}
      <button class="primary-button" onclick={() => showCreateGoal = !showCreateGoal}>
        <Target size={14} />新建目标
      </button>
    {:else}
      <button class="quiet-button" onclick={reload} disabled={loading}>
        <RefreshCw size={14} />刷新
      </button>
    {/if}
  </PageHeader>

  <div class="activity-tabs">
    <button
      class:active={activeTab === 'goals'}
      onclick={() => { activeTab = 'goals'; void reload(); }}
    >
      <Target size={15} />目标 (Goals)
    </button>
    <button
      class:active={activeTab === 'runs'}
      onclick={() => { activeTab = 'runs'; void reload(); }}
    >
      <Activity size={15} />执行留痕 (Runs)
    </button>
    <button
      class:active={activeTab === 'workflows'}
      onclick={() => { activeTab = 'workflows'; void reload(); }}
    >
      <GitMerge size={15} />工作流 (Workflows)
    </button>
    <button
      class:active={activeTab === 'scheduled'}
      onclick={() => { activeTab = 'scheduled'; void reload(); }}
    >
      <CalendarClock size={15} />定时调度 (Scheduled)
    </button>
  </div>

  {#if error}
    <p class="error-banner" role="alert">{error}</p>
  {/if}

  {#if activeTab === 'goals'}
    <div class="activity-content">
      {#if showCreateGoal}
        <div class="goal-create-box">
          <input
            bind:value={newObjective}
            placeholder="输入目标愿景与预期交付 (例如: 调研开源项目架构并输出总结报告)…"
          />
          <div class="goal-create-actions">
            <button class="primary-button" onclick={addGoal} disabled={!newObjective.trim()}>创建目标</button>
            <button class="quiet-button" onclick={() => showCreateGoal = false}>取消</button>
          </div>
        </div>
      {/if}

      <div class="goal-list">
        {#each goals as goal}
          <article class="goal-card">
            <div class="goal-card-header">
              <div class="goal-phase-badge">
                <span
                  class="badge"
                  class:green={goal.phase === 'active'}
                  class:amber={goal.phase === 'paused'}
                  class:blue={goal.phase === 'completed'}
                >
                  {goal.phase === 'active' ? '推进中' : goal.phase === 'paused' ? '已暂停' : '已完成'}
                </span>
                <strong>{goal.objective}</strong>
              </div>
              <small>{formatTime(goal.updatedAt)}</small>
            </div>

            <div class="goal-progress-bar">
              <div
                class="goal-progress-fill"
                style="width: {(goal.roundsStarted / goal.maxRounds) * 100}%"
              ></div>
            </div>
            <div class="goal-progress-stats">
              <span>轮次进度: {goal.roundsStarted} / {goal.maxRounds} 轮</span>
              <span>状态机 Revision: 1</span>
            </div>

            {#if goal.milestones?.length}
              <ul class="goal-milestones">
                {#each goal.milestones as milestone}
                  <li class:done={milestone.completed}>
                    {#if milestone.completed}
                      <CheckCircle2 size={13} class="ok" />
                    {:else}
                      <Clock size={13} class="dim" />
                    {/if}
                    <span>{milestone.title}</span>
                  </li>
                {/each}
              </ul>
            {/if}
          </article>
        {/each}
      </div>
    </div>
  {:else if activeTab === 'runs'}
    <div class="activity-content">
      {#if loading && !auditRecords.length}
        <p class="dim-hint">正在拉取审计执行留痕 (ActionStream)…</p>
      {:else if !auditRecords.length}
        <div class="blank-state">
          <div class="blank-mark">⌁</div>
          <h3>暂无工具执行留痕</h3>
          <p>在对话中调用工具或运行任务后，所有操作留痕将记录在 ActionStream 审计流中。</p>
        </div>
      {:else}
        <div class="audit-list">
          {#each auditRecords as rec}
            <article class="audit-card">
              <div class="audit-card-head">
                <div class="audit-tool-box">
                  <Wrench size={13} />
                  <strong>{rec.tool_name || '系统操作'}</strong>
                  {#if rec.masked}
                    <span class="badge dim">隐私脱敏</span>
                  {/if}
                </div>
                <time>{formatTime(rec.created_at || rec.timestamp)}</time>
              </div>

              {#if rec.call_content}
                <div class="audit-call-box">
                  <Terminal size={11} />
                  <code>{rec.call_content}</code>
                </div>
              {/if}

              {#if rec.execution_result}
                <pre class="audit-result">{rec.execution_result}</pre>
              {/if}
            </article>
          {/each}
        </div>
      {/if}
    </div>
  {:else if activeTab === 'workflows'}
    <div class="activity-content">
      <div class="workflow-presets-grid">
        <article class="workflow-card">
          <div class="workflow-card-head">
            <GitMerge size={16} class="exec-icon-accent" />
            <strong>自动认知反思工作流 (Cognitive Reflection DAG)</strong>
            <span class="badge green">常驻调度</span>
          </div>
          <p>对话积累 → 周期安静期检测 → MiniMax 深度反思 → 提炼沉淀至 ExperienceStore。</p>
          <div class="workflow-steps-flow">
            <span class="step-pill">1. 记忆捕获</span>
            <i>→</i>
            <span class="step-pill">2. 模式提取</span>
            <i>→</i>
            <span class="step-pill">3. 经验入库</span>
          </div>
        </article>

        <article class="workflow-card">
          <div class="workflow-card-head">
            <ShieldCheck size={16} class="ok" />
            <strong>高危工具权限洋葱工作流 (Permission Onion Flow)</strong>
            <span class="badge amber">审批拦截</span>
          </div>
          <p>工具请求 → Tier 分级判定 → RequireApproval 拦截入库 → 主人一键授权放行。</p>
          <div class="workflow-steps-flow">
            <span class="step-pill">1. Tool Call</span>
            <i>→</i>
            <span class="step-pill">2. 洋葱拦截</span>
            <i>→</i>
            <span class="step-pill">3. 主人批准</span>
          </div>
        </article>
      </div>
    </div>
  {:else}
    <div class="activity-content">
      <div class="scheduled-grid">
        <article class="scheduled-card">
          <div class="scheduled-card-head">
            <Clock size={16} class="exec-icon-accent" />
            <strong>做梦调度器 (DreamScheduler)</strong>
            <span class="badge blue">6 小时安静期</span>
          </div>
          <p>夜间或用户离开超过 6 小时后，自动合并关联记忆并提炼高价值记忆摘要。</p>
        </article>

        <article class="scheduled-card">
          <div class="scheduled-card-head">
            <CalendarClock size={16} class="exec-icon-accent" />
            <strong>反思调度器 (ReflectionScheduler)</strong>
            <span class="badge green">24 小时周期</span>
          </div>
          <p>每 24 小时对近期记忆进行深度模式挖掘与习惯偏好洞察，生成自成长日志。</p>
        </article>

        <article class="scheduled-card">
          <div class="scheduled-card-head">
            <Flame size={16} class="exec-icon-accent" />
            <strong>作息节律感知 (Rhythm Tracker)</strong>
            <span class="badge amber">实时学习</span>
          </div>
          <p>根据用户消息交互时间统计活跃直方图，智能计算活跃概率与最佳问候时机。</p>
        </article>
      </div>
    </div>
  {/if}
</section>
