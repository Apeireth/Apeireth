# v1.0 Legacy Archive (per R179 Stage 2)

> **0 装 PASS**: git history archive, NOT workspace member. **不要 cargo build / cargo test**.

84 v1.0 era crates already migrated to v2 13 submodules (see ../../ROADMAP.md §16).

## git access

git log --all -- crates/_archived/v1.0-legacy/
git diff HEAD~1 HEAD -- crates/_archived/v1.0-legacy/

## 禁止

- 不要 cargo build 这个目录
- 不要 git checkout 整个目录
- 不要修改这个目录下的任何文件 (git 拒绝)