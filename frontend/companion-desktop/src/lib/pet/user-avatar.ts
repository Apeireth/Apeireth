/**
 * user-avatar.ts — 用户自带形象（BYO）装载口的纯逻辑与持久化
 *
 * 渠道纪律（avatar-pack-admission.md §1b）：用户自带形象 = 用户自担许可责任。
 * 本模块只做**格式合法性**校验与本机持久化，不审用户资产的许可（免责文案见
 * USER_AVATAR_DISCLAIMER，装载前必呈现）。平台侧注册的内置/官方/测试形象才走
 * avatar-registry 的许可红线（manifest v2）。
 *
 * 真实边界（0 装纪律，如实标注）：
 * - 持久化落 IndexedDB（同 bg-store.ts 口径）：web dev 按原点持久、换端口/清站
 *   数据需重选；Tauri 壳 WebView2 数据目录内持久，但不假装落了磁盘路径；
 * - validateUserAvatarFile 是纯函数（Node 可测）；IndexedDB 三件套只在浏览器侧调用；
 * - 形象文件内嵌的许可元数据由渲染层读出后透传（onModelMeta），未声明就是未声明，
 *   不替用户解读、不替第三方背书。
 */

// 独立库名（不与 bg-store.ts 共库）：同库多 store 要动对方的升级钩子，
// 跨范围最小改动原则下宁可独立成库，零耦合。
const DB_NAME = 'apeireth-pet';
const DB_VERSION = 1;
const STORE = 'user-avatar';
const KEY = 'user-avatar';

/** 允许的形象文件扩展名（开放 3D 人形格式的容器）。 */
export const USER_AVATAR_EXTENSIONS = ['.vrm', '.glb'] as const;

/** 体积上限（64MB：形象文件的合理上限，更大的不收——IndexedDB 与内存都省）。 */
export const USER_AVATAR_MAX_BYTES = 64 * 1024 * 1024;

/** 容器魔数：glTF 二进制头 4 字节（两种扩展名共用同一容器）。 */
const GLTF_BINARY_MAGIC = 'glTF';

/** 装载前必呈现的免责文案（用户自带形象 = 用户自担许可责任）。 */
export const USER_AVATAR_DISCLAIMER =
  '这是你自带的形象文件。请确认你有权使用它（含展示与修改）；' +
  '平台不分发、不托管、不生成派生，许可责任由你自担。' +
  '不同来源的授权条款各异，接入前请自行核对。';

/** 形象文件内嵌许可元数据的归一形状（渲染层读出透传；未声明 = 对应字段 null）。 */
export interface UserAvatarMeta {
  /** 文件内嵌的授权条款链接（无则 null，不猜） */
  licenseUrl: string | null;
  /** 商用许可声明（无则 null） */
  commercialUsage: string | null;
  /** 再分发许可声明（无则 null） */
  redistribution: string | null;
}

export const EMPTY_USER_AVATAR_META: UserAvatarMeta = {
  licenseUrl: null,
  commercialUsage: null,
  redistribution: null,
};

/**
 * 形象文件内嵌许可元数据归一（纯函数）。开放 3D 人形格式的模型自带许可声明
 * 字段（各代/各工具字段名不同），这里防御性读取归一成 UserAvatarMeta：
 * 读得到什么报什么，读不到 = null（未声明），**不替用户解读、不替第三方背书**。
 */
export function normalizeVrmMetaLicense(raw: unknown): UserAvatarMeta {
  if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) return EMPTY_USER_AVATAR_META;
  const m = raw as Record<string, unknown>;
  const firstString = (...keys: string[]): string | null => {
    for (const key of keys) {
      const value = m[key];
      if (typeof value === 'string' && value.trim()) return value.trim();
    }
    return null;
  };
  return {
    licenseUrl: firstString('licenseUrl', 'otherLicenseUrl'),
    commercialUsage: firstString('commercialUsage', 'commercialUssageName', 'allowCommercialUsage'),
    redistribution: firstString('allowRedistribution', 'redistribution'),
  };
}

export type AvatarFileValidation = {ok: true} | {ok: false; reason: string};

/**
 * 用户形象文件校验（纯函数）：扩展名白名单 + 体积上限 + glTF 二进制魔数。
 * headBytes 取文件头若干字节（≥4）；给不出魔数 = 不猜不收。
 */
export function validateUserAvatarFile(
  name: string,
  size: number,
  headBytes: Uint8Array | null,
): AvatarFileValidation {
  const lower = name.toLowerCase();
  const extOk = (USER_AVATAR_EXTENSIONS as readonly string[]).some((ext) => lower.endsWith(ext));
  if (!extOk) {
    return {
      ok: false,
      reason: `只支持 ${USER_AVATAR_EXTENSIONS.join(' / ')} 形象文件（收到 ${name || '未命名文件'}）`,
    };
  }
  if (size <= 0) return {ok: false, reason: '文件是空的'};
  if (size > USER_AVATAR_MAX_BYTES) {
    return {
      ok: false,
      reason: `形象 ${(size / 1024 / 1024).toFixed(1)}MB 超过 64MB 上限`,
    };
  }
  if (!headBytes || headBytes.length < 4) {
    return {ok: false, reason: '读不到文件头，无法确认是 glTF 二进制容器（不猜不收）'};
  }
  const magic = String.fromCharCode(...headBytes.slice(0, 4));
  if (magic !== GLTF_BINARY_MAGIC) {
    return {ok: false, reason: '文件头不是 glTF 二进制容器，可能是损坏文件或别的格式'};
  }
  return {ok: true};
}

// ============================================================
// IndexedDB 持久化（浏览器侧三件套，口径同 bg-store.ts）
// ============================================================

function idbAvailable(): boolean {
  return typeof indexedDB !== 'undefined';
}

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    if (!idbAvailable()) {
      reject(new Error('当前环境没有 IndexedDB，自带形象无法持久化'));
      return;
    }
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      req.result.createObjectStore(STORE);
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error('IndexedDB 打开失败'));
  });
}

/** 存入（覆盖式）：形象文件本体 + 接受免责的时刻（审计留痕）。 */
export async function putUserAvatar(blob: Blob, acceptedAt: number): Promise<void> {
  const db = await openDb();
  try {
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction(STORE, 'readwrite');
      tx.objectStore(STORE).put({blob, acceptedAt}, KEY);
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error ?? new Error('IndexedDB 写入失败'));
    });
  } finally {
    db.close();
  }
}

export interface StoredUserAvatar {
  blob: Blob;
  /** 用户接受免责文案的时刻（epoch ms） */
  acceptedAt: number;
}

/** 取出；没有存过 = null（诚实缺省，不造占位形象）。 */
export async function getUserAvatar(): Promise<StoredUserAvatar | null> {
  const db = await openDb();
  try {
    return await new Promise<StoredUserAvatar | null>((resolve, reject) => {
      const tx = db.transaction(STORE, 'readonly');
      const req = tx.objectStore(STORE).get(KEY);
      req.onsuccess = () => resolve((req.result as StoredUserAvatar | undefined) ?? null);
      req.onerror = () => reject(req.error ?? new Error('IndexedDB 读取失败'));
    });
  } finally {
    db.close();
  }
}

/** 清除（含撤销对象 URL 的责任在调用方——URL 是借的，还的时候自己 revoke）。 */
export async function clearUserAvatar(): Promise<void> {
  const db = await openDb();
  try {
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction(STORE, 'readwrite');
      tx.objectStore(STORE).delete(KEY);
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error ?? new Error('IndexedDB 删除失败'));
    });
  } finally {
    db.close();
  }
}
