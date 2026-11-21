# Cosign 公钥评审与轮换

本页登记 Apeireth 发布物 `cosign sign-blob` 的公开验证密钥。公钥不是秘密，
但它是发布信任根：修改必须经过安全评审，不能由工作流直接推送或自动合并。

## 1. 当前状态

- 生产公钥以已经评审合并的 `docs/security/cosign.pub` 及本页登记为准。
- 缺少 `docs/security/cosign.pub` 时，签名和验签工作流必须 fail-closed；不得用
  本模板、测试公钥或临时 key 让发布门禁变绿。
- 私钥只允许进入批准的外部密钥保管面和 GitHub Actions secrets；不得进入仓库、
  artifact、日志、截图或评审文本。

## 2. 当前/候选公钥登记

以下带标记区域由 `scripts/update_cosign_key_docs.py` 更新。`NOT CONFIGURED` 表示待配置。

<!-- COSIGN_ACTIVE_PUBLIC_KEY_BEGIN -->
```text
NOT CONFIGURED — run the explicit publish-pubkey preparation job after external key provisioning.
```
<!-- COSIGN_ACTIVE_PUBLIC_KEY_END -->

<!-- COSIGN_ACTIVE_METADATA_BEGIN -->
| 项 | 值 |
|---|---|
| 登记日期（UTC） | `NOT CONFIGURED` |
| 算法 | `ECDSA P-256 / SubjectPublicKeyInfo` |
| Key ID（SHA-256 of DER SPKI） | `NOT CONFIGURED` |
| 状态 | `NOT CONFIGURED` |
<!-- COSIGN_ACTIVE_METADATA_END -->

## 3. 公钥评审清单

在默认分支手动启动 Cosign 工作流，设置 `skip_publish=false` 准备审查 artifact。
此模式独立于发行签验，支持首次登记和轮换。评审者在合并前逐项核对。

1. artifact 只包含 `docs/security/cosign.pub` 和本页，且没有私钥、密码或编码后的
   私钥材料；
2. 公钥是 ECDSA P-256 SubjectPublicKeyInfo，PEM 与本页逐字一致；
3. 独立重算 DER SPKI 的 SHA-256，结果与本页 Key ID 一致；
4. 用同一受控签名源签一个合成 blob，再用候选公钥验签，且错误公钥必须失败；
5. 登记日期是本次候选生成的真实 UTC 日期，PR 说明记录轮换原因、评审者和影响的
   release 范围；
6. 只有经评审并合并、且外部 secret 已指向同一密钥对后，候选公钥才成为 active。

## 4. 私钥与 CI 合同

- `COSIGN_KEY`：GitHub Actions 中保存的 base64 编码私钥材料；
- `COSIGN_PASSWORD`：该私钥的解密密码；
- `publish-pubkey` 从上述受控配置导出公钥，校验 P-256 结构，重算指纹并生成 7 天
  保留的审查 artifact；
- 工作流不上传私钥、不打印 secret、不创建提交，也不生成新的生产密钥。

真实 secret 的创建、托管、轮换和删除是仓库外操作。本仓只能验证配置是否存在、
导出的公开材料是否自洽，以及缺配置时是否阻断。

## 5. 轮换

1. 在批准的外部密钥保管面生成或导入新密钥；不要在普通 CI 运行中隐式 keygen。
2. 显式运行 `publish-pubkey` 准备候选 artifact。
3. 按 §3 评审，并在同一个 PR 中更新 `cosign.pub` 与本页。
4. 合并后再把签名 secret 切换到已评审公钥对应的私钥，随后执行合成签名/验签和
   正式 release dry-run。
5. 历史 release 的旧公钥和 fingerprint 必须保留在 Git 历史及对应 release 说明中；
   不得把轮换描述成对历史签名的追溯替换。

## 6. 撤销与用户验证

发现私钥泄露、错误签名或公钥不匹配时：立即停止发布，撤销外部 secret，发布安全
公告并登记旧 fingerprint、撤销时间和受影响 release。新密钥必须走完整轮换评审。

用户验证仓库发布物时，应从已评审提交取得 `docs/security/cosign.pub`，再运行：

```bash
cosign verify-blob --bundle <artifact>.sigstore.json \
  --key docs/security/cosign.pub <artifact>
```

验签失败、公钥缺失或 bundle 缺失都应视为阻断条件。
