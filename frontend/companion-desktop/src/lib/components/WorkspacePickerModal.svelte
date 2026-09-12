<script lang="ts">
  import {X, Folder} from 'lucide-svelte';

  interface WorkspacePickerModalProps {
    open: boolean;
    current: string;
    suggestions: string[];
    onPick: (dir: string) => void;
    onCancel: () => void;
  }

  let {
    open = false,
    current = '',
    suggestions = [],
    onPick,
    onCancel,
  }: WorkspacePickerModalProps = $props();

  let input = $state('');
  let error = $state('');

  function validate(dir: string): string | null {
    const trimmed = dir.trim();
    if (!trimmed) return '请输入工作区路径';
    if (trimmed.startsWith('~')) return null;
    if (trimmed.startsWith('/')) return null;
    if (/^[A-Za-z]:[\\/]/.test(trimmed)) return null;
    if (trimmed.startsWith('\\\\')) return null;
    return '请输入绝对路径，或以 ~ 开头';
  }

  function confirmInput(): void {
    const trimmed = input.trim();
    const problem = validate(trimmed);
    error = problem ?? '';
    if (!problem) {
      onPick(trimmed);
    }
  }

  function handleInputKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter') {
      e.preventDefault();
      confirmInput();
    }
  }

  function handleWindowKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape' && open) {
      e.stopPropagation();
      onCancel();
    }
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if open}
  <div
    class="picker-backdrop"
    onclick={(e) => {
      if (e.target === e.currentTarget) onCancel();
    }}
    role="presentation"
  >
    <div
      class="picker-card"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="picker-title"
    >
      <div class="picker-header">
        <div class="title-wrap">
          <span class="title-icon"><Folder size={16} /></span>
          <h3 id="picker-title">选择工作区</h3>
        </div>
        <button class="close-btn" onclick={onCancel} aria-label="关闭">
          <X size={16} />
        </button>
      </div>

      <div class="picker-body">
        <div class="current-block">
          <span class="block-label">当前工作区</span>
          <code class="current-path">{current || '（未设置）'}</code>
        </div>

        {#if suggestions.length > 0}
          <div class="suggestions">
            <span class="block-label">建议</span>
            <div class="suggestion-list">
              {#each suggestions as dir (dir)}
                <button class="suggestion" onclick={() => onPick(dir)} title={dir}>
                  <Folder size={13} />
                  <span class="dir-text">{dir}</span>
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <div class="manual">
          <label class="block-label" for="workspace-input">手动输入路径</label>
          <input
            id="workspace-input"
            type="text"
            bind:value={input}
            onkeydown={handleInputKeydown}
            placeholder={current ? `如 ${current}` : 'C:\\projects\\apeireth'}
          />
          {#if error}<p class="error-text">{error}</p>{/if}
        </div>
      </div>

      <div class="picker-footer">
        <button class="quiet-btn" onclick={onCancel}>取消</button>
        <button class="primary-btn" onclick={confirmInput}>选择</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .picker-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(4px);
    display: grid;
    place-items: center;
    z-index: 1000;
    padding: 20px;
    animation: fadeIn 0.15s ease-out;
  }
  .picker-card {
    width: 100%;
    max-width: 440px;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: 12px;
    box-shadow: var(--shadow);
    overflow: hidden;
  }
  .picker-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px 12px;
    border-bottom: 1px solid var(--line);
  }
  .title-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .title-icon {
    display: grid;
    place-items: center;
    color: var(--amber);
  }
  .picker-header h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
  }
  .close-btn {
    border: 0;
    background: transparent;
    color: var(--muted);
    padding: 4px;
    border-radius: 6px;
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .close-btn:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .picker-body {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .block-label {
    display: block;
    font-size: 11px;
    font-weight: 500;
    color: var(--faint);
    margin-bottom: 6px;
  }
  .current-path {
    display: block;
    padding: 8px 10px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--text);
    font-family: var(--mono);
    font-size: 12px;
    word-break: break-all;
  }
  .suggestion-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 160px;
    overflow-y: auto;
  }
  .suggestion {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    border: 1px solid var(--line);
    background: transparent;
    border-radius: 6px;
    color: var(--muted);
    font-size: 12px;
    cursor: pointer;
    text-align: left;
  }
  .suggestion:hover {
    border-color: var(--amber-line);
    color: var(--text);
    background: var(--surface-2);
  }
  .dir-text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--mono);
  }
  .manual input {
    width: 100%;
    padding: 9px 12px;
    border-radius: 6px;
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    color: var(--text);
    font-size: 13px;
  }
  .manual input:focus {
    outline: 0;
    border-color: var(--amber-line);
    box-shadow: 0 0 0 2px var(--amber-wash);
  }
  .error-text {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--danger);
  }
  .picker-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 12px 20px 16px;
    background: var(--surface-2);
    border-top: 1px solid var(--line);
  }
  .quiet-btn {
    padding: 7px 14px;
    border-radius: 6px;
    background: transparent;
    border: 1px solid var(--line-strong);
    color: var(--muted);
    font-size: 13px;
    cursor: pointer;
  }
  .quiet-btn:hover {
    color: var(--text);
    background: var(--surface-3);
  }
  .primary-btn {
    padding: 7px 16px;
    border-radius: 6px;
    background: var(--amber);
    border: 1px solid var(--amber);
    color: #1a1408;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .primary-btn:hover {
    background: var(--amber-hi);
  }
  @keyframes fadeIn {
    from { opacity: 0; transform: scale(0.98); }
    to { opacity: 1; transform: scale(1); }
  }
</style>
