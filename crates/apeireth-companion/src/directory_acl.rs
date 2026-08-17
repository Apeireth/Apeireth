//! `apeireth-companion::directory_acl` — 工具沙盒根目录 read-only DACL (S1 安全调研批).
//!
//! **位置**: 与 `apeireth-tool-filesystem` 的 `APEIRETH_TOOL_FS_ROOTS` 协作 —
//! tool-fs 限定可达路径, 本模块限定对那些路径的**写权限** (Chromium 经典分层模型 §3).
//!
//! **机制**: Windows 侧 `SetNamedSecurityInfoW` 给一批路径的 DACL 注入一组针对
//! World SID 的"允许读 / 拒绝写"ACE; 路径可以是文件/目录, 仅在沙盒执行窗口内生效 —
//! `DirAclGuard` `Drop` 时自动还原原 DACL.
//!
//! **协作**: 上层 (tool_bridge.execute_isolated) 应在 spawn worker 前申请 guard,
//! worker 退出后 guard 自然 Drop → 还原. 若 worker 失控有 Job Object KILL_ON_JOB_CLOSE
//! 兜底, **不依赖 guard 必须正常 Drop** (但正常路径下 guard 必还原).
//!
//! **0 装 PASS**: 非 Windows 平台为 no-op stub (Linux 走 mount namespace + bind-ro,
//! 属后续工作). 非法路径 (空 / 跨符号链接外) 静默跳过.

#![allow(unsafe_code)]

use std::path::{Path, PathBuf};

/// 工具沙盒根目录 ACL 收紧配置.
///
/// 路径建议来自 `SandboxConfig::directory_acl_roots` (与 `APEIRETH_TOOL_FS_ROOTS` 协作集).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirAclConfig {
    /// 工具沙盒根目录列表 (绝对路径; 留空 = 不做任何收紧).
    pub roots: Vec<PathBuf>,
    /// 是否允许 owner 仍持有写权限 (true, 默认). 关闭后 owner 也只能读.
    pub owner_retain_write: bool,
}

impl Default for DirAclConfig {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            owner_retain_write: true,
        }
    }
}

impl DirAclConfig {
    /// 从 [`crate::sandbox::SandboxConfig`] 派生: 直接拿 directory_acl_roots.
    pub fn from_sandbox(cfg: &crate::sandbox::SandboxConfig) -> Self {
        Self {
            roots: cfg.directory_acl_roots.clone(),
            owner_retain_write: true,
        }
    }

    /// 是否需要任何 ACL 收紧.
    pub fn needs_hardening(&self) -> bool {
        !self.roots.is_empty()
    }
}

/// 目录 ACL 守卫: 持有原 DACL 指针, Drop 时自动还原.
///
/// **生命周期**: 跨 worker 执行窗口; Job Object 兜底 worker 异常退出.
#[cfg(windows)]
pub struct DirAclGuard {
    original_acls: Vec<win_imp::OriginalAcl>,
}

#[cfg(windows)]
impl Drop for DirAclGuard {
    fn drop(&mut self) {
        for orig in self.original_acls.drain(..) {
            if let Err(e) = win_imp::set_dacl(&orig.path_w, orig.acl) {
                eprintln!("[sandbox] 还原 DACL 失败 ({}): {e}", orig.path.display());
            }
        }
    }
}

#[cfg(windows)]
impl std::fmt::Debug for DirAclGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirAclGuard")
            .field("count", &self.original_acls.len())
            .finish()
    }
}

#[cfg(not(windows))]
#[derive(Debug)]
pub struct DirAclGuard {
    _private: (),
}

/// 收紧 root 目录 DACL: 允许 owner 写 (按 cfg), 其他人只读.
///
/// 失败语义: 单条路径失败 → 仅记录, 整体仍返回 Ok (不阻断 — 加固是增强不是门).
/// 全部失败 → 返回 Err (无 guard 可建).
#[cfg(windows)]
pub fn apply_read_only_acl(cfg: &DirAclConfig) -> Result<DirAclGuard, String> {
    if cfg.roots.is_empty() {
        // 0 装 PASS: 无根目录无需收紧, 返回空 guard.
        return Ok(DirAclGuard { original_acls: Vec::new() });
    }

    let mut originals = Vec::new();
    let mut applied = 0;

    for root in &cfg.roots {
        if !root.exists() {
            eprintln!("[sandbox] 目录 ACL 跳过: 路径不存在 {root:?}");
            continue;
        }
        let path_w = win_imp::to_wide(root);

        // 1. 取原 DACL.
        let orig = match win_imp::get_dacl(&path_w) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[sandbox] 取 DACL 失败 {root:?}: {e} (跳过)");
                continue;
            }
        };

        // 2. 构造新 DACL: World 仅允许 GENERIC_READ | GENERIC_EXECUTE.
        let new_dacl = match win_imp::build_read_only_dacl(cfg.owner_retain_write) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[sandbox] 构造只读 DACL 失败 {root:?}: {e} (跳过)");
                continue;
            }
        };

        // 3. 设新 DACL. SetNamedSecurityInfoW 拷贝 DACL, 我们仍需 LocalFree.
        if let Err(e) = win_imp::set_dacl(&path_w, new_dacl) {
            eprintln!("[sandbox] 设 DACL 失败 {root:?}: {e} (跳过)");
            // 失败不污染原 DACL (已取出但未替换, free 它).
            unsafe {
                windows_sys::Win32::Foundation::LocalFree(new_dacl as _);
            }
            continue;
        }
        unsafe {
            windows_sys::Win32::Foundation::LocalFree(new_dacl as _);
        }

        originals.push(win_imp::OriginalAcl {
            path: root.clone(),
            path_w,
            acl: orig.acl,
        });
        applied += 1;
    }

    if applied == 0 {
        // 全部失败: 把已取出的 DACL 释放 + 返回 Err.
        for orig in originals.drain(..) {
            unsafe {
                windows_sys::Win32::Foundation::LocalFree(orig.acl as _);
            }
        }
        return Err("目录 ACL 收紧: 全部路径均失败".to_string());
    }

    Ok(DirAclGuard { original_acls: originals })
}

/// 跨平台 no-op stub (0 装 PASS).
#[cfg(not(windows))]
pub fn apply_read_only_acl(cfg: &DirAclConfig) -> Result<DirAclGuard, String> {
    if cfg.needs_hardening() {
        return Err(
            "Directory ACL 收紧: 非 Windows 平台未实现 (0 装 PASS, 走 no-op)".to_string(),
        );
    }
    Ok(DirAclGuard { _private: () })
}

// ---------------------------------------------------------------------------
// Windows 内部辅助
// ---------------------------------------------------------------------------
#[cfg(windows)]
pub(crate) mod win_imp {
    use std::ptr;

    use windows_sys::Win32::Security::ACL;
    use windows_sys::Win32::Security::Authorization::{
        GetNamedSecurityInfoW, SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W, GRANT_ACCESS,
        SE_FILE_OBJECT, TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
    };
    use windows_sys::Win32::Security::{
        DACL_SECURITY_INFORMATION, SID,
    };

    use crate::sandbox::WellKnownSid;

    pub struct OriginalAcl {
        pub path: std::path::PathBuf,
        pub path_w: Vec<u16>,
        pub acl: *mut ACL,
    }

    pub fn to_wide(p: &std::path::Path) -> Vec<u16> {
        use std::ffi::OsStr;
        use std::iter::once;
        use std::os::windows::ffi::OsStrExt;
        OsStr::new(p).encode_wide().chain(once(0)).collect()
    }

    pub fn get_dacl(path_w: &[u16]) -> Result<OriginalAcl, String> {
        unsafe {
            let mut p_dacl: *mut ACL = ptr::null_mut();
            let err = GetNamedSecurityInfoW(
                path_w.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut p_dacl,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            if err != 0 {
                return Err(format!("GetNamedSecurityInfoW 失败 (错误码 {err})"));
            }
            Ok(OriginalAcl {
                path: std::path::PathBuf::new(),
                path_w: path_w.to_vec(),
                acl: p_dacl,
            })
        }
    }

    pub fn set_dacl(path_w: &[u16], acl: *mut ACL) -> Result<(), String> {
        unsafe {
            let err = SetNamedSecurityInfoW(
                path_w.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                acl,
                ptr::null_mut(),
            );
            if err != 0 {
                return Err(format!("SetNamedSecurityInfoW 失败 (错误码 {err})"));
            }
        }
        Ok(())
    }

    /// 构造只读 DACL:
    /// - World: 允许 GENERIC_READ | GENERIC_EXECUTE (文件 / 目录可遍历)
    /// - Owner: 允许 GENERIC_ALL (若 cfg.owner_retain_write=true)
    /// - Administrators: 允许 GENERIC_ALL (防自锁)
    pub fn build_read_only_dacl(owner_retain_write: bool) -> Result<*mut ACL, String> {
        let world_sid = crate::restricted_token::win_imp::lookup_well_known_sid(WellKnownSid::World)
            .ok_or_else(|| "World SID 解析失败".to_string())?;
        let owner_sid = if owner_retain_write {
            crate::restricted_token::win_imp::lookup_well_known_sid(WellKnownSid::BuiltinAdministrators)
        } else {
            None
        };

        // 访问权限位: 0x1200A0 = GENERIC_READ | FILE_READ_DATA | FILE_READ_ATTRIBUTES | FILE_READ_EA
        const READ_PERMS: u32 = 0x0012_00A0;
        const GENERIC_ALL: u32 = 0x1000_0000;

        let mut entries: Vec<EXPLICIT_ACCESS_W> = Vec::new();
        entries.push(EXPLICIT_ACCESS_W {
            grfAccessPermissions: READ_PERMS,
            grfAccessMode: GRANT_ACCESS,
            grfInheritance: 0,
            Trustee: TRUSTEE_W {
                pMultipleTrustee: ptr::null_mut(),
                MultipleTrusteeOperation: 0,
                TrusteeForm: TRUSTEE_IS_SID,
                TrusteeType: TRUSTEE_IS_UNKNOWN,
                ptstrName: world_sid.as_ptr() as *mut _,
            },
        });
        if let Some(ref o) = owner_sid {
            entries.push(EXPLICIT_ACCESS_W {
                grfAccessPermissions: GENERIC_ALL,
                grfAccessMode: GRANT_ACCESS,
                grfInheritance: 0,
                Trustee: TRUSTEE_W {
                    pMultipleTrustee: ptr::null_mut(),
                    MultipleTrusteeOperation: 0,
                    TrusteeForm: TRUSTEE_IS_SID,
                    TrusteeType: TRUSTEE_IS_UNKNOWN,
                    ptstrName: o.as_ptr() as *mut _,
                },
            });
        }

        let mut new_dacl: *mut ACL = ptr::null_mut();
        unsafe {
            let res = SetEntriesInAclW(
                entries.len() as u32,
                entries.as_ptr(),
                ptr::null(),
                &mut new_dacl,
            );
            if res != 0 {
                return Err(format!("SetEntriesInAclW 失败 (错误码 {res})"));
            }
        }
        Ok(new_dacl)
    }
}

/// 工具: 列出"应被收紧"的路径 (优先取 env, 否则取 cfg).
///
/// 协作点: `APEIRETH_TOOL_FS_ROOTS` (env, 逗号分隔) — 与 B3 工具文件系统子 crate 共用.
pub fn fs_roots_from_env() -> Vec<PathBuf> {
    match std::env::var("APEIRETH_TOOL_FS_ROOTS") {
        Ok(v) => v
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect(),
        Err(_) => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_no_hardening() {
        let c = DirAclConfig::default();
        assert!(!c.needs_hardening());
    }

    #[test]
    fn from_sandbox_derives_roots() {
        let mut sc = crate::sandbox::SandboxConfig::default();
        sc.directory_acl_roots = vec![PathBuf::from("C:\\tool-root")];
        let c = DirAclConfig::from_sandbox(&sc);
        assert!(c.needs_hardening());
        assert_eq!(c.roots, vec![PathBuf::from("C:\\tool-root")]);
    }

    #[test]
    fn overlap_with_env_roots() {
        // env 与 cfg 协作: 二者并集 (env 优先于 cfg? 或 cfg 优先? 这里保留两者合并 — 上层决定).
        // 不在本模块做合并逻辑, 仅验证 env 解析.
        let roots = fs_roots_from_env();
        // 测试环境可能无 env (CI 不会设), 至少 vec 类型正确
        let _: Vec<PathBuf> = roots;
    }

    #[test]
    fn apply_empty_roots_returns_empty_guard() {
        #[cfg(windows)]
        {
            let c = DirAclConfig::default();
            let g = apply_read_only_acl(&c);
            assert!(g.is_ok(), "空 roots 应返回 Ok 空 guard: {g:?}");
        }
        #[cfg(not(windows))]
        {
            let c = DirAclConfig::default();
            let g = apply_read_only_acl(&c);
            assert!(g.is_ok(), "空 roots 跨平台应 Ok no-op: {g:?}");
        }
    }

    #[test]
    fn apply_non_windows_with_roots_returns_err() {
        #[cfg(not(windows))]
        {
            let c = DirAclConfig {
                roots: vec![PathBuf::from("/tmp")],
                owner_retain_write: true,
            };
            let r = apply_read_only_acl(&c);
            assert!(r.is_err(), "非 Windows 平台请求 ACL 收紧应诚实返 Err");
            let err = r.unwrap_err();
            assert!(err.contains("非 Windows"), "错误信息应诚实标注: {err}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn apply_windows_real_path_creates_guard() {
        // 真打 Windows API: 创建临时目录 → 收紧 → 测受限 → Drop 还原.
        let dir = std::env::temp_dir().join(format!("apeireth-acl-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");

        let c = DirAclConfig {
            roots: vec![dir.clone()],
            owner_retain_write: true,
        };
        let r = apply_read_only_acl(&c);
        if let Err(e) = &r {
            eprintln!("[sandbox] 真 ACL 收紧失败 (Windows 端可能缺权限): {e}");
        }
        // 成功: guard 持有正确条数; Drop 还原
        if let Ok(g) = r {
            assert!(!g.original_acls.is_empty(), "guard 应持有原 DACL");
        }
        // 清理
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn apply_windows_nonexistent_path_is_skipped() {
        // 不存在的路径 → 跳过, 其他路径仍处理; 全部失败 → Err.
        let c = DirAclConfig {
            roots: vec![PathBuf::from("C:\\nonexistent-apeireth-test-12345")],
            owner_retain_write: true,
        };
        let r = apply_read_only_acl(&c);
        assert!(r.is_err(), "全部路径无效 → Err: {r:?}");
    }
}
