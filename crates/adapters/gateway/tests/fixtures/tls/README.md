# TLS 测试夹具 (测试专用, 无真实信任价值)

安全加固批 B5 的服务面钉测材料。**私钥仅测试用途** —— 本目录任何材料不得、
也不需要被任何真实部署信任。

## 内容

| 文件 | 内容 |
| --- | --- |
| `ca-cert.pem` | 自签测试根 CA (CA:TRUE, KeyUsage=CertSign+DigitalSignature), RSA-2048 |
| `server-chain.pem` | 服务端证书链 (叶 + 根, 顺序: 叶在前): CN=localhost, SAN=DNS:localhost + IP:127.0.0.1, EKU=ServerAuth |
| `server-key.pem` | 服务端私钥 (PKCS#8, 与叶证书配对) |
| `mismatch-key.pem` | 一把无关私钥 (PKCS#8): 专测「证书私钥不配对拒加载」 |
| `ca-ecdsa-cert.pem` | 自签测试根 CA (ECDSA P-256): ECC 材料面的根锚 |
| `server-ecdsa-chain.pem` | ECDSA P-256 服务端证书链 (叶 + 根): SAN/EKU 同 RSA 面 |
| `server-ecdsa-key.pem` | ECDSA P-256 服务端私钥 (PKCS#8) |
| `ca-b-cert.pem` | 第二把自签根 CA (RSA-2048, 轮换测试专用) |
| `server-b-chain.pem` | 轮换后的服务端证书链 (叶 + 根), 与 A 面互不信任 |
| `server-b-key.pem` | 轮换后的服务端私钥 (PKCS#8) |

轮换语义口径 (B5-TEST ③): 证书面 A/B 是两套互不信任的自签材料, 钉
「冷启动换证书: 重启后新证书生效、只信旧面的客户端被拒」。
**热重载未接线** —— 材料只在启动加载一次, 运行中替换本目录文件不生效,
换证书 = 重启。

## 生成口径 (可复核)

RSA 面一次性生成于 2026-10-20 (RSA-2048 / SHA-256), ECDSA 面与轮换 B 面
补充于 2026-10-21 (ECDSA P-256 / SHA-256), 有效期均 10 年。
工具 = 本机运行时自带的 X509 证书请求 API。复现口径:
`CertificateRequest` 自签根 CA → 以 CA 签发叶证书 → 导出 PEM
(`ExportCertificatePem` / `ExportPkcs8PrivateKeyPem`), 链文件 = 叶 PEM + 根 PEM 拼接。

若夹具过期 (测试报证书时间校验失败), 按上述口径重新生成即可;
测试只钉「真加载/真握手/真拒绝」的行为, 不钉任何具体字节。

## 提交注意 (批 B5 报备)

仓库根 `.gitignore` 有 `*.pem` 通配忽略 (第 88 行), 本目录 `.pem` 夹具
默认不会被 `git add` 收进。提交时二选一: `git add -f` 本目录全部 `.pem`, 或在
`.gitignore` 加一行路径豁免 (`!crates/adapters/gateway/tests/fixtures/tls/*.pem`)。
secret-scan 若点名本目录测试私钥, 按测试夹具放行 (上面已声明仅测试用途、
零真实信任价值)。
