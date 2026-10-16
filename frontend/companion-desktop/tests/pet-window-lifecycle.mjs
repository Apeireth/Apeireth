// 桌宠常驻窗生命周期与互斥礼让（源码级镜像校验，同 quick-window-close.mjs 约定）。
//
// 契约：
//   1. 用户关闭 = 真正销毁（hide 清影 + destroy），绝不存在「隐藏中的透明窗」
//      （幽灵窗教训，与快捷窗同一条纪律）；
//   2. 互斥礼让：主窗在场 → 桌宠隐身；主窗收起 → 桌宠接管常驻（纯决策表在
//      src-tauri pet_leeway 单测）；
//   3. 右键卷宗入口：open_main_target 唤起主窗并 emit shell-target，主窗按
//      shell-nav 目标表改道（App.svelte 监听）；
//   4. 0 装：桌宠窗只消费真实事件与 presence 帧，不生成内容、不编造状态
//      （无 Math.random、SIM 标注在场）；
//   5. 形象层经 AvatarHost 插拔，未知 id 回落微光简模（绝不空窗）。
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const testsDir = dirname(fileURLToPath(import.meta.url));
const srcDir = join(testsDir, '..', 'src');
const rustLib = join(testsDir, '..', 'src-tauri', 'src', 'lib.rs');

console.log('--- Starting Pet Window Lifecycle Check ---');

const rustSrc = readFileSync(rustLib, 'utf8');

// ---------------------------------------------------------------------------
// 1. 开关决策表 + 销毁纪律（与快捷窗同一条幽灵窗教训）
// ---------------------------------------------------------------------------
{
  for (const arm of [
    '(true, true) => PetWindowAction::Close',
    '(true, false) => PetWindowAction::Show',
    '(false, _) => PetWindowAction::Create',
  ]) {
    assert.ok(rustSrc.includes(arm), `pet_window_action 决策表必须含 ${arm}`);
  }

  const closeRegion = /fn close_pet_window[\s\S]*?\n\}/.exec(rustSrc);
  assert.ok(closeRegion, 'Rust 必须提供 close_pet_window 外部处决命令');
  assert.ok(
    closeRegion[0].includes('window.hide()') && closeRegion[0].includes('window.destroy()'),
    'close_pet_window 必须先隐身后销毁',
  );
  const cHide = closeRegion[0].indexOf('window.hide()');
  const cDestroy = closeRegion[0].indexOf('window.destroy()');
  assert.ok(cHide < cDestroy, 'hide 仅允许作销毁前清影（必须先于 destroy）');

  const toggleRegion = /fn toggle_pet_window[\s\S]*?\n\}/.exec(rustSrc);
  assert.ok(toggleRegion, 'src-tauri 必须有 toggle_pet_window');
  assert.ok(toggleRegion[0].includes('window.destroy()'), '托盘关闭桌宠必须销毁窗口');
  assert.ok(toggleRegion[0].includes('build_pet_window(&app, true)'), '不存在时必须新开');
  console.log('  -> PASS: 桌宠开关 = 销毁关闭 / 唤起 / 新开，无隐藏残留态');
}

// ---------------------------------------------------------------------------
// 2. 互斥礼让：主窗在场 → 隐身；主窗收起 → 接管
// ---------------------------------------------------------------------------
{
  const leewayRegion = /fn pet_leeway[\s\S]*?\n\}/.exec(rustSrc);
  assert.ok(leewayRegion, 'src-tauri 必须有 pet_leeway 纯决策');
  assert.ok(leewayRegion[0].includes('PetLeeway::Hide'), '主窗在场 → 桌宠隐身');
  assert.ok(leewayRegion[0].includes('PetLeeway::Show'), '主窗收起 → 桌宠接管');
  assert.ok(
    rustSrc.includes('apply_pet_leeway(window.app_handle(), false)'),
    '主窗关闭（进托盘）时必须让桌宠接管常驻',
  );
  assert.ok(
    rustSrc.includes('apply_pet_leeway(window.app_handle(), true)'),
    '主窗回到前台时必须让桌宠隐身礼让',
  );
  assert.ok(/CloseRequested[\s\S]{0,200}apply_pet_leeway/.test(rustSrc), '礼让必须挂在主窗关闭事件上');
  console.log('  -> PASS: 互斥礼让双向接线（接管 / 隐身）');
}

// ---------------------------------------------------------------------------
// 3. 右键卷宗入口链：pet 窗菜单 → open_main_target → shell-target → 壳层改道
// ---------------------------------------------------------------------------
{
  const openRegion = /fn open_main_target[\s\S]*?\n\}/.exec(rustSrc);
  assert.ok(openRegion, 'Rust 必须提供 open_main_target');
  assert.ok(openRegion[0].includes('window.emit("shell-target", target)'), '必须 emit shell-target');
  assert.ok(openRegion[0].includes('window.show()'), '必须唤起主窗');

  const appSrc = readFileSync(join(srcDir, 'App.svelte'), 'utf8');
  assert.ok(appSrc.includes("listen<string>('shell-target'"), '主窗必须监听 shell-target');
  assert.ok(appSrc.includes('resolveLegacyDrawer(event.payload'), '导航必须走 shell-nav 目标表');

  const petSrc = readFileSync(join(srcDir, 'features/pet/PetWindowView.svelte'), 'utf8');
  for (const target of ['memory', 'diary', 'governance', 'logs']) {
    assert.ok(petSrc.includes(`'${target}'`), `卷宗入口必须含 ${target}`);
  }
  assert.ok(petSrc.includes('open_main_target'), '卷宗入口必须走外部命令唤起主窗');
  console.log('  -> PASS: 右键卷宗链贯通（菜单 → 主窗 → 目标表改道）');
}

// ---------------------------------------------------------------------------
// 4. 0 装：真实信号驱动，不生成内容不编造状态
// ---------------------------------------------------------------------------
{
  const petSrc = readFileSync(join(srcDir, 'features/pet/PetWindowView.svelte'), 'utf8');
  assert.ok(!petSrc.includes('Math.random'), '桌宠不许有随机造作（节奏由真实事件驱动）');
  assert.ok(petSrc.includes('subscribePresence'), '必须消费 presence_state 真实帧');
  assert.ok(petSrc.includes('subscribeCompanionEvents'), '必须消费网关生命周期事件');
  assert.ok(petSrc.includes('frame.simulated'), 'SIM 标注必须外显');
  assert.ok(petSrc.includes('isGlanceActive'), '抬头必须由 proactive 引擎裁决');

  const contractSrc = readFileSync(join(srcDir, 'lib/pet/pet-contract.ts'), 'utf8');
  assert.ok(contractSrc.includes('PET_LIGHT_BY_STANCE'), '光参数表必须在契约层');
  assert.ok(contractSrc.includes('ember_hud_driver.rs'), '光参数镜像来源必须注明');
  console.log('  -> PASS: 桌宠窗零伪造（真实事件 + SIM 外显 + 光参数同源）');
}

// ---------------------------------------------------------------------------
// 5. 形象层插拔：宿主回落 + 入口路由
// ---------------------------------------------------------------------------
{
  const hostSrc = readFileSync(join(srcDir, 'features/pet/AvatarHost.svelte'), 'utf8');
  assert.ok(hostSrc.includes('FALLBACK_AVATAR_LAYER_ID'), '装载失败必须回落微光简模');
  assert.ok(hostSrc.includes('registerBuiltInAvatarLayers'), '内置层必须注册');

  const mainTs = readFileSync(join(srcDir, 'main.ts'), 'utf8');
  assert.ok(mainTs.includes("windowRole === 'pet'"), '入口必须路由 ?window=pet 到桌宠窗');
  assert.ok(mainTs.includes('PetWindowView'), '桌宠窗视图必须挂载');
  console.log('  -> PASS: 形象层可插拔（回落兜底）+ 入口路由接线');
}

console.log('--- All Pet Window Lifecycle Checks PASSED! ---');
