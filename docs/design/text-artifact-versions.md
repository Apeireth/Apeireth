# C2 文本成果与版本契约

[Document-Meta]
Document: docs/design/text-artifact-versions.md
Class: SPEC
Version: 1
Last-Modified: 2026-11-18
Status: 施工契约；实现与实测结果以进度页登记
Author: desktop-collaboration-line

## 目标与施工基线

用户已批准从协作中的文本方案稿开始C2：先落实成果/版本的前后端契约，再接可编辑文本工作面。中央协作对话为主线，文稿随工作展开。人和agent使用同一权威内核；文本内容模型仅为首工作面，不作为未来表格、画布或版式的统一格式。

后端独立工作树`C:\Apeireth\wt\c2-artifact-versions-20261113`，基线`b6b98ab07795a1cb72747027418ccc34b4ecfe4a`；私有前端`C:\Apeireth\wt\ui-desktop-private`，基线`568e6335992e7a22b5e8de413e0bd300c220f151`（N10已获用户通过）。2026-11-13 本批为本地施工口径，不推送/合并/发布；2026-11-18 复核以 cherry-pick 方式进入当前主干合并评审，最新门禁与边界见进度页。不改LOCKED、workspace版本和既存网站/S1文档脏改。当前主体分工、验证和还差什么见进度页。

## 首批边界与授权

- 新增独立受管Artifact工作副本，原导入Document/S2选区/N9和N10任务保持只读原义。不得直接改旧Document而破坏历史快照验证。
- 首批production明确`single_operator`：默认域、未激活用户表的本机操作员工作区。现有ToolExecutionContext没有HTTP用户身份链；不得从进程内session_users推造跨重启owner。多用户或非默认域未装配时两条出口均明确不可用，不回落读取默认库。用户表配置出现后每次调用失败关闭。
- owner/scope/author由可信服务/执行context构造，不接受HTTP或模型自报。work_id是当前本地项目的有界opaque归类引用，不是服务端项目授权凭证。conversation_session_id可选，仅归因，不授权，用户可在首条模型请求前保存空稿。
- same Arc从组合根显式提供受治理agent工具与HTTP服务端口，不用隐藏全局registry或私有宿主另建权威数据库。agent写必须RequireApproval并核真实context；HTTP保存是用户的明确内容决定，不伪造工具审批。
- 固定数据根SQLite，正文/资料不接受任意读写路径。复用现有seal策略整行保护；持久加密模式和当前配置不符拒开，不能明文回落。

## 不变量与保存真相

1. `store_id`是数据库内持久UUID，同数据重启不变，不同数据库不同；缓存以store_id与artifact_id归属，当前strong source/incarnation保护在途请求，切模型不改变成果身份。
2. Artifact UUID、immutable revision UUID与对话session/request/trace、全应用快照保存序号彼此独立。Revision不可修改，head仅通过原子提交推进。
3. 写带operation_id和期望store_id；已有成果再带expected_head。一个SQLite事务完成持久幂等检查、CAS、revision/head与receipt。不能仅靠进程Mutex。
4. 同operation同规范载荷返回原outcome，即使head后来已推进；同operation不同载荷409。CAS冲突也留原outcome，重试原operation仍回原冲突，不暗中变成功。坏输入/无权访问不产生内容写入。
5. 同正文no-op提交不新增revision，回执changed=false。恢复旧版以当前expected_head创建新revision、restored_from指旧版，历史不可倒转。拒绝AI候选不写权威正文。
6. 正文原样UTF-8，不正规化，不截断；空稿允许，NUL拒绝。256KiB content、256B title、history/list每页1..32。JSON转义体积预算独立，不能用body字节上限误拒合法正文。固定容量满则拒绝新入场，不淘汰已接纳receipt身份。
7. 本地未提交稿、权威已提交版本、未确认写操作分层。发送前冻结operation/payload并等待本地flush；不明写结果按同operation读回执，不换ID或自动补POST。ACK迟到只能确认发送时稿件，不能覆盖/清除之后输入。

## HTTP schema 1

以下写DTO均strict unknown/duplicate字段拒绝。UUID使用canonical lowercase字符串；所有读取（capabilities除外）以query `store_id`核对数据库身份，写body同样核验。响应敏感资料带no-store与nosniff。路由置于原有认证、跨源、角色及域闸内。

| 方法与路径 | 载荷/结果 |
| --- | --- |
| GET `/v1/artifacts/capabilities` | `{schema:1,store_id,scope:"single_operator",media_types,max_content_bytes,max_title_bytes,max_page_limit,max_artifacts,max_receipts}` |
| GET `/v1/artifacts?store_id&work_id&after?&limit?` | `{schema:1,store_id,work_id,artifacts:[Artifact],next_cursor:null|artifact_id}` |
| POST `/v1/artifacts` | `{store_id,operation_id,work_id,title,media_type,content,conversation_session_id:null|uuid}` → Receipt；create没有expected_head |
| GET `/v1/artifacts/:id?store_id` | `{schema:1,store_id,artifact:Artifact,revision:Revision}` |
| POST `/v1/artifacts/:id/revisions` | `{store_id,operation_id,expected_head,content}` → Receipt |
| GET `/v1/artifacts/:id/revisions?store_id&after?&limit?` | `{schema:1,store_id,artifact_id,revisions:[RevisionSummary],next_cursor:null|sequence}` |
| GET `/v1/artifacts/:id/revisions/:rev?store_id` | `{schema:1,store_id,artifact:Artifact,revision:Revision}` |
| POST `/v1/artifacts/:id/restore` | `{store_id,operation_id,expected_head,revision_id}` → 新Revision的Receipt |
| GET `/v1/artifacts/operations/:op?store_id` | 原持久Receipt；不存在明确404，不等于可自动重发 |
| GET `/v1/artifacts/:id/revisions/:rev/export?store_id` | 精确UTF-8正文byte，media_type与固定安全下载名，ETag为内容SHA；无任意path参数 |

Artifact=`{artifact_id,work_id,conversation_session_id,title,media_type,head_revision,sequence,content_sha256,content_bytes,created_at,updated_at}`。media_type闭合为text/plain或text/markdown。

Revision=`{revision_id,artifact_id,parent_revision:null|uuid,sequence,content,content_sha256,content_bytes,created_at,author:{kind:"human"|"agent",id},restored_from:null|uuid,origin?:server-controlled}`。History摘要不包含content。

Receipt=`{schema:1,store_id,operation_id,request_sha256,outcome:"committed"|"conflict",changed,artifact,revision:null|Revision,expected_head:null|uuid}`。成功200原Receipt。CAS409使用标准ErrorFrame，error.details.receipt携带原冲突Receipt；GET operation仍200回该Receipt，outcome区分。同op异payload409标准错误，不替换原receipt。错误不能被客户端当作空列表/零版本。

## 私有原生桥与文本工作面

私有桥只开放`/bridge/artifacts`固定typed子路由：GET `/store`→capabilities、根GET列表、POST `/create`→create、POST `/:id/commit`→revisions、`/:id/restore`、history/指定revision/operation/export固定映射。每个读写都核expected-context和expected-incarnation，固定本机上游、禁代理/重定向、有界正文与单次转发，不新增任意代理或文件写面。浏览器不接provider或gateway凭据。

前端schema6明确迁移旧schema1～5，保留旧工作/资料/任务。编辑器使用引擎VNode与textarea binding，不共享控制器改同一DOM子树；稳定surface key独立于head变化。首次每项目一份活跃文本稿，本机稿/已提交/冲突/Unknown分别可见。异步回复不夺焦点、不覆盖中文输入；关窗必须保留未提交稿与未知操作身份。

AI续做只基于明确已保存revision。底部要求与冻结store/artifact/base_revision/base_sha/content形成有界只读资料封套，经既有C1 canonical回合与治理执行。C1 wire schema1及原审批恢复身份规则保持；schema6另存C2原instruction/operation关联，界面显示人的要求。结果保留为绑定原版本/原tuple的整稿候选；用户又改稿或head变化时只能查看，不能自动覆盖。明确采纳先成为编辑缓冲，再保存为新revision。编辑正文可256KiB；超出当前AI封套预算应明示，不能截断却声称完整续做。

## 施工批次与可证伪验收

A. 权威契约/持久内核/工具/HTTP：create/read、原子CAS并发仅一成功、no-op、receipt同ID异载荷、丢回执与同数据重启、restore追加历史、精确export、scope/domain拒绝、seal不降级、容量拒绝。

B. 实际可编辑工作面：schema迁移与旧资料保持、新建/保存/继续输入/迟到ACK、dirty本地恢复、source/store隔离、未知写只读确认、历史与导出分清当前未保存稿。

C. AI接续与完整原生验收：原saved revision→C1候选、用户改后迟到候选不可采纳、拒绝0权威写、采纳为稿再提交、正常关闭/同数据重启恢复、原三个项目和中文草稿不变。首轮可用真实canonical runtime+受控provider，不复用已用完的临时key；外部模型质量另行记账。

每个阶段保存源码检查点；只有实际完成/验证后进度页才标通过。关键断言做反向验伪，失败证据不删除。UI由助手实屏与用户共同验收，截图只临时查看，不入库、不生成桌面截图包。
