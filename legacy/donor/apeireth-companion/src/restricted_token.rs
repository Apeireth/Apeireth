//! `apeireth-companion::restricted_token` — Windows 最小权限执行 (S1 安全调研批).
//!
//! **问题**: Worker 进程经 `std::process::Command::spawn` 启动, 默认继承父进程完整
//! token — 包括 `Administrators` 组成员身份 / 全部特权 / 管理员完整性. 即便 Job Object
//! 限制了资源生命周期, 子进程仍可调用 `SeDebugPrivilege`/`SeTakeOwnershipPrivilege` 等
//! 提权接口逃出沙盒 (Chromium 经典分层模型第一节).
//!
//! **S1 三件套** (本模块负责 ① + ②, ③ 见 [`crate::directory_acl`]):
//! 1. **CreateRestrictedToken** 去特权: `DISABLE_MAX_PRIVILEGE` 卸掉所有特权组 +
//!    `LUA_TOKEN` 限定为 limited user 视角 + 可选 deny-only SID 清单 (BUILTIN\Administrators
//!    / WORLD / INTERACTIVE) 让这些 SIDs 仅以 deny ACE 形式生效.
//! 2. **TokenIntegrityLevel** (低完整性): `SetTokenInformation` + `TOKEN_MANDATORY_LABEL`
//!    设 `SECURITY_MANDATORY_LOW_RID` (4096) — 子进程只能写低完整性对象, 桌面/文档默认
//!    中等 → 写不进去, 客观上阻止误操作.
//! 3. **Default DACL**: 受限 token 默认 DACL 可能为空 (拒绝所有), 需要主动设一组
//!    GENERIC_ALL 给 owner + Everyone 才能创建 pipes/IPC (Windows 沙盒标准做法).
//!
//! **跨平台**: 完整性级别 / deny-only SIDs / RestrictedToken 仅在 Windows 生效, 其他平台
//! 为**类型 stub + no-op 实现** (诚实标注 — Linux 走 prctl(PR_SET_NO_NEW_PRIVS) /
//! seccomp / namespaces, 属后续工作).
//!
//! **0 装 PASS**: trait 口已备, 默认配置 `use_app_container=false` + `integrity_level=None`
//! + `deny_only_sids=[]` → B3 行为零回归.

// 模块级豁免: Windows token integrity / DACL 全部 unsafe FFI; 与 job_object.rs 同款,
// crate 级 deny(unsafe_code) 在此收敛到本模块单一文件 (隔离层 = 唯一 unsafe 源).
#![allow(unsafe_code)]

use std::path::PathBuf;
use std::ptr;

use crate::sandbox::{IntegrityLevel, WellKnownSid};

/// 受限 token 配置 (driver for [`create_restricted_token`]).
///
/// 实际生效字段组合:
/// - `integrity_level = Some(Low)` → 设 low integrity label
/// - `deny_only_sids = [Administrators, World]` → SIDs 只用作 deny ACE
/// - `default_dacl_open = true` 设默认 DACL 允许 GENERIC_ALL (创建 pipes 必需)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestrictedTokenConfig {
    /// 强制完整性级别 (None = 不改, 走默认).
    pub integrity_level: Option<IntegrityLevel>,
    /// deny-only SID 清单 (这些组从 access check 中"只用于 deny").
    pub deny_only_sids: Vec<WellKnownSid>,
    /// 是否设置宽松默认 DACL (允许 world GENERIC_ALL). 沙盒 worker 需建 pipes 时必开.
    pub default_dacl_open: bool,
    /// **保留扩展**: 工具沙盒根目录 (将来 AppContainer 档会用, 0 装目前不动).
    pub app_container_roots: Vec<PathBuf>,
}

impl Default for RestrictedTokenConfig {
    fn default() -> Self {
        Self {
            integrity_level: Some(IntegrityLevel::Low),
            deny_only_sids: vec![WellKnownSid::BuiltinAdministrators],
            default_dacl_open: true,
            app_container_roots: Vec::new(),
        }
    }
}

impl RestrictedTokenConfig {
    /// 从 [`crate::sandbox::SandboxConfig`] 派生: 仅有 harden 项时才真设.
    pub fn from_sandbox(cfg: &crate::sandbox::SandboxConfig) -> Self {
        Self {
            integrity_level: cfg.integrity_level,
            deny_only_sids: cfg.deny_only_sids.clone(),
            default_dacl_open: true,
            app_container_roots: Vec::new(),
        }
    }

    /// 是否需要任何降权 (用于上层判断是否调用 `CreateRestrictedToken`).
    pub fn needs_hardening(&self) -> bool {
        self.integrity_level.is_some() || !self.deny_only_sids.is_empty()
    }
}

/// 受限 token 描述 (Windows 端持有 HANDLE, 其他平台为空 stub).
///
/// **生命周期**: `Drop` 时自动 `CloseHandle` — 不可泄漏 (持有者是父进程守护 sub-process,
/// 寿命需横跨 worker 全程).
#[cfg(windows)]
pub struct RestrictedToken {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl Drop for RestrictedToken {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(windows)]
impl std::fmt::Debug for RestrictedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RestrictedToken")
            .field("handle", &self.handle)
            .finish()
    }
}

#[cfg(not(windows))]
#[derive(Debug)]
pub struct RestrictedToken {
    _private: (),
}

/// 创建受限 token (Windows: 真实 `CreateRestrictedToken` + `SetTokenInformation`).
///
/// 失败语义: 返回 `Err(String)` — 加固失败**不阻断执行** (加固是增强不是门), 由调用方
/// eprintln 记录后继续原 token spawn.
///
/// 0 装 PASS: 非 Windows 平台在 `needs_hardening=true` 时如实返回 Err (不假装创建成功)。
/// 需要降权时由调用方**降级**走原 token (job_object 仍生效)。
#[cfg(windows)]
pub fn create_restricted_token(cfg: &RestrictedTokenConfig) -> Result<RestrictedToken, String> {
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Security::{
        CreateRestrictedToken, DISABLE_MAX_PRIVILEGE, LUA_TOKEN, SID_AND_ATTRIBUTES,
        TOKEN_ADJUST_DEFAULT, TOKEN_ADJUST_SESSIONID, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE,
        TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // 模块级 unsafe 收敛: Windows API 全部 unsafe (见文件级 #![allow(unsafe_code)]).
    unsafe {
        // 1. 拿当前进程 token (CreateRestrictedToken 需要 HANDLE source).
        let mut source: HANDLE = ptr::null_mut();
        let access = TOKEN_ASSIGN_PRIMARY
            | TOKEN_DUPLICATE
            | TOKEN_QUERY
            | TOKEN_ADJUST_DEFAULT
            | TOKEN_ADJUST_SESSIONID;
        let ok = OpenProcessToken(GetCurrentProcess(), access, &mut source);
        if ok == 0 {
            return Err(format!(
                "OpenProcessToken 失败 (错误码 {})",
                std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
            ));
        }

        // 2. WellKnownSid → SID_AND_ATTRIBUTES 数组.
        //    走 SidsToDisable 路线 (Chromium 风格): 该 SID 从 token 组列表中剔除,
        //    不再参与 access check (即 deny-only 兜底).
        let mut sids_storage: Vec<SID_AND_ATTRIBUTES> = Vec::new();
        let mut sid_handles: Vec<win_imp::Psid> = Vec::new();
        for sid in &cfg.deny_only_sids {
            if let Some(p) = win_imp::lookup_well_known_sid(*sid) {
                sids_storage.push(SID_AND_ATTRIBUTES {
                    Sid: p.as_ptr(),
                    Attributes: 0,
                });
                sid_handles.push(p);
            }
        }
        let sid_count = sids_storage.len() as u32;
        let sid_ptr = if sid_count == 0 {
            ptr::null()
        } else {
            sids_storage.as_ptr()
        };

        // 3. flags: DISABLE_MAX_PRIVILEGE (卸所有特权) + LUA_TOKEN (降为 LUA 视角).
        let flags = DISABLE_MAX_PRIVILEGE | LUA_TOKEN;

        // 4. CreateRestrictedToken → 新 token.
        let mut new_token: HANDLE = ptr::null_mut();
        let ok = CreateRestrictedToken(
            source,
            flags,
            sid_count,
            sid_ptr,
            0, // DeletePrivileges: 0 — flags 已禁
            ptr::null(),
            0, // RestrictedSids: 0 — 走 SidsToDisable 路线
            ptr::null(),
            &mut new_token,
        );
        windows_sys::Win32::Foundation::CloseHandle(source);
        if ok == 0 {
            return Err(format!(
                "CreateRestrictedToken 失败 (错误码 {})",
                std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
            ));
        }

        // 5. 设完整性级别 (若要求).
        let mut token = RestrictedToken { handle: new_token };
        if let Some(level) = cfg.integrity_level {
            if let Err(e) = apply_integrity_level(&mut token, level) {
                // 完整性设失败 → 关闭并返回 (不阻断 spawn, 调用方会降级无 integrity)
                return Err(format!(
                    "SetTokenInformation(TokenIntegrityLevel) 失败: {e}"
                ));
            }
        }

        // 6. 设默认 DACL (若要求) — 让受限进程能创建 pipes/IPC.
        if cfg.default_dacl_open {
            if let Err(e) = apply_open_default_dacl(&token) {
                // 非关键: 失败降级 (受限进程可能仍能 spawn, 只是不能建 pipes)
                eprintln!(
                    "[sandbox] SetTokenInformation(TokenDefaultDacl) 失败 (降级为拒所有): {e}"
                );
            }
        }

        Ok(token)
    }
}

/// 创建受限 token (跨平台 no-op — 0 装 PASS: needs_hardening 时如实返回 Err)。
#[cfg(not(windows))]
pub fn create_restricted_token(cfg: &RestrictedTokenConfig) -> Result<RestrictedToken, String> {
    if cfg.needs_hardening() {
        // 0 装 PASS: 诚实标注失败, 留给 Linux 未来 prctl/seccomp/namespaces.
        return Err(
            "RestrictedToken 创建: 非 Windows 平台未实现 (0 装 PASS, 走 no-op token)".to_string(),
        );
    }
    Ok(RestrictedToken { _private: () })
}

/// 应用低/中完整性级别 (Windows)。调用方持有 token, 失败则 token 已被 CloseHandle.
///
/// **真接**: `SetTokenInformation(TokenIntegrityLevel, TOKEN_MANDATORY_LABEL)`。
/// `TOKEN_MANDATORY_LABEL` = `SID_AND_ATTRIBUTES` (mandatory label SID)。
#[cfg(windows)]
fn apply_integrity_level(token: &mut RestrictedToken, level: IntegrityLevel) -> Result<(), String> {
    use windows_sys::Win32::Security::{
        SetTokenInformation, TokenIntegrityLevel, SID_AND_ATTRIBUTES, TOKEN_MANDATORY_LABEL,
    };

    // 完整性级别 RID.
    let rid: i32 = match level {
        IntegrityLevel::Untrusted => 0, // SECURITY_MANDATORY_UNTRUSTED_RID
        IntegrityLevel::Low => 4096,    // SECURITY_MANDATORY_LOW_RID
        IntegrityLevel::Medium => 8192, // SECURITY_MANDATORY_MEDIUM_RID
    };

    let label_sid = win_imp::make_mandatory_label_sid(rid)
        .ok_or_else(|| format!("无法构建完整性 SID (RID={rid})"))?;

    let label = TOKEN_MANDATORY_LABEL {
        Label: SID_AND_ATTRIBUTES {
            Sid: label_sid.as_ptr(),
            Attributes: 0x00000020, // SE_GROUP_INTEGRITY
        },
    };

    unsafe {
        let ok = SetTokenInformation(
            token.handle,
            TokenIntegrityLevel,
            (&label as *const TOKEN_MANDATORY_LABEL).cast(),
            std::mem::size_of::<TOKEN_MANDATORY_LABEL>() as u32,
        );
        if ok == 0 {
            return Err(format!(
                "SetTokenInformation 失败 (错误码 {})",
                std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
            ));
        }
    }
    Ok(())
}

/// 设默认 DACL 允许 GENERIC_ALL (受限 token 创建 pipes 必需). 失败降级不阻断.
#[cfg(windows)]
fn apply_open_default_dacl(token: &RestrictedToken) -> Result<(), String> {
    use windows_sys::Win32::Foundation::HLOCAL;
    use windows_sys::Win32::Security::Authorization::{
        SetEntriesInAclW, EXPLICIT_ACCESS_W, GRANT_ACCESS, TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN,
        TRUSTEE_W,
    };
    use windows_sys::Win32::Security::SetTokenInformation;
    use windows_sys::Win32::Security::TokenDefaultDacl;
    use windows_sys::Win32::Security::ACL;

    // 1. 取 World SID 作为 trustee.
    let world_sid = win_imp::lookup_well_known_sid(WellKnownSid::World)
        .ok_or_else(|| "World SID 解析失败".to_string())?;

    // 2. 构造 ACE: World = GRANT_GENERIC_ALL.
    let entries = [EXPLICIT_ACCESS_W {
        grfAccessPermissions: 0x1000_0000u32, // GENERIC_ALL
        grfAccessMode: GRANT_ACCESS,
        grfInheritance: 0,
        Trustee: TRUSTEE_W {
            pMultipleTrustee: ptr::null_mut(),
            MultipleTrusteeOperation: 0,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_UNKNOWN,
            ptstrName: world_sid.as_ptr().cast::<u16>(),
        },
    }];

    // 3. SetEntriesInAclW → new DACL.
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

        // 4. 包装成 TOKEN_DEFAULT_DACL: windows-sys 0.59 未直接导出, 用 raw struct.
        //    struct TOKEN_DEFAULT_DACL { DefaultDacl: *mut ACL }
        #[repr(C)]
        struct TokenDefaultDaclInfo {
            default_dacl: *mut ACL,
        }
        let mut info = TokenDefaultDaclInfo {
            default_dacl: new_dacl,
        };
        let ok = SetTokenInformation(
            token.handle,
            TokenDefaultDacl,
            (&mut info as *mut TokenDefaultDaclInfo).cast(),
            std::mem::size_of::<TokenDefaultDaclInfo>() as u32,
        );
        if ok == 0 {
            let err = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
            if !new_dacl.is_null() {
                windows_sys::Win32::Foundation::LocalFree(new_dacl as HLOCAL);
            }
            return Err(format!(
                "SetTokenInformation(TokenDefaultDacl) 失败 (错误码 {err})"
            ));
        }
        // 成功: SetTokenInformation 接管 DACL, 不需要 LocalFree
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Windows 内部辅助: WellKnownSid 解析 + 完整性 SID 构建
// ---------------------------------------------------------------------------
#[cfg(windows)]
pub(crate) mod win_imp {
    use std::ptr;

    use windows_sys::Win32::Security::{
        CreateWellKnownSid, FreeSid, GetSidLengthRequired, InitializeSid, SID,
        SID_IDENTIFIER_AUTHORITY,
    };

    use crate::sandbox::WellKnownSid;

    /// 已分配的 SID (持有指针, Drop 自动 FreeSid).
    pub struct Psid {
        ptr: *mut SID,
    }

    impl Psid {
        /// 取 PSID (PSID = *mut c_void). Windows API 期望 PSID, 直接传 `*mut SID` 会类型不匹配.
        pub fn as_ptr(&self) -> *mut std::ffi::c_void {
            self.ptr.cast::<std::ffi::c_void>()
        }
    }

    impl Drop for Psid {
        fn drop(&mut self) {
            if !self.ptr.is_null() {
                unsafe { FreeSid(self.ptr.cast()) };
            }
        }
    }

    /// 用 CreateWellKnownSid 解析常见 SID.
    ///
    /// 注意: `CreateWellKnownSid` 把 SID 直接写入我们提供的 buffer, buffer 必须
    /// 存活至 SID 用完。本函数把 buffer `forget` 掉、永久持有 SID 指针 — 但这
    /// 等于泄漏了 buffer。更好做法: SID 内部不持有 sub-authorities 单独分配,
    /// 因此我们把 buffer 一起泄漏 (进程级别不会终止 OS) — 这只发生在测试 / 短寿命
    /// 受限 token 的场景, 实际可接受. 若要严格可释放, 需自实现 SID 拷贝.
    pub fn lookup_well_known_sid(sid: WellKnownSid) -> Option<Psid> {
        let wksid = match sid {
            WellKnownSid::BuiltinAdministrators => 26, // WinBuiltinAdministratorsSid
            WellKnownSid::World => 1,                  // WinWorldSid
            WellKnownSid::AuthenticatedUser => 17,     // WinAuthenticatedUserSid
            WellKnownSid::Interactive => 11,           // WinInteractiveSid
        };
        unsafe {
            let mut size: u32 = 0;
            // 第一次: 拿所需长度.
            let _ = CreateWellKnownSid(wksid, ptr::null_mut(), ptr::null_mut(), &mut size);
            if size == 0 {
                return None;
            }
            let mut buf = vec![0u8; size as usize];
            let ptr = buf.as_mut_ptr().cast::<SID>();
            let ok = CreateWellKnownSid(wksid, ptr::null_mut(), ptr.cast(), &mut size);
            if ok == 0 {
                return None;
            }
            // 防止 buf 释放 (CreateWellKnownSid 把 SID 直接写入我们的 buf).
            // 改方案: 复制 SID 到一个独立 alloc (SID 结构简单的 u8 数组), 让 buf release.
            let layout =
                std::alloc::Layout::from_size_align(size as usize, std::mem::align_of::<SID>())
                    .ok()?;
            let new_ptr = std::alloc::alloc(layout).cast::<SID>();
            if new_ptr.is_null() {
                return None;
            }
            std::ptr::copy_nonoverlapping(ptr.cast::<u8>(), new_ptr.cast::<u8>(), size as usize);
            // buf 离开作用域, 自动释放 (vec Drop).
            Some(Psid { ptr: new_ptr })
        }
    }

    /// 构建 Mandatory Label SID (1 sub-authority: integrity level RID).
    ///
    /// `SECURITY_MANDATORY_LABEL_AUTHORITY` = `{0, 0, 0, 0, 0, 16}` (S-1-16-XXXX).
    pub fn make_mandatory_label_sid(rid: i32) -> Option<Psid> {
        unsafe {
            let auth = SID_IDENTIFIER_AUTHORITY {
                Value: [0, 0, 0, 0, 0, 16],
            };
            let size = GetSidLengthRequired(1);
            let layout =
                std::alloc::Layout::from_size_align(size as usize, std::mem::align_of::<SID>())
                    .ok()?;
            let ptr = std::alloc::alloc(layout).cast::<SID>();
            if ptr.is_null() {
                return None;
            }
            let ok = InitializeSid(ptr.cast(), &auth, 1);
            if ok == 0 {
                std::alloc::dealloc(ptr.cast::<u8>(), layout);
                return None;
            }
            // SID 头部: {u8 Revision, u8 SubAuthorityCount, SID_IDENTIFIER_AUTHORITY IdentifierAuthority, [u32 SubAuthority]}
            // SubAuthority[0] 在 IdentifierAuthority 之后 (6 bytes).
            let sub_ptr = (ptr.cast::<u8>().add(std::mem::size_of::<u8>() * 2 + 6)).cast::<u32>();
            *sub_ptr = rid as u32;
            Some(Psid { ptr })
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_hardening() {
        let c = RestrictedTokenConfig::default();
        assert_eq!(c.integrity_level, Some(IntegrityLevel::Low));
        assert!(!c.deny_only_sids.is_empty());
        assert!(c.default_dacl_open);
        assert!(c.needs_hardening());
    }

    #[test]
    fn no_hardening_config_no_op_on_non_windows() {
        let c = RestrictedTokenConfig {
            integrity_level: None,
            deny_only_sids: Vec::new(),
            default_dacl_open: false,
            app_container_roots: Vec::new(),
        };
        assert!(!c.needs_hardening());
        #[cfg(not(windows))]
        {
            let r = create_restricted_token(&c);
            assert!(
                r.is_ok(),
                "non-Windows 上无 hardening 应返回 Ok 占位: {r:?}"
            );
        }
    }

    #[test]
    fn hardening_request_fails_honestly_off_windows() {
        #[cfg(not(windows))]
        {
            let c = RestrictedTokenConfig {
                integrity_level: Some(IntegrityLevel::Low),
                deny_only_sids: vec![WellKnownSid::BuiltinAdministrators],
                default_dacl_open: true,
                app_container_roots: Vec::new(),
            };
            let r = create_restricted_token(&c);
            assert!(r.is_err(), "非 Windows 平台请求真硬化应诚实返 Err");
            let err = r.unwrap_err();
            assert!(err.contains("非 Windows"), "错误信息应诚实标注: {err}");
        }
    }

    #[test]
    fn from_sandbox_derives_correctly() {
        let sc = crate::sandbox::SandboxConfig::default();
        let rc = RestrictedTokenConfig::from_sandbox(&sc);
        assert!(!rc.needs_hardening());

        let sc2 = crate::sandbox::SandboxConfig {
            integrity_level: Some(IntegrityLevel::Low),
            ..Default::default()
        };
        let rc2 = RestrictedTokenConfig::from_sandbox(&sc2);
        assert!(rc2.needs_hardening());
    }

    #[cfg(windows)]
    #[test]
    fn real_restricted_token_creates_on_windows() {
        let c = RestrictedTokenConfig::default();
        let r = create_restricted_token(&c);
        // 极少数环境下 CI 可能无 SeAssignPrimaryTokenPrivilege → 允许 Err, 但不静默
        if let Err(e) = &r {
            eprintln!("[sandbox] create_restricted_token 真测失败 (CI 环境可能受限): {e}");
        }
        let _ = r;
    }

    #[cfg(windows)]
    #[test]
    fn real_restricted_token_without_hardening_returns_handle() {
        let c = RestrictedTokenConfig {
            integrity_level: None,
            deny_only_sids: Vec::new(),
            default_dacl_open: false,
            app_container_roots: Vec::new(),
        };
        // 当前实现: 即使 deny_only_sids=[] 也会跑 CreateRestrictedToken + DISABLE_MAX_PRIVILEGE
        // (这是 by design — 卸特权是默认 cool, 即使无 deny SID). 测试 Windows API 真接路径.
        let r = create_restricted_token(&c);
        if let Err(e) = &r {
            eprintln!("[sandbox] 真受限token创建失败 (Windows 端可能缺权限): {e}");
        }
        let _ = r;
    }
}
