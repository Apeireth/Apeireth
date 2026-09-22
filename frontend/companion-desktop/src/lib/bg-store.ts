/**
 * bg-store.ts — 自定义背景图存取（规范 §8 增补④，2026-09-22 主人拍板）
 *
 * 持久化落 IndexedDB（localStorage 5MB 上限放不下图片，不用）。
 *
 * 真实边界（0 装纪律，如实标注）：
 * - web dev：IndexedDB 按 localhost 原点持久化，换端口/换浏览器/清站数据后需重传；
 * - Tauri 壳：WebView2 用户数据目录里的 IndexedDB 同样可用且持久，但 desktop-bridge
 *   没有文件持久化能力（全库无 writeFile 类接口），不假装落了磁盘路径；
 * - validateCustomBgFile 是纯函数（Node 可测）；IndexedDB 三件套只在浏览器侧调用。
 */

const DB_NAME = 'apeireth-personalization';
const DB_VERSION = 1;
const STORE = 'backgrounds';
const KEY = 'custom-bg';

/** 允许的图片类型与体积上限（12MB： IndexedDB 放得下，但更大对背景无意义） */
export const CUSTOM_BG_TYPES = ['image/png', 'image/jpeg', 'image/webp', 'image/avif', 'image/gif'] as const;
export const CUSTOM_BG_MAX_BYTES = 12 * 1024 * 1024;

export type BgFileValidation = {ok: true} | {ok: false; reason: string};

/** 上传文件校验（纯函数）：类型白名单 + 体积上限，失败给出用户可读理由。 */
export function validateCustomBgFile(size: number, type: string): BgFileValidation {
  if (!(CUSTOM_BG_TYPES as readonly string[]).includes(type)) {
    return {ok: false, reason: `只支持 PNG / JPEG / WebP / AVIF / GIF 图片（收到 ${type || '未知类型'}）`};
  }
  if (size <= 0) return {ok: false, reason: '文件是空的'};
  if (size > CUSTOM_BG_MAX_BYTES) {
    return {ok: false, reason: `图片 ${(size / 1024 / 1024).toFixed(1)}MB 超过 12MB 上限，请压缩后再传`};
  }
  return {ok: true};
}

function idbAvailable(): boolean {
  return typeof indexedDB !== 'undefined';
}

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    if (!idbAvailable()) {
      reject(new Error('当前环境没有 IndexedDB，自定义背景无法持久化'));
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

/** 存入（覆盖式）。 */
export async function putCustomBg(blob: Blob): Promise<void> {
  const db = await openDb();
  try {
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction(STORE, 'readwrite');
      tx.objectStore(STORE).put(blob, KEY);
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error ?? new Error('IndexedDB 写入失败'));
    });
  } finally {
    db.close();
  }
}

/** 取出；没有存过 = null（诚实缺省，不造占位图）。 */
export async function getCustomBg(): Promise<Blob | null> {
  const db = await openDb();
  try {
    return await new Promise<Blob | null>((resolve, reject) => {
      const tx = db.transaction(STORE, 'readonly');
      const req = tx.objectStore(STORE).get(KEY);
      req.onsuccess = () => resolve((req.result as Blob | undefined) ?? null);
      req.onerror = () => reject(req.error ?? new Error('IndexedDB 读取失败'));
    });
  } finally {
    db.close();
  }
}

/** 清除。 */
export async function clearCustomBg(): Promise<void> {
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
