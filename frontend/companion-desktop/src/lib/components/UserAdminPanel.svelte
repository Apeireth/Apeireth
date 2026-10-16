<!-- 住户卡（D10 用户/域管理上屏）：用户列表（id/角色/客户端数）+ owner 视角
     角色调整/移除（管理端点 PATCH/DELETE /v1/admin/users/:id，契约 §10.6）+
     当前域节（self_status 域节合同 §10.3 同形，counts 缺探测口显式 null + reason）。
     只读态：读面被拒（501 no_role_face / 403 permission_denied / 网络失败）→
     只读展示 + CLI 指引（命令可复制，与写面同源改同一张用户表），零假旋钮。
     双入口同源：面板写面与 `apeireth user` CLI 互为镜像，CLI 指引常驻。 -->
<script lang="ts">
  import {ShieldCheck, Copy, RefreshCcw} from 'lucide-svelte';
  import type {ApeirethConfig} from '../types';
  import {describeCaught, fetchDomains, fetchUsers, normalizeBaseUrl} from '../runtime';
  import {gatewayAuthHeaders} from '../tauri-bridge';
  import {
    USER_ROLES,
    userAdminView,
    userCliBootstrap,
    userCliGuidance,
    userPanelMode,
    domainSectionView,
    patchUserRole,
    removeUser,
  } from '../user-admin';
  import type {DomainAdminRow, UserAdminRow} from '../user-admin';

  let {config}: {config: ApeirethConfig} = $props();

  let users = $state<UserAdminRow[]>([]);
  let domains = $state<DomainAdminRow[]>([]);
  let faceOk = $state(false);
  let faceStatus = $state(0);
  let faceCode = $state('');
  let errorText = $state('');
  let loading = $state(false);
  let busyId = $state('');
  let copied = $state('');

  const view = $derived(userAdminView(users));
  const mode = $derived(userPanelMode({ok: faceOk, status: faceStatus, code: faceCode, error: errorText}));
  // 当前域节：self_status 域节合同形状（§10.3 同形消费）；多域时逐域上屏。
  const domainSections = $derived(domains.map(domainSectionView));

  /** 管理写面注入传输缝：真 fetch + 令牌面鉴权头单源（与 metrics 卡同法）。 */
  const deps = {
    authHeaders: () => gatewayAuthHeaders(),
    fetchJson: async (url: string, init?: {method?: string; headers?: Record<string, string>; body?: string}) => {
      const res = await fetch(url, init);
      return {ok: res.ok, status: res.status, json: () => res.json().catch(() => null)};
    },
  };

  async function refresh(): Promise<void> {
    loading = true;
    try {
      const [usersRes, domainsRes] = await Promise.all([fetchUsers(config), fetchDomains(config)]);
      if ('users' in usersRes) {
        users = usersRes.users;
        faceOk = true;
        faceStatus = 200;
        faceCode = '';
        errorText = '';
      } else {
        // 错误即帧：status + code 保留，只读态据此判定（501/403/网络各有真因）。
        faceOk = false;
        faceStatus = usersRes.status;
        faceCode = usersRes.code;
        errorText = usersRes.error;
      }
      domains = 'domains' in domainsRes ? domainsRes.domains : [];
    } catch (error) {
      faceOk = false;
      errorText = describeCaught(error);
    } finally {
      loading = false;
    }
  }

  /** owner 视角角色调整（PATCH /v1/admin/users/:user_id {role}）。 */
  async function applyRole(userId: string, role: string): Promise<void> {
    busyId = userId;
    try {
      const result = await patchUserRole(normalizeBaseUrl(config.baseUrl), userId, role, deps);
      if (result.ok) {
        users = users.map((u) => (u.user_id === result.user.user_id ? result.user : u));
        errorText = '';
      } else {
        errorText = result.error;
      }
    } catch (error) {
      errorText = describeCaught(error);
    } finally {
      busyId = '';
    }
  }

  /** owner 视角移除住户（DELETE /v1/admin/users/:user_id）。 */
  async function dropUser(userId: string): Promise<void> {
    busyId = userId;
    try {
      const result = await removeUser(normalizeBaseUrl(config.baseUrl), userId, deps);
      if (result.ok) {
        users = users.filter((u) => u.user_id !== userId);
        errorText = '';
      } else {
        errorText = result.error;
      }
    } catch (error) {
      errorText = describeCaught(error);
    } finally {
      busyId = '';
    }
  }

  /** CLI 指引复制（命令可复制；失败报真因，不静默）。 */
  async function copyCli(command: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(command);
      copied = command;
    } catch (error) {
      errorText = describeCaught(error);
    }
  }

  $effect(() => {
    void refresh();
  });
</script>

<div class="ua-card">
  <div class="ua-head">
    <span class="ua-title"><ShieldCheck size={13} /> 住户</span>
    <button class="ua-refresh" onclick={() => void refresh()} disabled={loading}>
      <span class="ua-ico"><RefreshCcw size={13} /></span>
      <span>{loading ? '刷新中…' : '刷新'}</span>
    </button>
  </div>

  <div class="ua-body">
    <div class="ua-list">
      <div class="ua-list-head">
        <span>住户列表</span>
        <span class="ua-mode" class:ro={mode.mode === 'read-only'}>
          {mode.mode === 'manage' ? '管理视角' : '只读态'}
        </span>
      </div>
      {#if mode.reason}
        <p class="ua-reason">{mode.reason}</p>
      {/if}
      {#each view.rows as row (row.id)}
        <div class="ua-row">
          <div class="ua-row-main">
            <b class="ua-id">{row.id}</b>
            <span class="ua-clients">{row.clientCount} 个客户端</span>
          </div>
          {#if mode.mode === 'manage'}
            <div class="ua-row-ops">
              <select
                class="ua-role"
                value={row.role}
                disabled={busyId === row.id}
                onchange={(e) => void applyRole(row.id, (e.currentTarget as HTMLSelectElement).value)}
              >
                {#each USER_ROLES as role (role)}
                  <option value={role} selected={role === row.role}>{role}</option>
                {/each}
              </select>
              <button class="ua-drop" disabled={busyId === row.id} onclick={() => void dropUser(row.id)}>移除</button>
            </div>
          {:else}
            <span class="ua-role-tag">{row.role}</span>
          {/if}
        </div>
      {:else}
        <p class="ua-empty">{faceOk ? '暂无住户记录' : '住户列表不可用（只读态）'}</p>
      {/each}
    </div>

    <div class="ua-domain">
      <div class="ua-list-head"><span>当前域节</span><span class="ua-src">self_status 域节同形</span></div>
      {#each domainSections as sec (sec.user_id)}
        <div class="ua-domain-row">
          <b>{sec.display_name}</b>
          <span class="ua-mono">{sec.user_id}</span>
          <span class="ua-mono">role {sec.role}</span>
          <span class="ua-mono">
            counts sessions {sec.counts.sessions ?? 'null'} · memories {sec.counts.memories ?? 'null'} · protected {sec.counts.protected ?? 'null'}
          </span>
          {#if sec.counts.reason}
            <small class="ua-reason">{sec.counts.reason}</small>
          {/if}
        </div>
      {:else}
        <p class="ua-empty">暂无域记录</p>
      {/each}
    </div>

    <div class="ua-cli">
      <div class="ua-list-head"><span>CLI 指引（命令可复制，与写面同源）</span></div>
      {#each view.rows as row (row.id)}
        {#each userCliGuidance(row.id, row.role) as command (command)}
          <div class="ua-cli-row">
            <code class="ua-mono">{command}</code>
            <button class="ua-copy" onclick={() => void copyCli(command)} aria-label="复制命令">
              <Copy size={12} />{copied === command ? '已复制' : '复制'}
            </button>
          </div>
        {/each}
      {:else}
        <div class="ua-cli-row">
          <code class="ua-mono">{userCliBootstrap('<user-id>')}</code>
          <button class="ua-copy" onclick={() => void copyCli(userCliBootstrap('<user-id>'))} aria-label="复制命令">
            <Copy size={12} />{copied === userCliBootstrap('<user-id>') ? '已复制' : '复制'}
          </button>
        </div>
      {/each}
    </div>
  </div>

  {#if errorText}
    <p class="ua-error">{errorText}</p>
  {/if}
</div>

<style>
  .ua-card {
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .ua-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--line);
    background: var(--surface);
  }
  .ua-title {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .ua-refresh {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--text);
    font-size: 11px;
    cursor: pointer;
  }
  .ua-refresh:disabled {
    cursor: default;
    opacity: 0.7;
  }
  .ua-ico {
    display: inline-flex;
  }
  .ua-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px 14px;
  }
  .ua-list-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 6px;
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
    letter-spacing: 0.04em;
  }
  .ua-mode {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--ok, var(--muted));
  }
  .ua-mode.ro {
    color: var(--amber);
  }
  .ua-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 6px 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
  }
  .ua-row-main {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
  }
  .ua-id {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .ua-clients {
    font-size: 11px;
    color: var(--faint);
    white-space: nowrap;
  }
  .ua-row-ops {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .ua-role {
    font-family: var(--mono);
    font-size: 11px;
    padding: 2px 6px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--surface-2);
    color: var(--text);
  }
  .ua-role-tag {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--muted);
  }
  .ua-drop {
    padding: 2px 8px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--surface-2);
    color: var(--amber);
    font-size: 11px;
    cursor: pointer;
  }
  .ua-domain-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
  }
  .ua-domain-row b {
    font-size: 12px;
    color: var(--text);
  }
  .ua-mono {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .ua-cli-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 4px 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
  }
  .ua-copy {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--surface-2);
    color: var(--text);
    font-size: 10px;
    cursor: pointer;
    white-space: nowrap;
  }
  .ua-empty {
    margin: 0;
    padding: 6px 8px;
    font-size: 11px;
    color: var(--faint);
  }
  .ua-reason {
    margin: 0 0 6px;
    font-size: 11px;
    color: var(--amber);
  }
  .ua-src {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
  }
  .ua-error {
    margin: 0;
    padding: 8px 14px 12px;
    font-size: 11px;
    color: var(--amber);
  }
</style>
