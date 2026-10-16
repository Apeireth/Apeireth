<!-- 微光简模 — 廉价形象层（T0 梯队 / 渲染失败回落）
     #
     # 同一 PetFrame 契约的第二个实现：单点余烬 + 一圈光晕，纯 CSS 无 SVG 滤镜，
     # 老机器零负担。它同时是「形象抽象接口确实可插」的活证据——双形态可插拔
     # 不是纸面契约，两个内置层共吃一份帧数据。姿态在这里只有明暗与倾角。-->
<script lang="ts">
  import type {PetFrame} from '../../../lib/pet/pet-contract';
  import {kelvinToRgb} from '../../../lib/pet/pet-contract';

  let {frame}: {frame: PetFrame} = $props();

  const tint = $derived(kelvinToRgb(frame.light.colorTempK));
  const tintCss = $derived(`rgb(${Math.round(tint[0] * 255)} ${Math.round(tint[1] * 255)} ${Math.round(tint[2] * 255)})`);
  const breathPeriod = $derived(`${Math.max(1, frame.breath.periodSecs).toFixed(2)}s`);
  const lean = $derived(frame.posture === 'leaning_close' ? -4 : frame.posture === 'corner_quiet' ? 7 : 0);
  const dim = $derived(frame.posture === 'curled_asleep' ? 0.55 : frame.posture === 'corner_quiet' ? 0.7 : 1);
</script>

<div
  class="ember posture-{frame.posture}"
  class:glance={frame.glance}
  style:--pet-tint={tintCss}
  style:--pet-breath-period={breathPeriod}
  style:--pet-glow={frame.glow.toFixed(3)}
  style:--pet-base={(0.3 + 0.7 * frame.light.baseIntensity).toFixed(3)}
  style:--pet-lean={`${lean}deg`}
  style:--pet-dim={dim.toFixed(2)}
>
  <div class="halo"></div>
  <div class="dot"></div>
</div>

<style>
  .ember {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    user-select: none;
  }

  .halo {
    position: absolute;
    width: 64%;
    aspect-ratio: 1;
    border-radius: 50%;
    background: radial-gradient(circle, var(--pet-tint) 0%, transparent 70%);
    opacity: calc(0.18 * var(--pet-glow) + 0.04);
    animation: ember-breath var(--pet-breath-period) ease-in-out infinite alternate;
    transform: rotate(var(--pet-lean));
  }

  .dot {
    position: absolute;
    width: 14%;
    aspect-ratio: 1;
    border-radius: 50%;
    background: var(--pet-tint);
    opacity: calc(var(--pet-base) * var(--pet-dim));
    box-shadow: 0 0 calc(6px + 16px * var(--pet-glow)) var(--pet-tint);
    animation: ember-breath var(--pet-breath-period) ease-in-out infinite alternate;
  }

  .glance .dot {
    transform: translateY(-5px);
    transition: transform 1s cubic-bezier(0.33, 1, 0.68, 1);
  }

  @keyframes ember-breath {
    from {
      transform: scale(0.96);
      opacity: 0.82;
    }
    to {
      transform: scale(1.05);
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .halo,
    .dot {
      animation: none;
    }
  }
</style>
