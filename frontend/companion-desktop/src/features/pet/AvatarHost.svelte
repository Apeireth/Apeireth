<!-- 形象层宿主 — 双形态可插拔的渲染插口
     #
     # 按 layerId 从 avatar-registry 解析形象层并渲染；装载失败、清单外 id、
     # 或渲染层上报 WebGL 不可用（onUnavailable 降级口）一律回落「微光简模」，
     # 桌宠绝不空窗。主题包/角色形象层在此动态装载（plug point）：清单校验在
     # 纯逻辑层（avatar-registry），这里只负责 import 模块并按 PetFrame 喂帧。-->
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
  import CharacterAvatar from './avatar/CharacterAvatar.svelte';
  import Skeletal2dAvatar from './avatar/Skeletal2dAvatar.svelte';

  let {
    frame,
    layerId,
    modelUrl = null,
    onModelMeta = undefined,
  }: {
    frame: PetFrame;
    layerId: string;
    /** 形象文件（开放 3D 人形格式）；角色形象层专用，其余层忽略 */
    modelUrl?: string | null;
    /** 形象文件内嵌许可元数据透传（角色形象层专用） */
    onModelMeta?: (meta: import('../../lib/pet/user-avatar').UserAvatarMeta) => void;
  } = $props();

  // 内置形象层注册（模块装载一次）：光态哨兵（默认）+ 微光简模（回落）
  // + 角色形象（华服，tier0Safe=false 不进 T0）。
  registerBuiltInAvatarLayers({
    lightSentinel: async (): Promise<AvatarLayerModule> => ({default: LightSentinelAvatar}),
    emberMinimal: async (): Promise<AvatarLayerModule> => ({default: EmberMinimalAvatar}),
    characterForm: async (): Promise<AvatarLayerModule> => ({default: CharacterAvatar}),
    skeletal2d: async (): Promise<AvatarLayerModule> => ({default: Skeletal2dAvatar}),
  });

  type LayerProps = {
    frame: PetFrame;
    modelUrl?: string | null;
    onUnavailable?: () => void;
    onModelMeta?: (meta: import('../../lib/pet/user-avatar').UserAvatarMeta) => void;
  };
  let Layer = $state<Component<LayerProps> | null>(null);
  let loadedId = $state('');
  /** 渲染层上报降级（WebGL 不可用）后强制回落微光简模 */
  let forcedFallback = $state(false);

  $effect(() => {
    const wanted = forcedFallback ? FALLBACK_AVATAR_LAYER_ID : layerId;
    let cancelled = false;
    const entry = resolveAvatarLayerOrFallback(wanted);
    entry
      .load()
      .then((module) => {
        if (cancelled) return;
        Layer = (module.default ?? null) as Component<LayerProps> | null;
        loadedId = entry.descriptor.id;
      })
      .catch(() => {
        // 装载失败 → 微光简模（诚实回落，不空窗）
        if (cancelled) return;
        const fallback = resolveAvatarLayerOrFallback(FALLBACK_AVATAR_LAYER_ID);
        fallback.load().then((m) => {
          if (cancelled) return;
          Layer = (m.default ?? null) as Component<LayerProps> | null;
          loadedId = fallback.descriptor.id;
        });
      });
    return () => {
      cancelled = true;
    };
  });

  function handleUnavailable(): void {
    forcedFallback = true;
  }
</script>

{#if Layer}
  <div class="avatar-host" data-layer={loadedId}>
    <Layer {frame} {modelUrl} onUnavailable={handleUnavailable} {onModelMeta} />
  </div>
{/if}

<style>
  .avatar-host {
    width: 100%;
    height: 100%;
  }
</style>
