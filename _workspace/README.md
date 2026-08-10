# _workspace/ — 施工人临时工作副本专用

> **R119-1 Mavis 留 (2026-08-10)**: 给所有施工人(sub-agent / Mavis / 未来接手者)放临时工作副本的目录。

## 规则(简单,容易遵守)

1. **可以用**:放临时脚本、临时 log、临时 markdown、临时 backup、worktree 工具等
2. **不要 commit 内容**:`_workspace/*` 全部在 .gitignore 里(除本 README.md 和 .gitkeep)
3. **用完即删**:本目录是工作副本,不是设计 / 代码 / 测试的一部分,**完工后清掉**
4. **不参与 build**:任何内容都不会被 cargo / pytest / CI 看到

## 子目录约定

| 目录 | 用途 | 进 git? |
|---|---|:---:|
| `_workspace/README.md` | 本说明 | ✅ 跟踪 |
| `_workspace/.gitkeep` | 空标记,保 _workspace/ 存在 | ✅ 跟踪 |
| `_workspace/_archive/` | 临时 stash(原 .tmp-* / _v1306_backup 等) | ❌ .gitignore |
| `_workspace/<your-name>/` | 你自己创建的临时目录(例: `_workspace/Mavis-R119/`) | ❌ .gitignore |

## 例子

```powershell
# ✅ 临时 Python 脚本
Write-File "_workspace/scratch.py" "print('debug')"
# .gitignore 自动忽略,跑完删

# ✅ 临时 log
cargo test 2>&1 | Out-File "_workspace/cargo-test.log"
# .gitignore 自动忽略,看完删

# ✅ 临时 backup before modify
Move-Item "Cargo.toml" "_workspace/_archive/Cargo.toml.bak"
# _archive/ 在 .gitignore,git 看不到
```

## 跟 `.tmp-*` 旧约定的区别

**R23 之前**(per `docs/stage4/docs-maintenance-sop-2026-08-05.md`):
- 临时文件直接放根目录 / reports/,用 `.tmp-*` 前缀
- .gitignore 模式 `.tmp-*.txt` / `.tmp-*.sh` 等过滤
- 累积 90+ 个文件(R20-R21 期间的工作副本),散在根目录 + reports/

**R119 之后**(本 README):
- 临时文件统一放 `_workspace/<sub>/`
- .gitignore 规则:`_workspace/*` 忽略 + `_workspace/README.md` / `.gitkeep` 例外
- 根目录 / reports/ 0 临时文件,**整齐**

## 关联

- 父目录: `.openclaw/workspace/promethean/Apeireth-rust/`
- .gitignore 规则:见根目录 `.gitignore` §"R119 Mavis: _workspace/ 目录约定"
- 顶层规范:见 `docs/conventions/02-path.md`(R119-3 落地后)
