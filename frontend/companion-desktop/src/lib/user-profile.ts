// 用户资料（设置 ›「用户中心」第一页）— 暂只随应用存本地的那一层。
//
// 刻意不进 ApeirethConfig：config 走 apply 缝推后端/进共享配置面；用户资料是
// 「本机本人」的身份层（昵称 / 头像 / 签名），先只落 localStorage 单键——无账号、
// 无同步、不上传。将来接账号体系只换本模块的存取实现，界面零改动。
//
// 纪律：
//   - 读侧永不抛：脏 JSON / 超长字段 / 非法头像一律归一到安全缺省；
//   - 写侧如实抛：localStorage 不可用/满时让 UI 显示可读理由，不静默丢改动；
//   - 顶层不触 DOM（头像压图只在函数内用浏览器 API），Node 直接 import 可测。

/** 头像图片 data URL 的字符上限（256px JPEG 压缩后正常 ~20-60KB，给足余量）。 */
export const AVATAR_MAX_CHARS = 512 * 1024;

const NICKNAME_MAX = 32;
const BIO_MAX = 120;

/** 本地落盘单键（与 call-logger / 会话本地账同一 localStorage 口径）。 */
export const USER_PROFILE_STORAGE_KEY = 'apeireth-user-profile';

export interface UserProfile {
  /** 昵称（空 = 未设置，界面回落默认文案/默认头像）。 */
  nickname: string;
  /** 头像图片 data URL（空 = 未设置，界面回落默认头像占位）。 */
  avatar: string;
  /** 一句话签名。 */
  bio: string;
  /** 最近一次成功写入时间（ms epoch；0 = 从未写过）。 */
  updatedAt: number;
}

export const EMPTY_USER_PROFILE: UserProfile = Object.freeze({
  nickname: '',
  avatar: '',
  bio: '',
  updatedAt: 0,
});

function clampText(value: unknown, max: number): string {
  return typeof value === 'string' ? value.trim().slice(0, max) : '';
}

/** 归一：非对象 / 脏字段 / 非图片 data URL / 超限头像一律拒收或截断，不炸界面。 */
export function normalizeUserProfile(raw: unknown): UserProfile {
  const record = (raw !== null && typeof raw === 'object' ? raw : {}) as Record<string, unknown>;
  const avatar =
    typeof record.avatar === 'string' &&
    record.avatar.startsWith('data:image/') &&
    record.avatar.length <= AVATAR_MAX_CHARS
      ? record.avatar
      : '';
  const updatedAt =
    typeof record.updatedAt === 'number' && Number.isFinite(record.updatedAt) && record.updatedAt >= 0
      ? record.updatedAt
      : 0;
  return {
    nickname: clampText(record.nickname, NICKNAME_MAX),
    avatar,
    bio: clampText(record.bio, BIO_MAX),
    updatedAt,
  };
}

/** 读本地用户资料；localStorage 不可用/脏数据一律回落空资料（读侧不抛）。 */
export function loadUserProfile(): UserProfile {
  try {
    const raw = localStorage.getItem(USER_PROFILE_STORAGE_KEY);
    if (!raw) return {...EMPTY_USER_PROFILE};
    return normalizeUserProfile(JSON.parse(raw));
  } catch {
    return {...EMPTY_USER_PROFILE};
  }
}

type UserProfileListener = (profile: UserProfile) => void;
const listeners = new Set<UserProfileListener>();

/** 订阅用户资料写入（侧栏头像实时跟随）；返回退订闭包。 */
export function subscribeUserProfile(listener: UserProfileListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * 写本地用户资料：归一 + 盖 updatedAt + 落盘 + 广播订阅方。
 * localStorage 不可用/满时如实抛（写侧不静默失败，UI 提示并保住输入）。
 */
export function saveUserProfile(profile: UserProfile): UserProfile {
  const next = normalizeUserProfile({...profile, updatedAt: Date.now()});
  localStorage.setItem(USER_PROFILE_STORAGE_KEY, JSON.stringify(next)); // 可能抛（配额/隐私模式）
  for (const listener of listeners) listener(next);
  return next;
}

/**
 * 头像入库前压成 256px 居中裁方的 JPEG data URL（浏览器 API，Node 不调用）。
 * 非图片文件 / 解码失败 / 画布不可用如实抛，由界面显示可读理由。
 */
export async function downscaleAvatarImage(file: Blob): Promise<string> {
  if (!file.type.startsWith('image/')) throw new Error('头像需要是图片文件');
  const objectUrl = URL.createObjectURL(file);
  try {
    const image = await new Promise<HTMLImageElement>((resolve, reject) => {
      const el = new Image();
      el.onload = () => resolve(el);
      el.onerror = () => reject(new Error('这张图片解码失败，换一张试试'));
      el.src = objectUrl;
    });
    const size = 256;
    const canvas = document.createElement('canvas');
    canvas.width = size;
    canvas.height = size;
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('画布不可用，无法压缩头像');
    // 居中裁方：短边铺满 256，长边居中裁掉；白底避免透明图压 JPEG 后糊黑。
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, size, size);
    const side = Math.max(1, Math.min(image.naturalWidth, image.naturalHeight));
    const sx = (image.naturalWidth - side) / 2;
    const sy = (image.naturalHeight - side) / 2;
    ctx.drawImage(image, sx, sy, side, side, 0, 0, size, size);
    return canvas.toDataURL('image/jpeg', 0.85);
  } finally {
    URL.revokeObjectURL(objectUrl);
  }
}
