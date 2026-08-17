# GitHub 加速插件调研与实战记录（gh-accel，2026-08-16）

## 一、调研：xiake.pro（虾壳）是什么

[xiake.pro](https://xiake.pro) 是 **GitHub 下载加速网站**——它自己不是代理，而是**聚合了大量 GitHub 加速镜像节点**，并提供「节点检测」：前端并发测各节点加载真实 GitHub 文件（vscode 图标 PNG），按延迟排序选最快，然后用户通过 `https://{节点}/https://{github链接}` 前缀式访问。

调研路径（实战，非二手资料）：
- 抓首页 HTML → meta 说明 + 结构（`static/js/script.js` 引用）
- 抓 `script.js` → 发现节点检测逻辑 + **节点池 API**：`https://xiake.pro/static/node.json`
- 抓 node.json → 59 个节点，字段：`{url, server, ip, location, latency, speed}`（latency 是**站侧服务器视角**的基准）

## 二、节点池 API 格式

```json
{ "code": 200, "msg": "success",
  "data": [ {"url": "https://gh.xx9527.cn", "server": "cloudflare",
             "ip": "104.21.95.182", "location": "  ", "latency": 186, "speed": 3.29}, ... ],
  "total": 59, "update_time": "2025-03-05 14:17:02" }
```

注意：`update_time` 陈旧（2025-03），池内有重复 URL（如 mirrors.chenby.cn ×2），节点随时增减——**每次使用必须现拉现测，不缓存**。

## 三、实战验证（本机两次完整运行，gh_accel_demo）

| 节点 | 站侧延迟 | 实测第1跑 | 实测第2跑 |
|---|---|---|---|
| github.linxi.info | **17ms**（站侧最低） | 超时 ✗ | 超时 ✗ |
| g.blfrp.cn | 93ms | 200 ✓ | 884ms ✓ 当选 |
| gh.nxnow.top | 142ms | 830ms ✓ | 928ms ✓ |
| ghproxy.cn | 178ms | 780ms ✓（但内容=HTML 包装页） | 869ms ✗ 内容验证剔除 |
| 其余 8 节点 | — | 超时/失败 | 超时/失败 |

**教训 1（站侧延迟不可信）**：站标 17ms 的 linxi.info 本机必超时；10 个低延迟节点本机仅 3 个可用。服务器视角的延迟与本机网络路径毫无关系——**必须本机实测**（主人需求：每次 ping 选最快）。

**教训 2（状态码不够，必须验内容）**：ghproxy.cn 返回 200 但 body 是 HTML 包装页（首行 `<!-- 31142001856 -->`），非真实文件。光看 2xx 会选到「假可用」节点。最终实现：探测测试文件（vscode code.png）要求 **2xx + PNG 魔数（\x89PNG）** 才算可用。

**教训 3（免费节点不稳定）**：节点随时死；两次运行结果不同（ghproxy.cn 第一跑过第二跑被剔除）。工具输出全程如实标注，且每次调用重新实测。

**终验（第 2 跑）**：最快节点 g.blfrp.cn 实测抓取 `github.com/octocat/Hello-World/archive/refs/heads/master.zip` → HTTP 200 + 351 bytes + zip 魔数 PK → **真实 GitHub archive，加速链路真实可用 ✅**。

## 四、设计决策

| 决策 | 理由 |
|---|---|
| 测 HTTP 整链路耗时，非 ICMP ping | 代理/CF 常禁 ICMP；整链路（DNS+TCP+TLS+下载）才是用户体感 |
| 并发探测 buffer_unordered(4)，单节点 6s 超时 | 对齐 Crawl v2 调研结论（并发+超时护栏）；12 节点 ≈ 8.5s |
| 探测顺序 = 站侧延迟升序（取前 limit=12） | 站侧延迟只作排序线索，不作结论 |
| 去重按 url，校验 code==200 与 http(s) 前缀 | node.json 有重复与脏数据 |
| 工具只探测+给命令，不执行不改环境 | 执行是 ShellExec 的职责（高危+审批）；本工具 Low 风险 |
| 插件形态 github-accel（非基础工具） | 生态位：基础 9 工具 = 本体；加速 = 可卸载扩展（生态单元） |

## 五、0 假装清单

- 节点全为第三方免费服务，结果只代表「本次实测时刻」，随时可能失效
- 探测只覆盖 raw.githubusercontent.com 测试文件；部分节点（如 gitproxy.click 404）仅支持 github.com 路径——结果表如实保留 http_status，AI 可自行判断
- 未实现：代理链/CONNECT 隧道、多源池合并（gh-proxy.com 等其它池）、节点健康历史（后续可接 memory 沉淀）
