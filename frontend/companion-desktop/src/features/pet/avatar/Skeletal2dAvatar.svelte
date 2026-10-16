<!-- 2D 骨骼形象层 — 第二华服（开放 2D 骨骼动画路线的程序化素体）
     #
     # 背景（选型报告 reports/avatar-runtime-selection-2026-10-11.md）：开放 2D
     # 骨骼格式的现成运行时生态薄、许可核验未收口（npm 实探：正名包不存在、
     # 杂名包许可不在白名单——2026-10-11 负证），故本层先以**程序化骨骼素体**
     # 落地"多一个可玩形态"：Canvas2D 骨骼小人（关节/骨段全代码生成，零资产
     # 零许可面），社区格式装载口作为插缝预留（等白名单运行时核验后接适配器）。
     #
     # 与其余华服同一契约：姿态/口型/眨眼/视线/呼吸从 PetFrame 显影，待机微
     # 动作与入定档从 idle-behavior 显影（衣服质感纪律：幅度封顶、不冒充信号）。
     # 验收纪律同门禁：隐藏必停渲染；无 WebGL 依赖（2D 纯 Canvas，天然回落档）。-->
<script lang="ts">
  import {onMount} from 'svelte';
  import type {PetFrame} from '../../../lib/pet/pet-contract';
  import {
    deriveCharacterParams,
    POSTURE_BODY_BIAS,
    type CharacterParams,
  } from '../../../lib/pet/character-mapping';
  import {deriveIdleMotion, type IdleMotion} from '../../../lib/pet/idle-behavior';
  import {petRenderPause} from '../../../lib/pet/pet-render-pause';

  let {frame}: {frame: PetFrame} = $props();

  let canvas: HTMLCanvasElement | null = $state(null);
  const seed = 11;

  onMount(() => {
    const el = canvas;
    const ctx = el?.getContext('2d') ?? null;
    if (!el || !ctx) return;

    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    el.width = 220 * dpr;
    el.height = 260 * dpr;
    ctx.scale(dpr, dpr);

    let disposed = false;
    let rafId = 0;
    let dreamHoldSec = 0;
    let lastT = performance.now();
    const startedAt = lastT;

    const loop = (now: number) => {
      if (disposed) return;
      rafId = requestAnimationFrame(loop);
      // 降级纪律①：隐藏即停（不画、不推时钟）——document.hidden 或宿主显式闩
      if (document.hidden || petRenderPause.paused) {
        lastT = now;
        return;
      }
      const dt = Math.min(0.05, Math.max(0, (now - lastT) / 1000));
      lastT = now;
      // 作息：只累计真实 dreaming 姿态时长（不发明"他睡了"）
      dreamHoldSec = frame.posture === 'curled_asleep' ? dreamHoldSec + dt : 0;
      const clockSec = (now - startedAt) / 1000;
      const params = deriveCharacterParams(frame, clockSec, seed);
      const idle = deriveIdleMotion(clockSec, seed, dreamHoldSec);
      draw(ctx, params, idle);
    };
    rafId = requestAnimationFrame(loop);

    return () => {
      disposed = true;
      cancelAnimationFrame(rafId);
    };
  });

  /** 骨架点位求解 + 描画（220×260 逻辑坐标；关节角全部来自契约/质感参数）。 */
  function draw(ctx: CanvasRenderingContext2D, p: CharacterParams, idle: IdleMotion): void {
    const bias = POSTURE_BODY_BIAS[p.posture];
    const doze = idle.doze;
    const sway = idle.swayRad * (1 - 0.5 * doze);
    const tilt = idle.headTiltRad * (1 - 0.5 * doze);
    const breath = Math.sin(p.breathPhase * Math.PI * 2) * p.breathAmplitude * idle.breathGain;
    const [tr, tg, tb] = p.tint.map((v) => Math.round(80 + v * 175));

    ctx.clearRect(0, 0, 220, 260);
    ctx.save();
    // 全身：姿态倾角 + 微摆 + 入定减淡
    ctx.globalAlpha = 1 - 0.45 * doze;
    ctx.translate(110, 178);
    ctx.rotate(((bias.leanDeg + sway * 57.3) * Math.PI) / 180);
    ctx.scale(bias.scale, bias.scale);

    const body = `rgb(${tr} ${tg} ${tb})`;
    const accent = `rgba(${tr} ${tg} ${tb} / 0.45)`;
    const line = (x1: number, y1: number, x2: number, y2: number, w: number) => {
      ctx.strokeStyle = body;
      ctx.lineWidth = w;
      ctx.lineCap = 'round';
      ctx.beginPath();
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.stroke();
    };

    // 腿（两段）
    line(-13, 0, -15, 44, 11);
    line(-15, 44, -16, 86, 9);
    line(13, 0, 15, 44, 11);
    line(15, 44, 16, 86, 9);

    // 手臂（curl 前收 + 偶发伸展上抬）
    const armSwing = bias.curl * 26 - idle.stretch * 34;
    const elbowY = -38 + armSwing * 0.25;
    line(-24, -44, -32 - bias.curl * 8, elbowY, 9);
    line(-32 - bias.curl * 8, elbowY, -30 - bias.curl * 16, elbowY + 34, 7);
    line(24, -44, 32 + bias.curl * 8, elbowY, 9);
    line(32 + bias.curl * 8, elbowY, 30 + bias.curl * 16, elbowY + 34, 7);

    // 躯干（呼吸起伏）
    const chestW = 26 + breath * 1.6;
    ctx.fillStyle = accent;
    ctx.beginPath();
    ctx.moveTo(-chestW, -52);
    ctx.lineTo(chestW, -52);
    ctx.lineTo(16, -2);
    ctx.lineTo(-16, -2);
    ctx.closePath();
    ctx.fill();
    ctx.strokeStyle = body;
    ctx.lineWidth = 3;
    ctx.stroke();

    // 头（视线 + 微倾 + 抬头）
    const headX = Math.sin(((p.gazeX * 12 + tilt * 57.3) * Math.PI) / 180) * 8;
    const headY = -78 - bias.headUpDeg * 0.5 - p.gazeY * 6;
    const headR = 20;
    ctx.fillStyle = body;
    ctx.beginPath();
    ctx.arc(headX, headY, headR, 0, Math.PI * 2);
    ctx.fill();

    // 眼神光（视线偏移）+ 眼睑（眨眼/入定收拢）
    const eyeOpen = Math.max(0.08, 1 - p.blink * 0.92 - doze * 0.75);
    const eyeDX = headX + p.gazeX * 5;
    const eyeDY = headY - 3 + p.gazeY * 4;
    ctx.fillStyle = `rgba(255 250 235 / ${0.6 + 0.4 * p.glow})`;
    for (const side of [-1, 1]) {
      ctx.beginPath();
      ctx.ellipse(eyeDX + side * 7, eyeDY, 2.6, 2.6 * eyeOpen, 0, 0, Math.PI * 2);
      ctx.fill();
    }

    // 口型（真实 speaking 才开）
    const jaw = 1 + p.visemeOpen * 5;
    ctx.fillStyle = `rgba(${tr} ${tg} ${tb} / 0.7)`;
    ctx.beginPath();
    ctx.ellipse(headX, headY + 11, 5, jaw * 0.45, 0, 0, Math.PI * 2);
    ctx.fill();

    ctx.restore();
  }
</script>

<canvas class="skeletal-canvas" bind:this={canvas} aria-label="2D 骨骼形象"></canvas>

<style>
  .skeletal-canvas {
    width: 100%;
    height: 100%;
    display: block;
  }
</style>
