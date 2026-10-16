// daemon 运维卡（D12 上屏：真实模块 src/lib/daemon-ops.ts），并对
// SettingsView.svelte / desktop-bridge.ts / backend_supervisor.rs 做源码级镜像校验。
// 契约：
//   1. 状态读取：侧车连接态/归属/端点/版本照监督面真值渲染；未知位「—」，
//      不伪造状态；非桌面环境如实标「仅桌面版可读」。
//   2. 常驻服务探测：live / stale / absent 三态如实（占位照报、陈旧如实）。
//   3. 服务启停指引：CLI 命令可复制（status/start/stop 三条，运维手册同源）；
//      零装边界——注册表无安装/卸载行，界面不代跑、不加安装按钮。
//   4. 即效缝零回归：新面只读，无保存按钮、无写面。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

import {
  DAEMON_GUIDANCE_COMMANDS,
  DAEMON_SERVICE_SCRIPT,
  DAEMON_ZERO_INSTALL_NOTE,
  daemonOpsView,
  daemonOwnershipLabel,
  daemonStateLabel,
  residentProbeLabel,
} from '../src/lib/daemon-ops.ts';
import {DASH} from '../src/lib/chat-shell/turn-telemetry.ts';

const testsDir = dirname(fileURLToPath(import.meta.url));
const settingsSrc = readFileSync(
  join(testsDir, '..', 'src', 'lib', 'views', 'SettingsView.svelte'),
  'utf8',
);
const desktopBridgeSrc = readFileSync(
  join(testsDir, '..', 'src', 'lib', 'desktop-bridge.ts'),
  'utf8',
);
const supervisorRust = readFileSync(
  join(testsDir, '..', 'src-tauri', 'src', 'backend_supervisor.rs'),
  'utf8',
);
const supervisorLibRust = readFileSync(
  join(testsDir, '..', 'src-tauri', 'src', 'lib.rs'),
  'utf8',
);

const dataStart = settingsSrc.indexOf("{:else if activeSection === 'data'}");
const dataEnd = settingsSrc.indexOf("{:else if activeSection === 'user'}");
const dataSection = settingsSrc.slice(dataStart, dataEnd);

console.log('--- daemon ops card (D12) ---');

const rowOf = (rows, key) => rows.find((row) => row.key === key);

// ---------------------------------------------------------------------------
// 1. 状态读取：监督面真值渲染 + 未知位「—」
// ---------------------------------------------------------------------------
{
  const view = daemonOpsView(
    {
      state: 'Ready',
      ownership: 'External',
      pid: 4242,
      endpoint: 'http://127.0.0.1:8080',
      port: 8080,
      restart_count: 0,
      backend_version: '2.0.0-rc.4',
    },
    null,
  );
  assert.equal(view.available, true, '有读数 = 可用');
  const rows = view.statusRows;
  assert.equal(rowOf(rows, 'state').value, '运行中', '连接态照监督面状态机');
  assert.ok(rowOf(rows, 'state').note.includes('pid 4242'), '连接态行带进程号（诊断元数据）');
  assert.equal(
    rowOf(rows, 'ownership').value,
    '外部常驻服务（直连，生命周期归服务管理面）',
    '归属如实（外部常驻不归桌面停机）',
  );
  assert.equal(rowOf(rows, 'endpoint').value, 'http://127.0.0.1:8080', '端点真值');
  assert.equal(rowOf(rows, 'version').value, '2.0.0-rc.4', '版本真值');

  // 缺位逐位「—」+ 诚实注记（不编数）。
  const sparse = daemonOpsView(
    {
      state: 'Starting',
      ownership: 'OwnedByDesktop',
      pid: null,
      endpoint: null,
      port: null,
      restart_count: 2,
      backend_version: null,
    },
    null,
  );
  assert.equal(rowOf(sparse.statusRows, 'state').value, '启动中');
  assert.equal(rowOf(sparse.statusRows, 'endpoint').value, DASH, '端点未上报 → 「—」');
  assert.ok(rowOf(sparse.statusRows, 'endpoint').note.includes('未上报端点'), '缺位给真因注记');
  assert.equal(rowOf(sparse.statusRows, 'version').value, DASH, '版本未上报 → 「—」');
  assert.ok(rowOf(sparse.statusRows, 'state').note.includes('重启 2 次'), '重启计数如实');

  // 非桌面环境：读不到就是读不到（不伪造状态）。
  const web = daemonOpsView(null, null);
  assert.equal(web.available, false, 'web 构建无监督面');
  for (const row of [...web.statusRows, ...web.probeRows]) {
    assert.equal(row.value, DASH, `${row.key} 无监督面 → 「—」`);
    assert.ok(row.note.includes('仅桌面版可读'), `${row.key} 如实标注来源边界`);
  }
  console.log('  -> PASS: 运维卡状态读取（真值渲染/未知「—」/web 如实标注）');
}

// ---------------------------------------------------------------------------
// 2. 状态标签词表（未知值原样透出，不粉饰）
// ---------------------------------------------------------------------------
{
  assert.equal(daemonStateLabel('Ready'), '运行中');
  assert.equal(daemonStateLabel('Stopped'), '未运行');
  assert.equal(daemonStateLabel('Starting'), '启动中');
  assert.equal(daemonStateLabel('Failed'), '异常');
  assert.equal(daemonStateLabel('Stopping'), '停止中');
  assert.equal(daemonStateLabel('Mystery'), 'Mystery', '未知状态原样透出（不猜）');
  assert.equal(daemonOwnershipLabel('OwnedByDesktop'), '桌面拉起的侧车（生命周期归桌面壳）');
  assert.equal(daemonOwnershipLabel('Mystery'), 'Mystery', '未知归属原样透出');
  console.log('  -> PASS: 状态/归属标签词表（未知值不粉饰）');
}

// ---------------------------------------------------------------------------
// 3. 常驻服务探测映射：live / stale / absent 如实
// ---------------------------------------------------------------------------
{
  const live = daemonOpsView(null, {
    descriptor_present: true,
    endpoint: 'http://127.0.0.1:8080',
    pid: 4242,
    health: 'live',
  });
  const liveRow = rowOf(live.probeRows, 'resident');
  assert.equal(liveRow.value, '在跑（/health 应答）', '活常驻 = live');
  assert.ok(liveRow.note.includes('http://127.0.0.1:8080'), '占位端点照报');
  assert.ok(liveRow.note.includes('pid 4242'), '占位进程号照报');

  const stale = daemonOpsView(null, {
    descriptor_present: true,
    endpoint: 'http://127.0.0.1:8080',
    pid: 4242,
    health: 'stale',
  });
  assert.equal(rowOf(stale.probeRows, 'resident').value, '占位陈旧（端口不应答）', '陈占位 = stale');

  const absent = daemonOpsView(null, {
    descriptor_present: false,
    endpoint: null,
    pid: null,
    health: 'absent',
  });
  const absentRow = rowOf(absent.probeRows, 'resident');
  assert.equal(absentRow.value, '无占位（未探测到常驻）', '无占位 = absent');
  assert.ok(absentRow.note.includes('读不懂即无常驻'), '探测面不猜');
  assert.equal(residentProbeLabel('Mystery'), 'Mystery', '未知判定原样透出');
  console.log('  -> PASS: 常驻探测三态如实（占位照报、陈旧不猜在跑）');
}

// ---------------------------------------------------------------------------
// 4. 启停指引注册表：status/start/stop 三条可复制；零装边界无安装行
// ---------------------------------------------------------------------------
{
  const keys = DAEMON_GUIDANCE_COMMANDS.map((row) => row.key);
  assert.deepEqual(keys, ['status', 'start', 'stop'], '指引 = 看状态 + 启停三条');
  for (const row of DAEMON_GUIDANCE_COMMANDS) {
    assert.ok(row.command.startsWith(DAEMON_SERVICE_SCRIPT), `${row.key} 命令同源服务脚本入口`);
    assert.ok(row.command.endsWith(` ${row.key}`), `${row.key} 命令动作键与注册表键一致`);
    assert.ok(row.command.includes('apeireth-service.ps1'), `${row.key} 命令可直接粘贴执行`);
    assert.ok(row.label.length > 0 && row.desc.length > 0, `${row.key} 有人话说明`);
  }
  // 零装边界：无安装/卸载行、无「install」动作（界面不代跑安装）。
  assert.ok(!keys.includes('install'), '注册表无安装行（零装边界）');
  assert.ok(!keys.includes('uninstall'), '注册表无卸载行（零装边界）');
  assert.ok(
    !DAEMON_GUIDANCE_COMMANDS.some((row) => row.command.includes(' install')),
    '命令串不带安装动作',
  );
  assert.ok(DAEMON_ZERO_INSTALL_NOTE.includes('不加安装按钮'), '零装边界如实声明');
  console.log('  -> PASS: 启停指引可复制（status/start/stop 三条，零装边界无安装行）');
}

// ---------------------------------------------------------------------------
// 5. 源码镜像：设置页数据区挂卡 + 桌面桥/监督面只读探针接线
// ---------------------------------------------------------------------------
{
  // 卡面（模板 data 区）+ 读数脚本（daemon 运维面块）两半分开钉。
  const scriptStart = settingsSrc.indexOf('// ---- daemon 运维面（D12）');
  const scriptEnd = settingsSrc.indexOf('async function copyOpsCommand');
  assert.ok(scriptStart >= 0 && scriptEnd > scriptStart, 'daemon 读数脚本块在案');
  const daemonScript = settingsSrc.slice(scriptStart, scriptEnd);

  assert.ok(dataSection.includes('daemon 状态（只读）'), '「数据与本地缓存」区挂 daemon 状态卡');
  assert.ok(dataSection.includes('refreshDaemonStatus()'), '刷新按钮走同一条只读取数缝');
  assert.ok(dataSection.includes('DAEMON_GUIDANCE_COMMANDS'), '启停指引由注册表渲染（不手抄第二份）');
  assert.ok(dataSection.includes('copyOpsCommand(row)'), '命令可复制');
  assert.ok(dataSection.includes('{row.command}'), '命令文本原样上屏');
  assert.ok(daemonScript.includes('getBackendStatus()'), '读数脚本取监督面状态（侧车连接态/端点/版本）');
  assert.ok(daemonScript.includes('getResidentProbe()'), '读数脚本取常驻探针报告');
  assert.ok(daemonScript.includes('daemonOpsView('), '展示映射走纯函数（不手搓状态）');

  assert.ok(desktopBridgeSrc.includes('get_resident_probe'), '桌面桥薄包装对准 IPC 命令名');
  assert.ok(desktopBridgeSrc.includes('ResidentProbeReport'), '探针报告类型镜像 Rust DTO');
  assert.ok(
    supervisorRust.includes('pub async fn probe_resident_service'),
    '监督面探针读面在案（复用既有探测缝）',
  );
  assert.ok(supervisorRust.includes('pub struct ResidentProbeReport'), '探针报告 DTO 在案');
  assert.ok(supervisorRust.includes('pub async fn get_resident_probe()'), 'IPC 只读命令在案');
  assert.ok(supervisorLibRust.includes('backend_supervisor::get_resident_probe'), '命令已注册进 handler');
  console.log('  -> PASS: 源码镜像——数据区上屏 + 桌面桥/监督面只读探针接线');
}

// ---------------------------------------------------------------------------
// 6. 只读纪律 + 即效缝零回归：新面无写面、无保存按钮、不代跑生命周期
// ---------------------------------------------------------------------------
{
  const scriptStart = settingsSrc.indexOf('// ---- daemon 运维面（D12）');
  const scriptEnd = settingsSrc.indexOf('async function copyOpsCommand');
  const newSurface = settingsSrc.slice(scriptStart, scriptEnd) + dataSection;
  for (const forbidden of ['onSave', 'handleSaveSettings', '保存设置', 'applyAdminConfig']) {
    assert.ok(!newSurface.includes(forbidden), `运维卡零写面（不许出现 ${forbidden}）`);
  }
  for (const forbidden of ['start_backend', 'stop_backend', 'restart_backend', 'startBackend', 'stopBackend']) {
    assert.ok(!newSurface.includes(forbidden), `运维卡不代跑生命周期（不许出现 ${forbidden}）`);
  }
  // 既有即效缝（组提交/密钥动作共用的 apply 缝）原样保留 = 零回归。
  assert.ok(settingsSrc.includes('async function handleSaveSettings'), '批量 apply 缝保留（零回归）');
  assert.ok(settingsSrc.includes('runLiveApply('), '三级即效外壳保留（零回归）');
  console.log('  -> PASS: 只读纪律（零写面/不代跑生命周期）+ 即效缝零回归');
}

console.log('daemon ops: all assertions passed');
