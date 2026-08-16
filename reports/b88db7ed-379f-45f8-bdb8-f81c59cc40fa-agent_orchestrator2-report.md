# 自检报告：脚本目录就绪度
任务ID: b88db7ed-379f-45f8-bdb8-f81c59cc40fa | 角色: agent_orchestrator2 | 结论: ⚠️

## 一、目录清单
| 位置 | 数量 | 用途 |
|---|---|---|
| `_scripts/` | 2 | 仅占位桩文件（`gen_modules.py`=`print('OK')`、`parse_writer.py`=`x=1`），无实际功能 |
| `scripts/` 顶层 `_` 前缀 | 145 | 一次性修复/补丁脚本（_fix*/_dump*/_add*），历史产物，不参与流程 |
| `scripts/` 顶层正式 | ~8 | build-all-packages.sh、release-1.0-checklist.sh、save_handover.py、stop-all-agents.ps1、verify-baseline.ps1 等；另有 patch_*/fix_env_* 一次性脚本混入 |
| `scripts/audit/` | 3 | cargo audit/deny、promise 审计 |
| `scripts/install/` | 6 | deb/rpm/brew/scoop/tarball 安装 + uninstall-all |
| `scripts/release/` | 16 | cosign 签名/验签、tag、git-push、GitHub Pages 部署（sh/ps1 成对） |
| `scripts/uninstall/`、`scripts/upgrade/` | 1、5 | 卸载与版本回滚 |

合计 scripts/ 198 文件。

## 二、抽查结果（引用路径/命令）
1. **有效引用**：`Cargo.toml`、`docs/security/cosign-keys.md`、`docs/pages-source/` 均存在 ✅
2. **缺失路径**（脚本引用但仓库中不存在）：
   - `docs/security/cosign.pub` — cosign-sign-all.sh / cosign-verify.sh 默认读取；脚本自带生成指引（cosign generate-key-pair 后提交），属**前置条件未满足**，运行时会报错并提示 ⚠️
   - `docs/i18n/zh-CN/README.md`、`docs/zh-CN/README.md`、`docs/stage4/8-locked-*`、`docs/stage4/v09021-*` — install/uninstall 脚本引用，抽查 install-deb.sh 仅为注释性蓝图引用，不影响执行 ✅（轻微）
   - `target/rpm` — install-rpm.sh 引用，为 rpmbuild 运行时产物，属正常 ⚠️→✅
3. **本机缺失外部命令**：cosign、gh、jq、yq、dpkg、rpmbuild（cargo/gpg 可用）。release 签名流程在本机无法直接执行；dpkg/rpmbuild 为 Linux 工具链，Windows 本机缺失属预期 ⚠️
4. `verify-baseline.ps1`、`stop-all-agents.ps1` 引用对象均有效 ✅

## 三、结论与建议
**⚠️ 基本就绪，存在 3 项待办**：
1. release 前需执行 `cosign generate-key-pair` 并提交 `docs/security/cosign.pub`（否则签名/验签脚本必失败）
2. release 依赖 cosign/gh/jq 未安装，需在发布环境预装
3. 卫生问题：`_scripts/` 2 个桩文件、`scripts/` 顶层 145 个 `_` 前缀一次性脚本 + patch_*/fix_env_* 混入，建议归档清理（不阻塞功能）

自检记录留存于本报告，可作为漂移检测依据。
