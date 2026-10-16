// daemon 运维面（D12 上屏）纯逻辑件：侧车连接态/端点/版本的只读展示映射、
// 常驻服务探测报告映射、服务启停指引（CLI 命令可复制）。
//
// 只读展示 + 指引纪律（零装边界）：本文件零动作面——不安装、不启停、不代跑
// 命令。指引注册表只登记运维手册的 status/start/stop 三条（看状态 + 启停）；
// 安装/卸载与参数面走运维手册（docs/02-guides/server-deployment.md、
// packaging/service/README.md），界面不代跑、不加安装按钮。
//
// 本文件零运行时依赖（不 import svelte / DOM 专属 API），纯函数可被 Node
// 直接 import 测试（tests/daemon-ops.mjs）。读数面来源：桌面壳监督面
// （src-tauri `BackendSupervisor`，经 desktop-bridge `getBackendStatus` /
// `getResidentProbe`）；非桌面环境读不到就是读不到——如实标「仅桌面版可读」，
// 不伪造状态。
//
// 合同出处：`frontend/companion-desktop/src-tauri/src/backend_supervisor.rs`
// （`BackendInfo` / `ResidentProbeReport`）。

import {DASH} from './chat-shell/turn-telemetry.ts';

// ---------------------------------------------------------------------------
// 侧车监督面读数（镜像 Rust `BackendInfo` 的安全投影，无密钥字段）
// ---------------------------------------------------------------------------

/** 后端生命周期状态（镜像 Rust `BackendState`）。 */
export type DaemonState = 'Stopped' | 'Starting' | 'Ready' | 'Failed' | 'Stopping';

/** 后端归属（镜像 Rust `BackendOwnership`）。 */
export type DaemonOwnership = 'OwnedByDesktop' | 'External';

/** 侧车状态读数（镜像 desktop-bridge `BackendStatus`）。 */
export interface DaemonStatusLike {
  state: DaemonState;
  ownership: DaemonOwnership;
  pid: number | null;
  endpoint: string | null;
  port: number | null;
  restart_count: number;
  backend_version: string | null;
}

/** 常驻服务健康判定（镜像 Rust `ResidentProbeHealth`）。 */
export type ResidentProbeHealth = 'live' | 'stale' | 'absent';

/** 常驻服务探测报告（镜像 Rust `ResidentProbeReport`）。 */
export interface ResidentProbeLike {
  descriptor_present: boolean;
  endpoint: string | null;
  pid: number | null;
  health: ResidentProbeHealth;
}

/** 状态标签（未知值原样透出，不粉饰）。 */
export const DAEMON_STATE_LABELS: Record<DaemonState, string> = {
  Stopped: '未运行',
  Starting: '启动中',
  Ready: '运行中',
  Failed: '异常',
  Stopping: '停止中',
};

/** 归属标签（生命周期归属如实陈述）。 */
export const DAEMON_OWNERSHIP_LABELS: Record<DaemonOwnership, string> = {
  OwnedByDesktop: '桌面拉起的侧车（生命周期归桌面壳）',
  External: '外部常驻服务（直连，生命周期归服务管理面）',
};

/** 常驻探测判定标签。 */
export const RESIDENT_PROBE_LABELS: Record<ResidentProbeHealth, string> = {
  live: '在跑（/health 应答）',
  stale: '占位陈旧（端口不应答）',
  absent: '无占位（未探测到常驻）',
};

export function daemonStateLabel(state: string): string {
  return DAEMON_STATE_LABELS[state as DaemonState] ?? state;
}

export function daemonOwnershipLabel(ownership: string): string {
  return DAEMON_OWNERSHIP_LABELS[ownership as DaemonOwnership] ?? ownership;
}

export function residentProbeLabel(health: string): string {
  return RESIDENT_PROBE_LABELS[health as ResidentProbeHealth] ?? health;
}

// ---------------------------------------------------------------------------
// 运维卡视图映射（只读展示；未知位诚实「—」）
// ---------------------------------------------------------------------------

/** 运维卡的一行。 */
export interface DaemonRowView {
  key: string;
  label: string;
  /** 展示串；未知位「—」。 */
  value: string;
  /** 口径注记（如实来源/原因）。 */
  note: string;
}

/** 运维卡视图（侧车状态行 + 常驻探测行）。 */
export interface DaemonOpsView {
  /** false = 非桌面环境读不到监督面（如实标注，不伪造状态）。 */
  available: boolean;
  statusRows: DaemonRowView[];
  probeRows: DaemonRowView[];
}

const WEB_MODE_NOTE = '仅桌面版可读（web 构建无监督面）';

/**
 * 运维卡视图映射：`status`/`probe` 为 null（非桌面/读取失败）时如实标
 * 「仅桌面版可读」，值位「—」；有读数按真值渲染，缺位逐位「—」。
 */
export function daemonOpsView(
  status: DaemonStatusLike | null,
  probe: ResidentProbeLike | null,
): DaemonOpsView {
  const available = status !== null || probe !== null;
  const unavailableNote = available ? '' : WEB_MODE_NOTE;

  const statusRows: DaemonRowView[] =
    status === null
      ? [
          {key: 'state', label: '侧车连接态', value: DASH, note: unavailableNote},
          {key: 'ownership', label: '归属', value: DASH, note: unavailableNote},
          {key: 'endpoint', label: '端点', value: DASH, note: unavailableNote},
          {key: 'version', label: '版本', value: DASH, note: unavailableNote},
        ]
      : [
          {
            key: 'state',
            label: '侧车连接态',
            value: daemonStateLabel(status.state),
            note:
              status.pid === null && status.restart_count === 0
                ? '未见进程号（状态未上报 pid）'
                : `pid ${status.pid ?? DASH} · 重启 ${status.restart_count} 次`,
          },
          {
            key: 'ownership',
            label: '归属',
            value: daemonOwnershipLabel(status.ownership),
            note: '',
          },
          {
            key: 'endpoint',
            label: '端点',
            value: status.endpoint ?? DASH,
            note: status.endpoint === null ? '未上报端点（后端未就绪或读取失败）' : '',
          },
          {
            key: 'version',
            label: '版本',
            value: status.backend_version ?? DASH,
            note: status.backend_version === null ? '侧车未上报版本' : '',
          },
        ];

  const probeRows: DaemonRowView[] =
    probe === null
      ? [{key: 'resident', label: '常驻服务探测', value: DASH, note: unavailableNote}]
      : [
          {
            key: 'resident',
            label: '常驻服务探测',
            value: residentProbeLabel(probe.health),
            note:
              probe.descriptor_present && probe.endpoint !== null
                ? `占位 ${probe.endpoint}${probe.pid === null ? '' : ` · pid ${probe.pid}`}`
                : '占位读面：读不懂即无常驻（不猜）',
          },
        ];

  return {available, statusRows, probeRows};
}

// ---------------------------------------------------------------------------
// 服务启停指引（CLI 命令可复制；只读展示 + 指引，零装边界）
// ---------------------------------------------------------------------------

/** 指引动作键（零装边界：只有看状态 + 启停，无安装/卸载）。 */
export type DaemonCommandKey = 'status' | 'start' | 'stop';

/** 指引行（命令文本原样可复制；界面不代跑）。 */
export interface DaemonCommandRow {
  key: DaemonCommandKey;
  label: string;
  command: string;
  desc: string;
}

/** 服务脚本入口（运维手册同源；`pwsh -NoProfile -File …` 可直接粘贴执行）。 */
export const DAEMON_SERVICE_SCRIPT = 'pwsh -NoProfile -File packaging/service/apeireth-service.ps1';

/**
 * 启停指引注册表（运维手册 §3 同源）。零装边界：这里只登记
 * status/start/stop——安装/卸载走手册人工执行，界面不代跑、不加安装按钮。
 */
export const DAEMON_GUIDANCE_COMMANDS: ReadonlyArray<DaemonCommandRow> = [
  {
    key: 'status',
    label: '看状态（只读）',
    command: `${DAEMON_SERVICE_SCRIPT} status`,
    desc: '任务安装态/常驻占位/健康检查/令牌在否（只报 present/absent，永不回显值）。',
  },
  {
    key: 'start',
    label: '启动常驻服务',
    command: `${DAEMON_SERVICE_SCRIPT} start`,
    desc: '已在跑则报 already running；桌面壳探测到常驻即直连，不重复拉起侧车。',
  },
  {
    key: 'stop',
    label: '优雅停机',
    command: `${DAEMON_SERVICE_SCRIPT} stop`,
    desc: '三级递降：令牌门停机端点 → 停计划任务 → 兜底强杀（非优雅路径收尾不保证）。',
  },
];

/** 零装边界注记（如实声明界面不代跑安装/启停）。 */
export const DAEMON_ZERO_INSTALL_NOTE =
  '本卡只读展示 + 指引：不代跑安装/启停、不加安装按钮。安装/卸载与参数面（-Port / -Bind / -RotateToken）见运维手册 docs/02-guides/server-deployment.md 与 packaging/service/README.md。';
