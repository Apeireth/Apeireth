// 星港 · 贡献点注册表（内置应用面 I 批 1，纯逻辑零依赖）
//
// 契约（I-starport-builtins.md）：
//   1. 数据优先 —— 内置件/命令以纯数据定义注册（id/title/kind/mount(handler)），
//      注册表只存定义，不执行渲染；
//   2. 防注入面 —— 定义禁止携带 html/innerHTML/template/url/channel 字段
//      （注册面不许成为注入面，零例外）；mount 只拿容器与上下文；
//   3. 命名即承诺 —— id 小写连字符，唯一；重复注册一律拒绝（不猜不合并）；
//   4. owner 生命周期 —— 注册可挂 owner，owner 失活拒绝注册，释放 owner
//      连带注销其全部贡献点（组件销毁不留孤儿）；
//   5. 0 装 —— 非法定义抛错而非静默修正；不猜、不修、不吞。
//
// 本文件刻意零运行时依赖（不 import svelte / DOM），纯函数可被 Node 直接
// import 测试（tests/starport-registry.mjs），与 gateway-events.ts 同一约定。

/** 内置应用贡献点（挂载协议：mount 只拿容器与上下文，可返回清理函数）。 */
export interface AppContribution {
  id: string;
  title: string;
  /** 固定 'internal' —— 星港批 1 只收内置件；外部嵌入件走后续批次的独立审批面。 */
  kind: 'internal';
  icon?: string;
  /** 容器由宿主层提供（Svelte 层传真实元素，测试传桩对象；本模块不触 DOM）。 */
  mount: (container: unknown, context: AppMountContext) => void | (() => void);
}

export interface AppMountContext {
  viewId: string;
  appId: string;
  /** 内置件可自关其视图（与宿主层 closeView 同一链路）。 */
  close: () => void;
}

/** 命令贡献点（命令面板/快捷键共用一个面）。 */
export interface CommandContribution {
  id: string;
  title: string;
  handler: () => void | Promise<void>;
  alias?: string[];
}

export interface ContributionOwner {
  id: string;
  active?: boolean;
}

export type ContributionEvent =
  | {kind: 'app'; id: string; action: 'registered' | 'unregistered'}
  | {kind: 'command'; id: string; action: 'registered' | 'unregistered'};

/** 注册面禁带字段（防注入；新增字段必须先过安全评审）。 */
export const FORBIDDEN_DEFINITION_FIELDS: readonly string[] = [
  'html',
  'innerHTML',
  'template',
  'url',
  'channel',
];

const ID_PATTERN = /^[a-z][a-z0-9-]{1,63}$/;

export class ContributionRegistryError extends Error {
  readonly code:
    | 'invalid-id'
    | 'invalid-definition'
    | 'forbidden-field'
    | 'duplicate'
    | 'missing'
    | 'owner-inactive'
    | 'owner-mismatch';

  constructor(
    code:
      | 'invalid-id'
      | 'invalid-definition'
      | 'forbidden-field'
      | 'duplicate'
      | 'missing'
      | 'owner-inactive'
      | 'owner-mismatch',
    message: string,
  ) {
    super(message);
    this.name = 'ContributionRegistryError';
    this.code = code;
  }
}

interface Owned<T> {
  definition: T;
  ownerId: string | null;
}

function validateId(id: unknown, what: string): string {
  if (typeof id !== 'string' || !ID_PATTERN.test(id)) {
    throw new ContributionRegistryError('invalid-id', `${what} id 非法（小写连字符，2-64 字符）：${String(id)}`);
  }
  return id;
}

function rejectForbiddenFields(definition: Record<string, unknown>): void {
  const hit = FORBIDDEN_DEFINITION_FIELDS.find((field) => Object.hasOwn(definition, field));
  if (hit) {
    throw new ContributionRegistryError(
      'forbidden-field',
      `贡献点定义禁止携带注入面字段「${hit}」（注册面不许成为注入面）`,
    );
  }
}

function validateApp(definition: unknown): AppContribution {
  if (!definition || typeof definition !== 'object' || Array.isArray(definition)) {
    throw new ContributionRegistryError('invalid-definition', '应用贡献点定义必须是对象');
  }
  const def = definition as unknown as Record<string, unknown>;
  rejectForbiddenFields(def);
  validateId(def.id, '应用');
  if (typeof def.title !== 'string' || !def.title) {
    throw new ContributionRegistryError('invalid-definition', `应用 ${String(def.id)} 缺 title`);
  }
  if (def.kind !== 'internal') {
    throw new ContributionRegistryError('invalid-definition', `应用 ${String(def.id)} 的 kind 只能是 internal`);
  }
  if (typeof def.mount !== 'function') {
    throw new ContributionRegistryError('invalid-definition', `应用 ${String(def.id)} 缺 mount()`);
  }
  return definition as AppContribution;
}

function validateCommand(definition: unknown): CommandContribution {
  if (!definition || typeof definition !== 'object' || Array.isArray(definition)) {
    throw new ContributionRegistryError('invalid-definition', '命令贡献点定义必须是对象');
  }
  const def = definition as unknown as Record<string, unknown>;
  rejectForbiddenFields(def);
  validateId(def.id, '命令');
  if (typeof def.title !== 'string' || !def.title) {
    throw new ContributionRegistryError('invalid-definition', `命令 ${String(def.id)} 缺 title`);
  }
  if (typeof def.handler !== 'function') {
    throw new ContributionRegistryError('invalid-definition', `命令 ${String(def.id)} 缺 handler()`);
  }
  return definition as CommandContribution;
}

export class ContributionRegistry {
  private readonly apps = new Map<string, Owned<AppContribution>>();
  private readonly commands = new Map<string, Owned<CommandContribution>>();
  private readonly listeners = new Set<(event: ContributionEvent) => void>();

  registerApp(definition: AppContribution, options?: {owner?: ContributionOwner}): AppContribution {
    const def = validateApp(definition);
    this.guardOwner(options?.owner);
    if (this.apps.has(def.id) || this.commands.has(def.id)) {
      throw new ContributionRegistryError('duplicate', `贡献点 id 冲突：${def.id}`);
    }
    this.apps.set(def.id, {definition: def, ownerId: options?.owner?.id ?? null});
    this.emit({kind: 'app', id: def.id, action: 'registered'});
    return def;
  }

  registerCommand(
    definition: CommandContribution,
    options?: {owner?: ContributionOwner},
  ): CommandContribution {
    const def = validateCommand(definition);
    this.guardOwner(options?.owner);
    if (this.apps.has(def.id) || this.commands.has(def.id)) {
      throw new ContributionRegistryError('duplicate', `贡献点 id 冲突：${def.id}`);
    }
    this.commands.set(def.id, {definition: def, ownerId: options?.owner?.id ?? null});
    this.emit({kind: 'command', id: def.id, action: 'registered'});
    return def;
  }

  getApp(id: string): AppContribution | null {
    return this.apps.get(id)?.definition ?? null;
  }

  getCommand(id: string): CommandContribution | null {
    return this.commands.get(id)?.definition ?? null;
  }

  listApps(): AppContribution[] {
    return [...this.apps.values()].map((entry) => entry.definition);
  }

  listCommands(): CommandContribution[] {
    return [...this.commands.values()].map((entry) => entry.definition);
  }

  /** 注销单个贡献点；挂了 owner 的只能由同一 owner 注销（防误删他人件）。 */
  unregister(
    kind: 'app' | 'command',
    id: string,
    options?: {owner?: ContributionOwner},
  ): void {
    const store = kind === 'app' ? this.apps : this.commands;
    const entry = store.get(id);
    if (!entry) throw new ContributionRegistryError('missing', `贡献点不存在：${id}`);
    const owner = options?.owner;
    if (entry.ownerId !== (owner?.id ?? null)) {
      throw new ContributionRegistryError('owner-mismatch', `贡献点 ${id} 的 owner 不匹配，拒绝注销`);
    }
    store.delete(id);
    this.emit({kind, id, action: 'unregistered'});
  }

  /** owner 释放：连带注销其全部贡献点（组件销毁不留孤儿）。返回注销数量。 */
  releaseOwner(ownerId: string): number {
    let removed = 0;
    for (const [kind, store] of [
      ['app', this.apps],
      ['command', this.commands],
    ] as const) {
      for (const [id, entry] of [...store.entries()]) {
        if (entry.ownerId === ownerId) {
          store.delete(id);
          this.emit({kind, id, action: 'unregistered'});
          removed += 1;
        }
      }
    }
    return removed;
  }

  subscribe(listener: (event: ContributionEvent) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private guardOwner(owner: ContributionOwner | undefined): void {
    if (owner && owner.active === false) {
      throw new ContributionRegistryError('owner-inactive', `owner ${owner.id} 已失活，拒绝注册`);
    }
  }

  private emit(event: ContributionEvent): void {
    for (const listener of this.listeners) listener(event);
  }
}
