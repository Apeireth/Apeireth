# 设置面三级即效语义 · 控件→生效时机对照表 + 测试证据

范围：`frontend/companion-desktop` 设置面（SettingsView.svelte / App.svelte /
ThemeSettingsPanel.svelte + 纯函数件 `src/lib/settings-live-apply.ts`）。
每类控件只有一条生效时机；页面「保存」按钮（页头按钮 + 底部保存栏）已删除。

## 三级即效语义

1. **开关 / 滑杆 / 预设**：点（或松手）即生效——立即走 apply 路径（await 推送），
   行内 pending（"正在应用到运行时…"），失败回填该项旧值 + 错误横幅。
2. **文本输入**：失焦或回车提交（服务商组整组提交，避免半截配置）；字段旁 ✓
   瞬间确认，未提交显「未保存」微提示；失焦提交失败整组回填旧值 + 横幅。
3. **危险动作**（删数据 / 清记忆 / 断开连接类）：即点 + 二次确认弹层，确认后即效。

apply 路径 = `onSave`（App 侧：配置持久化 + env 注入 + 侧车重启/热应用；
推送失败 App 先回退本地配置与文档主题再抛错 → 控件层回填 + 横幅）。
侧车重启/热应用期间只有行内 pending 态，全程不弹模态。

## 控件→生效时机对照表

| 页面 / 区块 | 控件 | 生效时机 | apply 路径 | 失败行为 |
| --- | --- | --- | --- | --- |
| 外观与主题 | 主题卡 | 1 点击即效 | onSave（App 侧回退 config + 文档主题） | 回退 + 横幅 |
| 外观与主题 | UI 配色卡 | 1 点击即效 | 同上 | 回退 + 横幅 |
| 外观与主题 | 上传背景图 | 1 选图即效 | 同上 | 横幅（图留本机库，可重启用） |
| 外观与主题 | 恢复主题默认 | 1 点击即效 | 同上 | 回退 + 横幅 |
| 外观与主题 | 清除已传图片 | 3 点击→二次确认→即效 | 同上 | 横幅 |
| 模型与提供商 | 协议选项卡 | 2 整组失焦/回车提交 | `configWithProviderGroup`（端点+模型+密钥+协议头一次落位） | 整组回填 + 横幅 |
| 模型与提供商 | 服务商预设 chip | 2 整组失焦/回车提交 | 同上 | 同上 |
| 模型与提供商 | API 端点 / 活动模型 ID / API Key / anthropic-version | 2 失焦或回车整组提交（✓ / 「未保存」） | 组提交 + 密钥入钥匙串 + 网关热应用回显 | 整组回填 + 横幅 |
| 模型与提供商 | 模型选择器 / 推荐模型 chip / 目录 chip | 2 点选即标未提交，整组失焦/回车提交 | 同上 | 同上 |
| 模型与提供商 | 测试连接并拉取模型 | 动作按钮（探测，不提交） | `testProviderConnection` | 结果框 |
| 模型与提供商 | 网关服务地址 | 2 失焦或回车提交 | `configWithGatewayUrl` | 回填旧值 + 横幅 |
| 模型与提供商 | 配置 / 更换 Key | 动作按钮（弹窗，弹窗内「写入并应用」为动作） | 钥匙串写入 + 组提交 | 弹窗内错误 |
| 模型与提供商 | 删除已存密钥 | 3 二次确认→即效 | 钥匙串删除 + 组清空提交 | 横幅 |
| 伙伴人设 | 名称 / 固定模型 / 人设文本 | 2 失焦或回车整组提交 | `configWithPersonas` | 整组回填 + 横幅 |
| 伙伴人设 | 设为当前 | 1 点击即效 | `submitPersonas` | 回填 + 横幅 |
| 伙伴人设 | 新增伙伴 | 1 点击即效 | `submitPersonas` | 回填 + 横幅 |
| 伙伴人设 | 删除该伙伴 | 3 二次确认→即效 | 删人设 + `submitPersonas` | 回填 + 横幅 |
| 记忆与认知 | 能力开关群（记忆流 / 认知增强 / 社区账本） | 1 拨动即效 | `applyCapabilityNow` → `configWithCapabilities` | 该项回填 + 横幅 |
| 记忆与认知 | 检索温度滑杆 | 1 松手（change）即效 | `applyKnobNow` | 该键回填 + 横幅 |
| 记忆与认知 | 应用推荐配置（预设） | 1 点击即效 | `applyKnobNow`（6 键） | 回填 + 横幅 |
| 性格与记忆 | 四滑杆（遗忘衰减 / 好奇 / 语气 / 整合节奏） | 1 松手即效 | `applyKnobNow` | 该键回填 + 横幅 |
| 性格与记忆 | 省心 / 均衡 / 深度记忆（预设） | 1 点击即效 | `applyKnobNow`（4 键） | 回填 + 横幅 |
| 性格与记忆 | 恢复基线（预设） | 1 点击即效 | `applyKnobNow`（4 键 + 自学习关） | 回填 + 横幅 |
| 性格与记忆 | 从使用中学习 | 1 拨动即效 | `handleCapabilityToggle` | 回填 + 横幅 |
| 性格与记忆 | 学习日志「撤销」 | 1 点击即效 | `applyKnobNow`（该键） | 回填 + 横幅 |
| 性格与记忆 | 学习日志「刷新」 | 动作按钮 | `readTuningLog` | — |
| 决策与治理 | 认知深度档位（轻量/平衡/深度/自定义） | 1 点选即效（预设） | `applyKnobNow(['judge','council'])` | 回填 + 横幅 |
| 决策与治理 | 评审 / 议会开关 | 1 拨动即效 | `handleCapabilityToggle` | 回填 + 横幅 |
| 决策与治理 | 顾问数量滑杆 | 1 松手即效 | `applyKnobNow` | 回填 + 横幅 |
| 决策与治理 | 单顾问超时 (ms) | 2 失焦或回车提交 | `submitCapabilityText` | 该键回填 + 横幅 |
| 决策与治理 | 三洋葱治理层 / 子代理 worktree 隔离 | 1 拨动即效 | `handleCapabilityToggle` | 回填 + 横幅 |
| 工具与安全 | 全局权限预设 | 1 点选即效（会话级本地记录） | localStorage | — |
| 工具与安全 | 工具开关（Shell / 网络读取 / 本地只读） | 1 拨动即效 | `handleCapabilityToggle` | 回填 + 横幅 |
| 工具与安全 | AppContainer 沙箱开关 | 1 拨动即效 | `handleCapabilityToggle` | 回填 + 横幅 |
| 运行时与诊断 | 立即执行深度诊断 | 动作按钮（探测） | `checkHealthDetailed` | — |
| 数据与存储 | 工作区路径（可直接编辑） | 2 失焦或回车提交 | `setWorkspaceDir` | 回填旧值 + 横幅 |
| 数据与存储 | 更改（工作区选择器） | 动作按钮；弹窗「选择」= 即效 | `setWorkspaceDir` | 弹窗内错误 |
| 数据与存储 | 清空本地会话数据 | 3 二次确认→即效 | `onClearLocalData` | — |
| 开发者选项 | 思考模式开关 | 1 拨动即效 | `handleCapabilityToggle` | 回填 + 横幅 |
| 开发者选项 | 生效模型过滤器 / reasoning 标签 | 2 失焦或回车整组提交 | `submitCapabilityText` | 该键回填 + 横幅 |
| 开发者选项 | 复制配置 JSON | 动作按钮 | clipboard | — |
| 全部页面 | ~~「保存设置」按钮（页头 + 底部保存栏）~~ | 已删除 | — | — |

纯空白差异不算改动（空提交只清「未保存」标记，不打扰运行时）。

## 测试证据

`pnpm test` = 22/22 套件通过（既有 20 套件零回归 + 新增 2 套件）：

- 新增 `tests/settings-live-apply.mjs`（7 组断言，import 真实模块 + 源码镜像）：
  1. 开关即效接线（能力开关群 → handleCapabilityToggle → applyCapabilityNow → onSave；pending / 失败回填 / 横幅字面量在案）；
  2. 滑杆即效（六根滑杆 onchange → applyKnobNow；滑杆值直映注入字段 tune_memory_fade 等）；
  3. 文本失焦/回车提交（isTextCommitKey / focusLeavesGroup / textCommitFlag 纯函数 + 三个文本组 focusout/keydown 接线 + ✓/「未保存」旁注）；
  4. 失败回填（runLiveApply 回滚 + 序号守卫；组/网关/人设/工作区各自回填旧值；App 侧 `config = previousCfg` 回退 + 抛错）；
  5. 组提交（configWithProviderGroup 四件一次落位、叠加不覆盖、镜像缓冲随协议走、trim 快照 = 空提交跳过）；
  6. 危险动作二次确认（登记表恰为 4 件 + 按钮只 requestDanger + 确认后 runDangerAction 即效 + 主题面板清图走确认）；
  7. 预设/恢复基线即效（三档 + 恢复基线 + 推荐配置 + 认知档位点击即效；预设确认框残留清零）。
- 新增 `tests/settings-save-buttons-removed.mjs`（4 组断言）：
  1. 页头「保存设置」按钮已删除（全文无「保存设置」/「已保存！」）；
  2. 底部批量保存栏（含样式）已删除；
  3. 花括号感知按钮扫描：任何 `<button>` 不再触发批量 apply 缝（该缝保留给服务商组提交/密钥动作）；
  4. 动作类按钮保留（测试连接 / 深度诊断 / 推荐配置 / 恢复基线 / 危险动作 / 刷新 / 工作区选择器）。

既有 20 套件输出与改动前逐项一致（含 capability-toggle-apply.mjs 的 apply 缝契约、
self-tuning-mapping.mjs 的滑杆取值域/预设表、recommended-preset.mjs 的预设映射）。

## 校验状态

| 校验 | 结果 |
| --- | --- |
| `pnpm check`（frontend/companion-desktop） | 0 errors / 0 warnings |
| `pnpm test` | 22/22 suites passed |
| `cargo test --locked`（src-tauri） | 全绿（含 supervisor_lifecycle 8/8） |
| `cargo test --workspace --locked` | 全绿（复跑；首轮 1 个与本改动无关的并发压测 flake，见备注） |
| `cargo clippy --workspace --all-targets --locked` | 0 warnings / 0 errors |
| `cargo fmt --all --check` | 干净（exit 0） |
| `scripts/check-neutral-terms.ps1` | 零命中（OK） |

备注：`cargo test --workspace --locked` 首轮在 `apeireth-core`
`storage_atomic::tests::file_lock_serializes_concurrent_writers`（文件锁并发写
压测）丢 1 次更新（99/100）；该测试单独重跑 3/3 通过、整批复跑全绿（161/161），
属既有并发压测在 Windows 并行负载下的偶发（本次改动只动前端 TS/Svelte，未触任何
Rust 路径）。
