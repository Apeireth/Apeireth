// 星港 · 内置面清单（I 批 2–5 交付台账，纯逻辑零依赖）
//
// 契约（I-starport-builtins.md 铁律「装一件亮一件」）：
//   1. 三态口径：live=面完整可用 / slice=MVP 切片可用（扩展位在册）/
//      not_assembled=未组装（依赖原生后端或后续批次，诚实降级不假旋钮）；
//   2. 每面必带 note：slice 说明切片边界，not_assembled 说明缺什么（0 装：
//      不许无理由上架、不许把切片吹成整面）；
//   3. 清单是单一真源：星港面板与完工汇报都从这张表读口径，禁止他处另说一套；
//   4. 命名即承诺：id=中性英文面名，codename=已定科幻代号（见命名体系）。
//
// 测试：tests/starport-faces.mjs（真实模块直测）。

export type FaceStatus = 'live' | 'slice' | 'not_assembled';

export interface StarportFace {
  /** 中性英文面名（工具/文件命名用）。 */
  id: string;
  /** 科幻代号（对外命名，owner 拍板）。 */
  codename: string;
  title: string;
  status: FaceStatus;
  /** 必填：切片边界或未组装缺件（诚实口径）。 */
  note: string;
}

export const STARPORT_FACES: readonly StarportFace[] = [
  {
    id: 'host',
    codename: '星港',
    title: '应用宿主总成',
    status: 'live',
    note: '贡献点注册表 + 多标签壳 + 工作台卡片入口（批 1 已交）。',
  },
  {
    id: 'agent-status',
    codename: '伴星',
    title: '任务状态形象面',
    status: 'live',
    note: '五态状态机 + 桌宠状态标 + 面板代审批（批 1 已交）。',
  },
  {
    id: 'notes',
    codename: '星屑',
    title: '便签面',
    status: 'live',
    note: '便签增删改查/置顶/持久化适配器（批 2）。',
  },
  {
    id: 'doc',
    codename: '星笺',
    title: '文档面',
    status: 'slice',
    note: 'MVP：唯一真源分段 + 字符区间/哈希 + expected 区间事务写回 + 渲染态直改映射 + 可编程岛（稳定语义 ID、沙箱 iframe、销毁即释放）。',
  },
  {
    id: 'text',
    codename: '铭刻台',
    title: '文本/源码面',
    status: 'slice',
    note: 'MVP：多会话文本编辑 + expected 区间事务 + 查找；语法高亮为后续扩展位。',
  },
  {
    id: 'browse',
    codename: '深望',
    title: '浏览面',
    status: 'slice',
    note: 'MVP：受控浏览视图（HTTP/HTTPS 白名单）+ 抓取转纯文本；语义树操控内核待安全纵深对象级判定后接。',
  },
  {
    id: 'organize',
    codename: '泊位',
    title: '桌面整理面',
    status: 'slice',
    note: 'MVP：真实目录列举磁贴（目录优先/下钻）+ 收纳账本 + 映射本机持久化（坏载荷如实报）+ 写盘三手势（建/改名/删，非空目录防误删拒绝）。',
  },
  {
    id: 'voice',
    codename: '星语',
    title: '语音面',
    status: 'slice',
    note: 'MVP：真实麦克风采集/电平/录制/回放 + 系统语音合成器真朗读 + 系统语音识别引擎真转写（受控子进程桥接、空结果如实）；全部缝清零。',
  },
  {
    id: 'memory',
    codename: '深流',
    title: '记忆压缩检索面',
    status: 'slice',
    note: 'MVP：观察账本 + 自然日压缩要点 + 词项召回；沉淀/召回走真实记忆通道（/v1/memory/append、/v1/panel/memory/episodes）；模型摘要注入为后续扩展位。',
  },
  {
    id: 'ecosystem',
    codename: '船坞',
    title: '生态分发面',
    status: 'slice',
    note: 'MVP：已装贡献点清单（注册表真值）；插件/技能/主题市场待后端分发面。',
  },
  {
    id: 'terminal',
    codename: '舵室',
    title: '终端面',
    status: 'slice',
    note: 'MVP：用户直操终端（工作区钉死/超时强杀/输出截断）；智能体驱动走审批族为接线缝。',
  },
  {
    id: 'mail',
    codename: '信使',
    title: '邮件面',
    status: 'slice',
    note: 'MVP：本地邮件台（.eml 真实文件面 + 头注入防御 + 收发件箱账本）+ SMTP 真实发送（隐式 TLS/STARTTLS/明文双确认）+ IMAP 信封级收信（三模式 TLS）；整封正文取回为增强位。',
  },
  {
    id: 'chart',
    codename: '星仪',
    title: '图表面',
    status: 'slice',
    note: 'MVP：数据驱动折线/柱状 SVG 渲染；三方渲染引擎为后续扩展位。',
  },
  {
    id: 'database',
    codename: '矿脉',
    title: '数据库工作台面',
    status: 'slice',
    note: 'MVP：只读 SQL 三引擎（SQLite 文件直查 + PostgreSQL 线协议（MD5/SCRAM 认证）+ MySQL 线协议（native/caching_sha2 加扰）），千行截断/CSV 导出；扩展查询协议与更多库为增强位。',
  },
  {
    id: 'translate',
    codename: '通译',
    title: '翻译面',
    status: 'slice',
    note: 'MVP：走真实模型通道直译/意译；自定义输出结构为后续扩展位。',
  },
  {
    id: 'lifeflow',
    codename: '信风',
    title: '生活信息流面',
    status: 'slice',
    note: 'MVP：信息源登记与条目流管理；多源抓取适配待深望抓取面扩批（J 想法）。',
  },
  {
    id: 'portable',
    codename: '星槎',
    title: '便携随身面',
    status: 'slice',
    note: 'MVP：便携判定与数据随身（标记文件/数据目录全链一致）；打包随身为接线缝。',
  },
];

export function listFacesByStatus(status: FaceStatus): StarportFace[] {
  return STARPORT_FACES.filter((face) => face.status === status);
}

export function getFace(id: string): StarportFace | null {
  return STARPORT_FACES.find((face) => face.id === id) ?? null;
}
