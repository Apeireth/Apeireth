// 星屑便签 · 纯逻辑存储（星港内置应用面 I 批 2，纯模块零依赖）
//
// 契约（I-starport-builtins.md「星屑」）：
//   1. 便签模型：id / title / body / createdAt / updatedAt / pinned；
//      updatedAt = 内容修改时刻（置顶切换不动它——置顶是独立坐标轴）；
//   2. 排序即承诺：置顶优先 → updatedAt 倒序 → createdAt 倒序 → id 升序
//      （全序确定，compareNotes 钉死，测试不许漂）；
//   3. 校验 0 装：title/body 至少一个非空（纯空白算空）、超限抛错
//      （title 200 / body 20000 字符，超限即拒不截断）、id 冲突抛错——
//      不猜、不修、不静默改写用户数据；
//   4. 持久化经注入适配器（load() / save(notes)），本模块不碰 localStorage
//      与磁盘；载入严格校验，损坏即抛（corrupt-persistence），不半信半疑；
//      load() 返回 null/undefined = 全新起步（合法，非猜测）；
//   5. list() 返回快照副本，外部改写不回流内部状态。
//
// 本文件零运行时依赖（不 import svelte / DOM），Node 直接 import 可测
// （tests/starport-notes.mjs），与 gateway-events.ts 同一约定。

/** 便签（持久化单元）。 */
export interface Note {
  id: string;
  title: string;
  body: string;
  createdAt: number;
  updatedAt: number;
  pinned: boolean;
}

/** 新建/更新输入（update 只取存在的字段，undefined = 不动）。 */
export interface NoteInput {
  title?: string;
  body?: string;
}

/** 持久化适配器（注入契约：存储不认介质，只认 load/save）。 */
export interface NotesPersistenceAdapter {
  /** 返回 null/undefined = 全新起步；返回损坏数据一律抛 corrupt-persistence。 */
  load: () => unknown;
  save: (notes: readonly Note[]) => void;
}

export const NOTE_TITLE_MAX = 200;
export const NOTE_BODY_MAX = 20_000;

export class NotesStoreError extends Error {
  readonly code:
    | 'empty-content'
    | 'limit-exceeded'
    | 'duplicate-id'
    | 'invalid-id'
    | 'missing'
    | 'corrupt-persistence';

  constructor(
    code:
      | 'empty-content'
      | 'limit-exceeded'
      | 'duplicate-id'
      | 'invalid-id'
      | 'missing'
      | 'corrupt-persistence',
    message: string,
  ) {
    super(message);
    this.name = 'NotesStoreError';
    this.code = code;
  }
}

function validateContent(title: string, body: string): void {
  if (typeof title !== 'string' || typeof body !== 'string') {
    throw new NotesStoreError('corrupt-persistence', 'title/body 必须是字符串');
  }
  if (!title.trim() && !body.trim()) {
    throw new NotesStoreError('empty-content', 'title 与 body 不能同时为空（纯空白算空）');
  }
  if (title.length > NOTE_TITLE_MAX) {
    throw new NotesStoreError('limit-exceeded', `title 超上限 ${NOTE_TITLE_MAX}（实得 ${title.length}）`);
  }
  if (body.length > NOTE_BODY_MAX) {
    throw new NotesStoreError('limit-exceeded', `body 超上限 ${NOTE_BODY_MAX}（实得 ${body.length}）`);
  }
}

/**
 * 排序即承诺：置顶优先 → updatedAt 倒序 → createdAt 倒序 → id 升序。
 * 纯函数，list() 与载入共用同一全序。
 */
export function compareNotes(a: Note, b: Note): number {
  if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
  if (a.updatedAt !== b.updatedAt) return b.updatedAt - a.updatedAt;
  if (a.createdAt !== b.createdAt) return b.createdAt - a.createdAt;
  return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
}

/**
 * 严格解析持久化载荷（0 装：字段缺失/类型不符/id 重复一律抛，
 * 不修补、不留半条）。null/undefined = 全新起步，返回空表。
 */
export function parseNoteList(raw: unknown): Note[] {
  if (raw === null || raw === undefined) return [];
  if (!Array.isArray(raw)) {
    throw new NotesStoreError('corrupt-persistence', '持久化载荷必须是便签数组');
  }
  const seen = new Set<string>();
  const notes: Note[] = [];
  for (const item of raw) {
    if (!item || typeof item !== 'object' || Array.isArray(item)) {
      throw new NotesStoreError('corrupt-persistence', '便签条目必须是对象');
    }
    const n = item as Record<string, unknown>;
    const {id, title, body, createdAt, updatedAt, pinned} = n;
    if (
      typeof id !== 'string' || !id ||
      typeof createdAt !== 'number' || !Number.isFinite(createdAt) ||
      typeof updatedAt !== 'number' || !Number.isFinite(updatedAt) ||
      typeof pinned !== 'boolean'
    ) {
      throw new NotesStoreError('corrupt-persistence', '便签条目字段缺失或类型不符');
    }
    validateContent(title as string, body as string);
    if (seen.has(id)) {
      throw new NotesStoreError('duplicate-id', `持久化载荷 id 冲突：${id}`);
    }
    seen.add(id);
    notes.push({id, title: title as string, body: body as string, createdAt, updatedAt, pinned});
  }
  return notes;
}

/** 星屑便签存储。适配器可缺省（纯内存模式）。 */
export class NotesStore {
  private notes: Note[];
  private readonly adapter: NotesPersistenceAdapter | null;

  constructor(options?: {adapter?: NotesPersistenceAdapter | null}) {
    this.adapter = options?.adapter ?? null;
    this.notes = parseNoteList(this.adapter ? this.adapter.load() : null);
  }

  /**
   * 新建便签。id 由调用方给定（命名即承诺），空 id / 冲突即抛。
   * title/body 缺省视为空串（声明的默认值，非修补）。
   */
  add(
    input: {id: string; title?: string; body?: string; pinned?: boolean},
    now: number,
  ): Note {
    if (typeof input.id !== 'string' || !input.id) {
      throw new NotesStoreError('invalid-id', '新建便签必须带非空 id');
    }
    const title = input.title ?? '';
    const body = input.body ?? '';
    validateContent(title, body);
    if (this.notes.some((n) => n.id === input.id)) {
      throw new NotesStoreError('duplicate-id', `便签 id 冲突：${input.id}`);
    }
    const note: Note = {
      id: input.id,
      // 用归一后的 title/body（input 字段可选，直接赋会把 undefined 带进 Note）
      title,
      body,
      createdAt: now,
      updatedAt: now,
      pinned: input.pinned ?? false,
    };
    this.notes.push(note);
    this.persist();
    return {...note};
  }

  /** 更新内容（title/body 只取传入字段；更新后整体重新校验）。 */
  update(id: string, patch: NoteInput, now: number): Note {
    const note = this.require(id);
    const title = patch.title === undefined ? note.title : patch.title;
    const body = patch.body === undefined ? note.body : patch.body;
    validateContent(title, body);
    const next: Note = {...note, title, body, updatedAt: now};
    this.notes = this.notes.map((n) => (n.id === id ? next : n));
    this.persist();
    return {...next};
  }

  /** 删除便签，返回被删快照。 */
  remove(id: string): Note {
    const note = this.require(id);
    this.notes = this.notes.filter((n) => n.id !== id);
    this.persist();
    return {...note};
  }

  /** 置顶开关（独立坐标轴：不动 updatedAt，见契约 §1）。 */
  togglePin(id: string): Note {
    const note = this.require(id);
    const next: Note = {...note, pinned: !note.pinned};
    this.notes = this.notes.map((n) => (n.id === id ? next : n));
    this.persist();
    return {...next};
  }

  /** 快照列表（compareNotes 全序；副本出参，外部改写不回流）。 */
  list(): Note[] {
    return [...this.notes].sort(compareNotes).map((n) => ({...n}));
  }

  private require(id: string): Note {
    const note = this.notes.find((n) => n.id === id);
    if (!note) throw new NotesStoreError('missing', `便签不存在：${id}`);
    return note;
  }

  private persist(): void {
    this.adapter?.save(this.notes.map((n) => ({...n})));
  }
}
