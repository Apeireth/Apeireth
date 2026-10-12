<!-- 形象层宿主 — 双形态可插拔的渲染插口
     #
     # 按 layerId 从 avatar-registry 解析形象层并渲染；装载失败或清单外 id
     # 一律回落「微光简模」，桌宠绝不空窗。主题包形象层在此动态装载（plug
     # point）：清单校验在纯逻辑层（avatar-registry.parseThemePackManifest），
     # 这里只负责 import 模块并按 PetFrame 契约喂帧。-->
<script lang="ts">
  import type {Component} from 'svelte';
  import {
    resolveAvatarLayerOrFallback,
    registerBuiltInAvatarLayers,
  } from '../../lib/pet/avatar-registry';
  import type {AvatarLayerModule} from '../../lib/pet/avatar-registry';
  import {FALLBACK_AVATAR_LAYER_ID, type PetFrame} from '../../lib/pet/pet-contract';
  import LightSentinelAvatar from './avatar/LightSentinelAvatar.svelte';
  import EmberMinimalAvatar from './avatar/EmberMinimalAvatar.svelte';

  let {frame, layerId}: {frame: PetFrame; layerId: string} = $props();

  // 内置形象层注册（模块装载一次）：光态哨兵（默认）+ 微光简模（回落）。
  registerBuiltInAvatarLayers({
    lightSentinel: async (): Promise<AvatarLayerModule> => ({default: LightSentinelAvatar}),
    emberMinimal: async (): Promise<AvatarLayerModule> => ({default: EmberMinimalAvatar}),
  });

  let Layer = $state<Component<{frame: PetFrame}> | null>(null);
  let loadedId = $state('');

  $effect(() => {
    const wanted = layerId;
    let cancelled = false;
    const entry = resolveAvatarLayerOrFallback(wanted);
    entry
      .load()
      .then((module) => {
        if (cancelled) return;
        Layer = (module.default ?? null) as Component<{frame: PetFrame}> | null;
        loadedId = entry.descriptor.id;
      })
      .catch(() => {
        // 装载失败 → 微光简模（诚实回落，不空窗）
        if (cancelled) return;
        const fallback = resolveAvatarLayerOrFallback(FALLBACK_AVATAR_LAYER_ID);
        fallback.load().then((m) => {
          if (cancelled) return;
          Layer = (m.default ?? null) as Component<{frame: PetFrame}> | null;
          loadedId = fallback.descriptor.id;
        });
      });
    return () => {
      cancelled = true;
    };
  });
</script>

{#if Layer}
  <div class="avatar-host" data-layer={loadedId}>
    <Layer {frame} />
  </div>
{/if}

<style>
  .avatar-host {
    width: 100%;
    height: 100%;
  }
</style>
