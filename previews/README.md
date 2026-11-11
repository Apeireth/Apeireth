# previews/ — 界面历史图与品牌标志

> Status: 34 PNG tracked，**经 git-lfs 存储**（F-02 执行批，2026-10-18）；路径不变、零断链。

- `apeireth-mark.svg`：2026-11-11 同步已确认的官网与设计原型标签标志，复用 32 × 32 画布上的双层菱形路径及原配色。README 使用此矢量标志；SVG 直接存 Git，不经过 LFS。
- 既有桌面与开场动画截图保留为设计历史，不能作为正在开发的新前端实机验收证据。

- 自 2026-10-18 起本目录 PNG 由 git-lfs 承载（`.gitattributes`：`previews/*.png filter=lfs diff=lfs merge=lfs -text`），
  git 对象面不再直接存二进制 blob；文件路径保持不变，`docs/design/frontend-handoff.md`、
  `docs/design/intro-animation.md` 等实拍验收引用不受影响。
- 克隆后若工作树图片为空/为指针文本，请执行 `git lfs pull` 取回对象。
- 34 件逐项 sha256（= LFS oid）见
  [f-02-migration-manifest-2026-10-18.tsv](../docs/04-internal/backend-enhancement/f-02-migration-manifest-2026-10-18.tsv)。
