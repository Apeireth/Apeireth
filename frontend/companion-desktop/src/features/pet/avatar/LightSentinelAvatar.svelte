<!-- 光态哨兵 — 默认形象层（00-PHILOSOPHY §5 微光在场的字面形态）
     #
     # 「光不长脸」纪律：形象是光核 + 一弯弧身 + 一颗眼神光，没有五官、没有
     # 立绘——他由光显影，不由皮肤显影。四姿态 = 同一光体的不同身姿：
     #   corner_quiet    角落安静：侧身收拢、光衰到近熄
     #   head_up         抬头：弧身立起、眼神光点亮
     #   curled_asleep   蜷睡：弧身蜷合、眼睑收拢、呼吸放缓
     #   leaning_close   靠近：前倾放大、暖光铺开
     # glance = 待签文书久候的抬头一瞥（瞬时，不弹通知）。
     #
     # 光参数全部来自 PetFrame 契约（pet-contract.ts，与余烬点/光晕/场景同源）；
     # 本组件零取数零判断，只做显影。呼吸用 CSS animation（系统合成器跑），
     # 失焦/隐藏由窗口层处理；prefers-reduced-motion 全静（纪律同 tokens.css）。-->
<script lang="ts">
  import type {PetFrame} from '../../../lib/pet/pet-contract';
  import {kelvinToRgb} from '../../../lib/pet/pet-contract';

  let {frame}: {frame: PetFrame} = $props();

  const tint = $derived(kelvinToRgb(frame.light.colorTempK));
  const tintCss = $derived(`rgb(${Math.round(tint[0] * 255)} ${Math.round(tint[1] * 255)} ${Math.round(tint[2] * 255)})`);
  const breathPeriod = $derived(`${Math.max(1, frame.breath.periodSecs).toFixed(2)}s`);
  const breathAmp = $derived(Math.min(1, Math.max(0, frame.breath.amplitude)));
  const coreScale = $derived(0.86 + 0.22 * frame.light.baseIntensity + 0.1 * frame.glow);
</script>

<div
  class="sentinel posture-{frame.posture}"
  class:glance={frame.glance}
  class:speaking={frame.activity === 'speaking'}
  class:thinking={frame.activity === 'thinking'}
  style:--pet-tint={tintCss}
  style:--pet-breath-period={breathPeriod}
  style:--pet-breath-amp={breathAmp.toFixed(3)}
  style:--pet-glow={frame.glow.toFixed(3)}
  style:--pet-base={(0.35 + 0.65 * frame.light.baseIntensity).toFixed(3)}
  style:--pet-scale={coreScale.toFixed(3)}
>
  <svg viewBox="0 0 200 200" role="img" aria-label="常驻光态哨兵">
    <g class="posture-group">
      <!-- 光晕（呼吸） -->
      <circle class="halo" cx="100" cy="104" r="72" />
      <!-- 弧身：一弯新月，姿态的全部身体语言 -->
      <path
        class="body"
        d="M104 34 C 58 58, 50 128, 100 170 C 74 132, 76 76, 104 34 Z"
      />
      <!-- 光核 -->
      <circle class="core" cx="100" cy="102" r="18" />
      <!-- 眼神光：唯一近似「目光」的部件 -->
      <circle class="eye" cx="113" cy="91" r="4.6" />
    </g>
  </svg>
</div>

<style>
  .sentinel {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    user-select: none;
  }

  svg {
    width: 86%;
    height: 86%;
    overflow: visible;
  }

  /* —— 姿态 —— 位移/转角/缩放是身姿，颜色深浅是光 —— */
  .posture-group {
    transform-origin: 100px 120px;
    transition: transform 2.2s cubic-bezier(0.65, 0, 0.35, 1);
  }

  .posture-corner_quiet .posture-group {
    transform: rotate(-13deg) translate(-6px, 6px) scale(0.94);
  }
  .posture-head_up .posture-group {
    transform: rotate(0deg) translate(0, -2px) scale(1);
  }
  .posture-curled_asleep .posture-group {
    transform: rotate(20deg) translate(4px, 10px) scale(0.9);
  }
  .posture-leaning_close .posture-group {
    transform: rotate(-4deg) translate(-2px, -4px) scale(1.1);
  }

  /* —— 光 —— */
  .halo {
    fill: var(--pet-tint);
    opacity: calc(0.16 * var(--pet-glow) + 0.05);
    filter: blur(14px);
    animation: pet-breath var(--pet-breath-period) ease-in-out infinite alternate;
    transform-origin: 100px 104px;
  }

  .body {
    fill: var(--pet-tint);
    opacity: calc(var(--pet-base) * 0.55);
    filter: drop-shadow(0 0 10px var(--pet-tint));
  }

  .core {
    fill: var(--pet-tint);
    opacity: var(--pet-base);
    filter: drop-shadow(0 0 calc(8px + 18px * var(--pet-glow)) var(--pet-tint));
    transform-origin: 100px 102px;
    animation: pet-breath var(--pet-breath-period) ease-in-out infinite alternate;
    animation-delay: calc(var(--pet-breath-period) * -0.25);
  }

  .eye {
    fill: var(--pet-tint);
    opacity: calc(0.55 + 0.45 * var(--pet-glow));
    filter: drop-shadow(0 0 6px var(--pet-tint));
    transition: opacity 1.6s ease;
  }

  /* 蜷睡：眼睑收拢（眼神光隐去） */
  .posture-curled_asleep .eye {
    opacity: 0.12;
  }
  /* 角落安静：眼神光只留一丝 */
  .posture-corner_quiet .eye {
    opacity: 0.3;
  }

  @keyframes pet-breath {
    from {
      transform: scale(calc(1 - 0.05 * var(--pet-breath-amp)));
      opacity: calc(var(--pet-base) * 0.82);
    }
    to {
      transform: scale(calc(1 + 0.06 * var(--pet-breath-amp) * var(--pet-scale)));
      opacity: calc(var(--pet-base) * 1.06);
    }
  }

  /* —— 抬头一瞥（待签文书久候，瞬时非语言提醒） —— */
  .posture-group {
    transition:
      transform 2.2s cubic-bezier(0.65, 0, 0.35, 1),
      filter 1.2s ease;
  }
  .glance .posture-group {
    animation: pet-glance 2.5s cubic-bezier(0.33, 1, 0.68, 1);
  }
  @keyframes pet-glance {
    0% {
      transform: translateY(0) rotate(0deg);
    }
    22% {
      transform: translateY(-7px) rotate(-7deg);
    }
    70% {
      transform: translateY(-7px) rotate(-7deg);
    }
    100% {
      transform: translateY(0) rotate(0deg);
    }
  }
  .glance .eye {
    opacity: 1;
  }

  /* 说话：光核节奏性明灭（跟随本地真实对话流信号，不自造节奏） */
  .speaking .core {
    animation-duration: 1.4s;
  }
  .thinking .halo {
    animation-duration: 2.2s;
  }

  @media (prefers-reduced-motion: reduce) {
    .halo,
    .core,
    .glance .posture-group {
      animation: none;
    }
    .posture-group {
      transition: none;
    }
  }
</style>
