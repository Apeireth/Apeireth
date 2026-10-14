// 星港 · 面安装器（I 批 2–5 总集成面）
//
// 契约：
//   1. 一处安装 —— 全部内置面应用在此下挂贡献点注册表（hello 契约件仍走
//      builtins.ts 的 installStarportBuiltins，双链并存：契约证明 + 功能面）；
//   2. 注册面纪律沿用注册表契约（id 唯一、禁注入字段、owner 生命周期）；
//   3. 面清单口径以 faces.ts 为单一真源（live/slice/not_assembled 三态），
//      本安装器只装「有工作切片」的面；未组装面不上假旋钮。
//
// 测试：纯件各面自带测试；本文件为装配面，由全量套件 + 源镜像钉接线。

import type {ApeirethConfig} from '../types';
import type {ContributionRegistry} from './contribution-registry';
import {createNotesApp} from './apps/notes-app.ts';
import {createTextEditorApp} from './apps/text-editor-app.ts';
import {createDocApp} from './apps/doc-app.ts';
import {createMailApp} from './apps/mail-app.ts';
import {createMemoryFlowApp} from './apps/memory-flow-app.ts';
import {createVoiceApp} from './apps/voice-app.ts';
import {createTerminalApp} from './apps/terminal-app.ts';
import {createPortableApp} from './apps/portable-app.ts';
import {createDbApp} from './apps/db-app.ts';
import {createFolderTilesApp} from './apps/folder-tiles-app.ts';
import {
  createBrowseApp,
  createChartApp,
  createOrganizeApp,
  createLifeflowApp,
  createTranslateApp,
  createEcosystemApp,
} from './apps/face-apps.ts';

export interface FaceInstallOptions {
  config: ApeirethConfig;
  registry: ContributionRegistry;
}

/** 把批 2–5 全部「有工作切片」的面装进注册表。返回注册数量。 */
export function installStarportFaces(options: FaceInstallOptions): number {
  const {config, registry} = options;
  const apps = [
    createNotesApp(),
    createTextEditorApp(),
    createDocApp(),
    createMailApp(),
    createMemoryFlowApp({config}),
    createVoiceApp(),
    createTerminalApp(),
    createPortableApp(),
    createDbApp(),
    createFolderTilesApp(),
    createBrowseApp(),
    createChartApp(),
    createOrganizeApp(),
    createLifeflowApp(),
    createTranslateApp({config}),
    createEcosystemApp({registry}),
  ];
  for (const app of apps) registry.registerApp(app);
  return apps.length;
}
