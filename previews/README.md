# previews/ — UI 实拍验收图

> Status: 34 PNG tracked，**经 git-lfs 存储**（F-02 执行批，2026-10-18）；路径不变、零断链。

- 自 2026-10-18 起本目录 PNG 由 git-lfs 承载（`.gitattributes`：`previews/*.png filter=lfs diff=lfs merge=lfs -text`），
  git 对象面不再直接存二进制 blob；文件路径保持不变，`docs/design/frontend-handoff.md`、
  `docs/design/intro-animation.md` 等实拍验收引用不受影响。
- 克隆后若工作树图片为空/为指针文本，请执行 `git lfs pull` 取回对象。
- 34 件逐项 sha256（= LFS oid）见
  [../docs/04-internal/backend-enhancement/f-02-migration-manifest-2026-10-18.tsv](../docs/04-internal/backend-enhancement/f-02-migration-manifest-2026-10-18.tsv)。
