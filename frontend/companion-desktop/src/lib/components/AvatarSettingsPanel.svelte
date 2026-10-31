<!-- 形象设置面板（设置 › 外观 › 形象）—— 桌宠形象层与自带形象的正经入口
     #
     # 与桌宠窗右键菜单同一套持久化口径（layerId: localStorage `apeireth:pet:
     # avatar-layer`；自带形象: user-avatar.ts 的 IndexedDB），跨窗同步借
     # storage 事件（同源窗口约定）：形象层切换即时生效、形象文件变更经
     # `apeireth:pet:avatar-reload` 标记通知桌宠窗重读（桌宠窗下次事件到达即
     # 换装；极端情况下重启桌宠窗生效——如实标注，不假装实时）。
     #
     # 免责纪律：接入形象文件必经 USER_AVATAR_DISCLAIMER 确认（与桌宠窗同文）。-->
<script lang="ts">
  import {onMount} from 'svelte';
  import {
    DEFAULT_AVATAR_LAYER_ID,
    CHARACTER_AVATAR_LAYER_ID,
    SKELETAL_2D_AVATAR_LAYER_ID,
    FALLBACK_AVATAR_LAYER_ID,
  } from '../pet/pet-contract';
  import {
    validateUserAvatarFile,
    getUserAvatar,
    putUserAvatar,
    clearUserAvatar,
    normalizeVrmMetaLicense,
    USER_AVATAR_DISCLAIMER,
    EMPTY_USER_AVATAR_META,
    type UserAvatarMeta,
  } from '../pet/user-avatar';
  import {getPetEnabled, setPetEnabled} from '../tauri-bridge';

  const LAYER_KEY = 'apeireth:pet:avatar-layer';
  const RELOAD_KEY = 'apeireth:pet:avatar-reload';

  const LAYER_OPTIONS = [
    {id: DEFAULT_AVATAR_LAYER_ID, label: '光态哨兵', desc: '默认 · 省电 · 金色光核（微光在场）'},
    {id: CHARACTER_AVATAR_LAYER_ID, label: '角色形象', desc: '开放 3D 人形格式 · 卡通渲染'},
    {id: SKELETAL_2D_AVATAR_LAYER_ID, label: '2D 骨骼', desc: '程序化骨骼小人 · 零 WebGL'},
    {id: FALLBACK_AVATAR_LAYER_ID, label: '微光简模', desc: 'T0 廉价档 · 单点余烬'},
  ] as const;

  let layerId = $state(DEFAULT_AVATAR_LAYER_ID);
  let userName = $state<string | null>(null);
  let avatarMeta = $state<UserAvatarMeta>(EMPTY_USER_AVATAR_META);
  let notice = $state<string | null>(null);
  let petEnabled = $state(false);
  let petNotice = $state<string | null>(null);
  let disclaimerFile = $state<File | null>(null);
  let fileInput = $state<HTMLInputElement | null>(null);

  onMount(() => {
    layerId = localStorage.getItem(LAYER_KEY) ?? DEFAULT_AVATAR_LAYER_ID;
    void (async () => {
      // 桌宠自动接管初值（Rust 侧 pet-prefs.json；桌面外恒 false = 关闭）
      petEnabled = await getPetEnabled();
    })();
    void (async () => {
      try {
        const stored = await getUserAvatar();
        if (stored) {
          userName = '本机自带形象';
          avatarMeta = EMPTY_USER_AVATAR_META; // 元数据由渲染层读文件后才有，本面板如实标未声明
        }
      } catch {
        // 读不到就是没有（不编造）
      }
    })();
  });

  function selectLayer(id: string): void {
    layerId = id;
    localStorage.setItem(LAYER_KEY, id);
  }

  /** 桌宠总开关：写 Rust 侧 pet-prefs.json（跨启动生效），关闭即刻收起在场桌宠。 */
  async function setPetTakeover(next: boolean): Promise<void> {
    petEnabled = next;
    const ok = await setPetEnabled(next);
    const text = ok
      ? next
        ? '已开启：主窗收起时桌宠自动接管右下角'
        : '已关闭：桌宠不再自动接管（在场桌宠已收起）'
      : '当前不是桌面环境，设置未保存';
    petNotice = text;
    window.setTimeout(() => {
      if (petNotice === text) petNotice = null;
    }, 8000);
  }

  function showNotice(text: string): void {
    notice = text;
    window.setTimeout(() => {
      if (notice === text) notice = null;
    }, 8000);
  }

  function pickFile(): void {
    fileInput?.click();
  }

  async function onFileSelected(e: Event): Promise<void> {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0] ?? null;
    input.value = '';
    if (!file) return;
    let head: Uint8Array | null = null;
    try {
      head = new Uint8Array(await file.slice(0, 8).arrayBuffer());
    } catch {
      head = null;
    }
    const verdict = validateUserAvatarFile(file.name, file.size, head);
    if (!verdict.ok) {
      showNotice(verdict.reason);
      return;
    }
    disclaimerFile = file;
  }

  async function acceptDisclaimer(): Promise<void> {
    const file = disclaimerFile;
    disclaimerFile = null;
    if (!file) return;
    try {
      await putUserAvatar(file, Date.now());
    } catch (caught) {
      showNotice(`形象持久化失败：${caught instanceof Error ? caught.message : String(caught)}`);
      return;
    }
    userName = file.name;
    selectLayer(CHARACTER_AVATAR_LAYER_ID);
    localStorage.setItem(RELOAD_KEY, String(Date.now()));
    showNotice('已换上你的形象（桌宠窗即时换装；如未生效请重开桌宠窗）');
  }

  async function removeAvatar(): Promise<void> {
    try {
      await clearUserAvatar();
    } catch {
      showNotice('本机存档清除失败');
      return;
    }
    userName = null;
    avatarMeta = EMPTY_USER_AVATAR_META;
    localStorage.setItem(RELOAD_KEY, String(Date.now()));
    showNotice('已移除自带形象');
  }
</script>

<div class="avatar-settings">
  <div class="pet-master">
    <h3 class="block-title">桌宠</h3>
    <p class="block-desc">主窗收起（关进托盘）时，桌宠是否自动接管屏幕右下角。默认关闭；托盘菜单「桌宠」仍可手动召唤。</p>
    <label class="layer-option" class:selected={petEnabled}>
      <input
        type="checkbox"
        name="pet-auto-takeover"
        checked={petEnabled}
        onchange={(event) => setPetTakeover((event.currentTarget as HTMLInputElement).checked)}
      />
      <span class="layer-name">主窗收起时自动接管</span>
      <span class="layer-desc">{petEnabled ? '已开启 · 收起主窗即出现在右下角' : '已关闭 · 桌宠不会自动出现'}</span>
    </label>
    {#if petNotice}
      <div class="byo-notice">{petNotice}</div>
    {/if}
  </div>

  <h3 class="block-title">形象</h3>
  <p class="block-desc">桌宠穿什么：形象层与自带形象。切换即写入本地配置，桌宠窗即时生效。</p>

  <div class="layer-list" role="radiogroup" aria-label="形象层">
    {#each LAYER_OPTIONS as option (option.id)}
      <label class="layer-option" class:selected={layerId === option.id}>
        <input
          type="radio"
          name="pet-avatar-layer"
          value={option.id}
          checked={layerId === option.id}
          onchange={() => selectLayer(option.id)}
        />
        <span class="layer-name">{option.label}</span>
        <span class="layer-desc">{option.desc}</span>
      </label>
    {/each}
  </div>

  <div class="byo-block">
    <div class="byo-title">自带形象（BYO）</div>
    <div class="byo-desc">
      载入你自己的形象文件（.vrm / .glb，≤64MB）。许可责任由你自担——接入前请自行核对授权条款。
    </div>
    {#if userName}
      <div class="byo-current">
        当前：{userName}
        <button type="button" class="byo-btn" onclick={removeAvatar}>移除</button>
      </div>
    {/if}
    <button type="button" class="byo-btn primary" onclick={pickFile}>
      {userName ? '换一个形象文件…' : '接入形象文件…'}
    </button>
    {#if notice}
      <div class="byo-notice">{notice}</div>
    {/if}
  </div>

  <input bind:this={fileInput} type="file" accept=".vrm,.glb" style="display: none" onchange={onFileSelected} />

  {#if disclaimerFile}
    <div class="disclaimer-inline" role="alertdialog" aria-label="自带形象免责声明">
      <div class="disclaimer-text">{USER_AVATAR_DISCLAIMER}</div>
      <div class="disclaimer-file">文件：{disclaimerFile.name}</div>
      <div class="disclaimer-actions">
        <button type="button" class="byo-btn primary" onclick={acceptDisclaimer}>我确认有权使用，换上</button>
        <button type="button" class="byo-btn" onclick={() => (disclaimerFile = null)}>取消</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .avatar-settings {
    display: block;
  }

  .pet-master {
    margin-bottom: 18px;
    padding-bottom: 14px;
    border-bottom: 1px dashed color-mix(in srgb, var(--border) 60%, transparent);
  }

  .layer-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 12px 0;
  }

  .layer-option {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: auto auto;
    column-gap: 10px;
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid rgba(232, 224, 204, 0.14);
    cursor: pointer;
  }
  .layer-option.selected {
    border-color: rgba(255, 210, 122, 0.5);
  }
  .layer-name {
    font-size: 13px;
  }
  .layer-desc {
    grid-column: 2;
    font-size: 11px;
    opacity: 0.65;
  }

  .byo-block {
    margin-top: 14px;
    padding: 10px;
    border-radius: 8px;
    border: 1px solid rgba(232, 224, 204, 0.14);
  }
  .byo-title {
    font-size: 13px;
    margin-bottom: 4px;
  }
  .byo-desc {
    font-size: 11px;
    opacity: 0.7;
    line-height: 1.7;
    margin-bottom: 8px;
  }
  .byo-current {
    font-size: 12px;
    margin-bottom: 8px;
    word-break: break-all;
  }

  .byo-btn {
    appearance: none;
    background: none;
    border: 1px solid rgba(232, 224, 204, 0.3);
    border-radius: 6px;
    color: inherit;
    font-size: 12px;
    padding: 5px 12px;
    cursor: pointer;
  }
  .byo-btn.primary {
    border-color: rgba(255, 210, 122, 0.5);
    color: #ffd27a;
  }

  .byo-notice {
    margin-top: 8px;
    font-size: 11px;
    opacity: 0.8;
  }

  .disclaimer-inline {
    margin-top: 12px;
    padding: 10px;
    border-radius: 8px;
    border: 1px solid rgba(255, 210, 122, 0.35);
  }
  .disclaimer-text {
    font-size: 12px;
    line-height: 1.7;
  }
  .disclaimer-file {
    margin-top: 6px;
    font-family: ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, monospace;
    font-size: 10px;
    opacity: 0.6;
    word-break: break-all;
  }
  .disclaimer-actions {
    margin-top: 10px;
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
</style>
