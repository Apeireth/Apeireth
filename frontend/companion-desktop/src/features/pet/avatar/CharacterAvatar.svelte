<!-- 角色形象层 — 开放 3D 人形格式渲染（批 B，2026-10-11 四次拍板）
     #
     # 他是"华服"：光仍是本体（tint/glow 从契约直通），本层把姿态/呼吸/口型/
     # 视线/眨眼显影成身体语言。两种形象来源：
     #   - modelUrl 有值：用户自带/官方形象文件（开放 3D 人形格式，本机装载）；
     #   - modelUrl 空：程序化"素体"（胶囊体人形，零资产零许可面）。
     # 形象文件加载失败 → 回落素体（绝不空窗）；WebGL 不可用 → onUnavailable
     # 通知宿主回落微光简模（探针实测过的降级纪律）。
     #
     # 验收门禁（reports/avatar-perf-probe-2026-10-11.md 固化）：
     #   ① document.hidden 必停渲染（帧计数口径）；② WebGL 不可用必优雅降级。
     # 性能：动态装载（three 系不进主包）；dpr 封顶 2；失焦即停。-->
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
  import {
    normalizeVrmMetaLicense,
    type UserAvatarMeta,
  } from '../../../lib/pet/user-avatar';

  let {
    frame,
    modelUrl = null,
    onUnavailable = undefined,
    onModelMeta = undefined,
  }: {
    frame: PetFrame;
    /** 形象文件地址（开放 3D 人形格式）；null = 程序化素体 */
    modelUrl?: string | null;
    /** WebGL 不可用时通知宿主回落（降级纪律的接线口） */
    onUnavailable?: () => void;
    /** 形象文件内嵌许可元数据透传（只读出不解读，宿主如实展示） */
    onModelMeta?: (meta: UserAvatarMeta) => void;
  } = $props();

  let host: HTMLDivElement | null = $state(null);
  const seed = 7;

  /** 形象文件运行时的最小契约面（开放 3D 人形格式加载库的鸭子类型）。 */
  interface VrmLike {
    scene: import('three').Object3D;
    update?: (dt: number) => void;
    humanoid?: {
      getNormalizedBoneNode: (name: string) => import('three').Object3D | null;
    };
    expressionManager?: {setValue: (name: string, value: number) => void} | null;
    lookAt?: {target?: import('three').Object3D} | null;
  }

  onMount(() => {
    let disposed = false;
    let rafId = 0;
    let cleanupScene: (() => void) | null = null;
    const startedAt = performance.now();

    void (async () => {
      let THREE: typeof import('three');
      try {
        THREE = await import('three');
      } catch {
        onUnavailable?.();
        return;
      }
      if (disposed || !host) return;

      // WebGL 探测（降级纪律②）：不可用即上报，由宿主回落
      const probe = document.createElement('canvas');
      const gl = probe.getContext('webgl2') ?? probe.getContext('webgl');
      if (!gl) {
        onUnavailable?.();
        return;
      }

      let renderer: import('three').WebGLRenderer;
      try {
        renderer = new THREE.WebGLRenderer({alpha: true, antialias: true});
      } catch {
        onUnavailable?.();
        return;
      }
      renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
      renderer.setSize(host.clientWidth || 220, host.clientHeight || 260, false);
      host.appendChild(renderer.domElement);

      const scene = new THREE.Scene();
      const camera = new THREE.PerspectiveCamera(28, 1, 0.1, 100);
      camera.position.set(0, 1.15, 3.4);
      camera.lookAt(0, 0.95, 0);

      const ambient = new THREE.AmbientLight(0xffffff, 0.55);
      const key = new THREE.DirectionalLight(0xffffff, 1.15);
      key.position.set(1.6, 2.4, 2.2);
      const rim = new THREE.DirectionalLight(0xffffff, 0.4);
      rim.position.set(-1.8, 1.2, -1.6);
      scene.add(ambient, key, rim);

      // —— 形象来源：用户/官方形象文件 → 开放 3D 人形格式装载；失败回落素体 ——
      let vrm: VrmLike | null = null;
      const rig = buildStandaloneRig(THREE);
      scene.add(rig.root);

      if (modelUrl) {
        try {
          const {GLTFLoader} = await import('three/examples/jsm/loaders/GLTFLoader.js');
          const {VRMLoaderPlugin} = await import('@pixiv/three-vrm');
          const loader = new GLTFLoader();
          loader.register((parser: unknown) => new VRMLoaderPlugin(parser as never));
          const gltf = await loader.loadAsync(modelUrl);
          const loaded = (gltf.userData as {vrm?: VrmLike | null}).vrm ?? null;
          if (loaded && !disposed) {
            vrm = loaded;
            scene.add(vrm.scene);
            // 形象文件到位：素体退场（换装不换契约）；内嵌许可元数据透传宿主
            scene.remove(rig.root);
            onModelMeta?.(normalizeVrmMetaLicense((loaded as {meta?: unknown}).meta));
          }
        } catch {
          // 形象文件加载失败 → 素体顶上（绝不空窗）；不冒充加载成功
        }
      }

      const reduced =
        typeof matchMedia === 'function' &&
        matchMedia('(prefers-reduced-motion: reduce)').matches;

      let lastT = startedAt;
      let dreamHoldSec = 0;
      const loop = (now: number) => {
        if (disposed) return;
        rafId = requestAnimationFrame(loop);
        // 降级纪律①：隐藏即停渲染（不渲染、不更新、不 tick）——document.hidden
        // 或宿主显式闩（真隐藏口径：window.hide() 不翻 visibilityState，2026-10-11 实测）
        if (document.hidden || petRenderPause.paused) {
          lastT = now;
          return;
        }
        const dt = Math.min(0.05, Math.max(0, (now - lastT) / 1000));
        lastT = now;
        const clockSec = (now - startedAt) / 1000;
        // 作息：只累计真实 dreaming 姿态时长（不发明"他睡了"）
        dreamHoldSec = frame.posture === 'curled_asleep' ? dreamHoldSec + dt : 0;
        const params = deriveCharacterParams(frame, clockSec, seed);
        applyParams(params, deriveIdleMotion(clockSec, seed, dreamHoldSec), reduced);
        if (vrm?.update) vrm.update(dt);
        renderer.render(scene, camera);
      };

      const applyParams = (p: CharacterParams, idle: IdleMotion, still: boolean) => {
        const bias = POSTURE_BODY_BIAS[p.posture];
        const breath = still ? 0 : Math.sin(p.breathPhase * Math.PI * 2) * p.breathAmplitude;
        const open = still && frame.activity !== 'speaking' ? 0.02 : p.visemeOpen;
        const blink = still ? 0 : Math.max(p.blink, idle.doze);

        if (vrm) {
          vrm.expressionManager?.setValue('aa', open);
          vrm.expressionManager?.setValue('blink', blink);
          const head = vrm.humanoid?.getNormalizedBoneNode('head');
          if (head) {
            head.rotation.x =
              THREE.MathUtils.degToRad(-bias.headUpDeg - p.gazeY * 10) * 0.25 +
              idle.headTiltRad +
              idle.doze * 0.22;
            head.rotation.y = THREE.MathUtils.degToRad(p.gazeX * 12) * 0.25;
          }
          const chest = vrm.humanoid?.getNormalizedBoneNode('chest');
          if (chest) {
            chest.rotation.x =
              THREE.MathUtils.degToRad(bias.leanDeg) * 0.12 + breath * 0.015 + idle.swayRad * 0.35;
          }
          return;
        }
        // 素体：分组变换直落（微动作/入定同契约显影）
        rig.apply(p, bias, breath, open, blink, idle, THREE);
      };

      rafId = requestAnimationFrame(loop);

      cleanupScene = () => {
        cancelAnimationFrame(rafId);
        renderer.dispose();
        renderer.domElement.remove();
      };
    })();

    return () => {
      disposed = true;
      cleanupScene?.();
    };
  });

  /** 程序化素体：胶囊体人形（零资产零许可面，同契约同映射）。 */
  function buildStandaloneRig(THREE: typeof import('three')) {
    const root = new THREE.Group();
    const tint = new THREE.Color(1, 0.824, 0.478);
    const mat = new THREE.MeshToonMaterial({color: tint});

    const torso = new THREE.Mesh(new THREE.CapsuleGeometry(0.32, 0.55, 6, 12), mat);
    torso.position.y = 1.05;
    const head = new THREE.Mesh(new THREE.SphereGeometry(0.27, 20, 16), mat);
    head.position.y = 1.72;
    const jaw = new THREE.Mesh(new THREE.BoxGeometry(0.14, 0.06, 0.1), mat);
    jaw.position.set(0, 1.62, 0.22);
    const eyeGeo = new THREE.SphereGeometry(0.045, 10, 8);
    const eyeL = new THREE.Mesh(eyeGeo, mat);
    eyeL.position.set(-0.09, 1.77, 0.23);
    const eyeR = new THREE.Mesh(eyeGeo, mat);
    eyeR.position.set(0.09, 1.77, 0.23);

    const armGeo = new THREE.CapsuleGeometry(0.09, 0.5, 4, 8);
    const armL = new THREE.Mesh(armGeo, mat);
    armL.position.set(-0.45, 1.12, 0);
    armL.rotation.z = 0.18;
    const armR = new THREE.Mesh(armGeo, mat);
    armR.position.set(0.45, 1.12, 0);
    armR.rotation.z = -0.18;

    const legGeo = new THREE.CapsuleGeometry(0.11, 0.55, 4, 8);
    const legL = new THREE.Mesh(legGeo, mat);
    legL.position.set(-0.17, 0.42, 0);
    const legR = new THREE.Mesh(legGeo, mat);
    legR.position.set(0.17, 0.42, 0);

    root.add(torso, head, jaw, eyeL, eyeR, armL, armR, legL, legR);

    return {
      root,
      apply(
        p: CharacterParams,
        bias: {leanDeg: number; curl: number; scale: number; headUpDeg: number},
        breath: number,
        open: number,
        blink: number,
        idle: IdleMotion,
        THREE_mod: typeof import('three'),
      ) {
        root.rotation.x = THREE_mod.MathUtils.degToRad(bias.leanDeg);
        root.rotation.z = idle.swayRad * 0.45;
        root.scale.setScalar(bias.scale * (1 + breath * 0.012 * p.breathAmplitude));
        head.rotation.x =
          THREE_mod.MathUtils.degToRad(-bias.headUpDeg - p.gazeY * 10) + idle.headTiltRad + idle.doze * 0.22;
        head.rotation.y = THREE_mod.MathUtils.degToRad(p.gazeX * 12);
        jaw.scale.set(1, 1 + open * 2.2, 1);
        const eyeScaleY = Math.max(0.08, 1 - blink * 0.92);
        eyeL.scale.y = eyeScaleY;
        eyeR.scale.y = eyeScaleY;
        armL.rotation.x = bias.curl * 0.9 - idle.stretch * 0.7;
        armR.rotation.x = bias.curl * 0.9 - idle.stretch * 0.7;
        torso.position.y = 1.05 + breath * 0.014 * p.breathAmplitude;
      },
    };
  }
</script>

<div class="character-host" bind:this={host}></div>

<style>
  .character-host {
    width: 100%;
    height: 100%;
    overflow: hidden;
  }
  .character-host :global(canvas) {
    width: 100%;
    height: 100%;
    display: block;
  }
</style>
