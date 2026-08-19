<script lang="ts">
  import {Plug, Plus, type Icon} from 'lucide-svelte';
  import StatusDot from '../components/StatusDot.svelte';
  import type {HealthState, ViewId} from '../lib/types';

  interface NavItem {
    id: ViewId;
    label: string;
    icon: typeof Icon;
  }

  let {
    nav,
    activeView = $bindable('chat'),
    healthState,
    healthLabel,
    onNewConversation,
  }: {
    nav: readonly NavItem[];
    activeView: ViewId;
    healthState: HealthState;
    healthLabel: Record<HealthState, string>;
    onNewConversation: () => void;
  } = $props();
</script>

<aside class="sidebar">
  <div class="sidebar-brand">
    <span class="logo-mark">A</span>
    <span class="brand-name">Apeireth 伙伴</span>
    <StatusDot
      size="small"
      off={healthState !== 'ready' && healthState !== 'generating'}
      active={healthState === 'generating'}
    />
  </div>
  <nav class="nav">
    {#each nav as item}
      <button class:active={activeView === item.id} onclick={() => activeView = item.id}>
        <item.icon size={17} />
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>
  <div class="sidebar-footer">
    <button class="quiet-button wide" onclick={onNewConversation}><Plus size={14}/>新对话</button>
    <div class="conn-hint" class:offline={healthState === 'offline'}>
      <Plug size={12} />
      <span>{healthLabel[healthState]}</span>
    </div>
  </div>
</aside>
