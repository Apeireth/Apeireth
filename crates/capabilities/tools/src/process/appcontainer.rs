//! Windows AppContainer 沙箱核心 (W1 P2, 2026-10-10)。
//!
//! 设计依据: `docs/01-architecture/shell-sandbox-lite-design-2026-10-06.md` §2.1。
//!
//! - **文件授权 (grants)**: 仅授予工作区目录 —— AppContainer 默认零访问, 未授权
//!   路径一律拒绝 ("只读全盘"随之消失, 2026-10-06 真机 shell 探针的 win.ini 全文
//!   泄露由此关闭)。
//! - **网络能力**: **不授予任何 capability SID** (无 `INTERNET_CLIENT`) ——
//!   AppContainer 无网络能力时 socket 创建即败, shell 完全断网 (SSRF 无从谈起)。
//! - **降级契约 (fail-closed, 0 装)**: 创建/授予任一步失败 → 上层拒绝执行,
//!   沙箱开时**绝不裸跑**。
//!
//! **探针纪律** (沿 `restricted_launch_capability` 先例): 只有真实 AppContainer
//! 子进程创建成功才报 `Enforced`; 仅凭句柄/注册表不称已实施。
//!
//! ## 真机矩阵定案 (2026-10-10, 12 组合实测, 排障记录见台账 #49)
//!
//! 1. **profile 注册决定成败**: 进程创建校验 AppContainer profile 注册 —— 无
//!    profile 背书的纯派生 SID, `CreateProcessW` 报误导性 "找不到文件"
//!    (os error 2); profile 存在后**同一 SID** 即成功。故 [`AppContainerSandbox::acquire`]
//!    = 建 profile (幂等) → 已存在则派生同一 SID。
//! 2. **可执行文件须绝对路径**: AC 零访问 token 做不了 PATH 搜索 (os error 2),
//!    父进程侧解析 (`windows::resolve_executable_absolute`)。
//! 3. **环境块须完整 + 排序 + `CREATE_UNICODE_ENVIRONMENT`**: AC 进程初始化要
//!    档案基础变量 (缺失报 os error 203); 块须大小写不敏感排序 (乱序报 203);
//!    宽字符块必须带 Unicode 旗 (缺旗按 ANSI 解释报 203)。
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, HANDLE};
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
};
use windows_sys::Win32::Security::{FreeSid, PSID, SECURITY_CAPABILITIES, SID_AND_ATTRIBUTES};
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, InitializeProcThreadAttributeList,
    TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject, CREATE_SUSPENDED,
    EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION,
    PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, STARTUPINFOEXW,
};

use super::{EnforcementLevel, ProcessError};

/// AppContainer 名 (profile 注册 + 确定性 SID 的输入)。
pub const SANDBOX_APPCONTAINER_NAME: &str = "Apeireth.Shell.Sandbox";

/// **AppContainer 沙箱会话**: 持有 SID (拥有者负责释放)。
pub struct AppContainerSandbox {
    sid: PSID,
}

// SAFETY: SID 是一块不透明内存, 仅在 spawn 时以只读方式交给系统;
// Drop 负责释放, 不存在跨线程别名可变。
unsafe impl Send for AppContainerSandbox {}
unsafe impl Sync for AppContainerSandbox {}

impl Drop for AppContainerSandbox {
    fn drop(&mut self) {
        unsafe {
            if !self.sid.is_null() {
                FreeSid(self.sid);
            }
        }
    }
}

impl AppContainerSandbox {
    /// 获取沙箱身份 (**幂等**)。
    ///
    /// 真机矩阵定案 (见模块头): 进程创建**校验 profile 注册** —— 故 ① 先
    /// `CreateAppContainerProfile` 建真 profile (拿返回 SID); ② `ERROR_ALREADY_EXISTS`
    /// (0x800700B7) = profile 已在 → 派生同一 SID (字节等同, 不再动注册表)。
    /// profile 条目生命周期 = 安装级, 不随进程删除 (删除会使既有 ACL 授权失效)。
    pub fn acquire() -> Result<Self, ProcessError> {
        unsafe {
            let name: Vec<u16> = std::ffi::OsStr::new(SANDBOX_APPCONTAINER_NAME)
                .encode_wide()
                .chain(Some(0))
                .collect();
            let display: Vec<u16> = std::ffi::OsStr::new("Apeireth shell sandbox")
                .encode_wide()
                .chain(Some(0))
                .collect();
            let mut sid: PSID = std::ptr::null_mut();
            let hr = CreateAppContainerProfile(
                name.as_ptr(),
                display.as_ptr(),
                display.as_ptr(),
                std::ptr::null(),
                0,
                &mut sid,
            );
            if hr >= 0 && !sid.is_null() {
                return Ok(Self { sid });
            }
            if hr as u32 == 0x8007_00B7u32 {
                let mut derived: PSID = std::ptr::null_mut();
                let hr2 = DeriveAppContainerSidFromAppContainerName(name.as_ptr(), &mut derived);
                if hr2 >= 0 && !derived.is_null() {
                    return Ok(Self { sid: derived });
                }
                return Err(ProcessError::ContainmentFailed(format!(
                    "AppContainer profile exists but SID derive failed: HRESULT=0x{hr2:08x}"
                )));
            }
            Err(ProcessError::ContainmentFailed(format!(
                "CreateAppContainerProfile failed: HRESULT=0x{hr:08x}"
            )))
        }
    }

    /// SID (交给 SECURITY_CAPABILITIES)。
    pub fn sid(&self) -> PSID {
        self.sid
    }

    /// **文件授权**: 给 AppContainer SID 对 `dir` 子树追加完全控制 ACE。
    ///
    /// 合并进既有 DACL (SetEntriesInAclW + SetNamedSecurityInfoW), 不清空原访问
    /// 表 —— 主人/系统对工作区的既有权限不受影响; 未授权路径保持零访问。
    /// 继承 = 容器+对象继承 (子目录/文件自动获得)。
    pub fn grant_directory(&self, dir: &Path) -> Result<(), ProcessError> {
        grant_acl_to_sid(dir, self.sid, FILE_ALL_ACCESS, 0x00000003u32)
    }

    /// 进程创建用 SECURITY_CAPABILITIES (零 capability = 断网)。
    pub fn security_capabilities(&self) -> SECURITY_CAPABILITIES {
        SECURITY_CAPABILITIES {
            AppContainerSid: self.sid,
            Capabilities: std::ptr::null_mut::<SID_AND_ATTRIBUTES>(),
            CapabilityCount: 0,
            Reserved: 0,
        }
    }
}

use windows_sys::Win32::Storage::FileSystem::FILE_ALL_ACCESS;

/// 一次性能力探针: 真实 profile + 真实 AppContainer 子进程创建 (`cmd /d /c exit 0`),
/// 成功才 `Enforced` (0 装: 不实测不称已实施)。
pub fn appcontainer_launch_capability() -> EnforcementLevel {
    static CACHED: OnceLock<EnforcementLevel> = OnceLock::new();
    *CACHED.get_or_init(|| {
        let Ok(sandbox) = AppContainerSandbox::acquire() else {
            return EnforcementLevel::Unsupported;
        };
        if probe_appcontainer_launch(&sandbox) {
            EnforcementLevel::Enforced
        } else {
            EnforcementLevel::Unsupported
        }
    })
}

/// 探针: 创建一个 AppContainer 下的挂起子进程即杀 —— 只证明创建可行。
fn probe_appcontainer_launch(sandbox: &AppContainerSandbox) -> bool {
    probe_appcontainer_launch_detailed(sandbox).is_ok()
}

/// 分步探针 (诊断用): 每步失败带步骤名 —— 0 装要求失败可归因。
pub(crate) fn probe_appcontainer_launch_detailed(
    sandbox: &AppContainerSandbox,
) -> Result<(), String> {
    unsafe {
        let mut sec_caps = sandbox.security_capabilities();
        let Ok(attrs) = AttributeList::new(&mut sec_caps) else {
            return Err("属性表构建失败 (Initialize/UpdateProcThreadAttribute)".into());
        };

        // AC 零访问 token 无 PATH 搜索能力 → 显式系统 cmd.exe 绝对路径
        // (System32 有 ALL APPLICATION PACKAGES 读执行 ACE)。
        let cmd_path: Vec<u16> = std::ffi::OsStr::new(
            &std::env::var("ComSpec")
                .unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".to_string()),
        )
        .encode_wide()
        .chain(Some(0))
        .collect();
        let mut cmd: Vec<u16> = std::ffi::OsStr::new("cmd.exe /d /c exit 0")
            .encode_wide()
            .chain(Some(0))
            .collect();
        let mut siex: STARTUPINFOEXW = std::mem::zeroed();
        siex.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        siex.lpAttributeList = attrs.as_raw();
        let mut pi: PROCESS_INFORMATION = std::mem::zeroed();

        let ret = CreateProcessW(
            cmd_path.as_ptr(),
            cmd.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT,
            std::ptr::null(),
            std::ptr::null(),
            &siex.StartupInfo,
            &mut pi,
        );
        if ret == 0 {
            return Err(format!(
                "CreateProcessW(AppContainer): {}",
                std::io::Error::last_os_error()
            ));
        }
        TerminateProcess(pi.hProcess, 0);
        WaitForSingleObject(pi.hProcess, 5000);
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
        Ok(())
    }
}

/// `PROC_THREAD_ATTRIBUTE_LIST` RAII (唯一作用: 挂 SECURITY_CAPABILITIES)。
///
/// 拥有属性表缓冲与列表本体; Drop 时先删列表再释放缓冲 (系统要求缓冲活得比
/// `DeleteProcThreadAttributeList` 长)。
pub(crate) struct AttributeList {
    raw: LPPROC_THREAD_ATTRIBUTE_LIST,
    buf: *mut usize,
    words: usize,
}

impl AttributeList {
    /// 初始化属性表并把 SECURITY_CAPABILITIES 挂上 (1 个属性)。
    ///
    /// # Safety
    /// `security_capabilities` 必须在 `CreateProcessW` 完成前保持有效且不被移动
    /// (系统在 CreateProcessW 内部读取它)。调用方以栈上 `mut` 变量满足之。
    pub(crate) unsafe fn new(
        security_capabilities: *mut SECURITY_CAPABILITIES,
    ) -> Result<Self, ProcessError> {
        let mut size: usize = 0;
        InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut size);
        if size == 0 {
            return Err(ProcessError::ContainmentFailed(
                "InitializeProcThreadAttributeList sizing failed".into(),
            ));
        }
        // 按指针对齐分配 (Vec<u8> 不能保证)。
        let words = (size + std::mem::size_of::<usize>() - 1) / std::mem::size_of::<usize>();
        let mut storage = vec![0usize; words];
        let buf = storage.as_mut_ptr();
        let raw = buf as LPPROC_THREAD_ATTRIBUTE_LIST;
        if InitializeProcThreadAttributeList(raw, 1, 0, &mut size) == 0 {
            return Err(ProcessError::ContainmentFailed(
                "InitializeProcThreadAttributeList failed".into(),
            ));
        }
        let ok = UpdateProcThreadAttribute(
            raw,
            0,
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            security_capabilities.cast::<c_void>(),
            std::mem::size_of::<SECURITY_CAPABILITIES>(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if ok == 0 {
            DeleteProcThreadAttributeList(raw);
            return Err(ProcessError::ContainmentFailed(
                "UpdateProcThreadAttribute(SECURITY_CAPABILITIES) failed".into(),
            ));
        }
        std::mem::forget(storage); // 所有权进 Self, Drop 重建释放
        Ok(Self { raw, buf, words })
    }

    pub(crate) fn as_raw(&self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.raw
    }
}

// SAFETY: 属性表由本模块独占, Drop 定序释放。
unsafe impl Send for AttributeList {}
unsafe impl Sync for AttributeList {}

impl Drop for AttributeList {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                DeleteProcThreadAttributeList(self.raw);
            }
            if !self.buf.is_null() {
                drop(Vec::from_raw_parts(self.buf, self.words, self.words));
            }
        }
    }
}

/// 给 `path` 追加 `sid` 的 `permissions` ACE (合并进既有 DACL, 保留原访问表)。
fn grant_acl_to_sid(
    path: &Path,
    sid: PSID,
    permissions: u32,
    inheritance: u32,
) -> Result<(), ProcessError> {
    use windows_sys::Win32::Security::Authorization::{
        GetNamedSecurityInfoW, SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W,
        GRANT_ACCESS, SE_FILE_OBJECT, TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
    };
    use windows_sys::Win32::Security::{ACL, DACL_SECURITY_INFORMATION};

    unsafe {
        let path_w: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut old_acl: *mut ACL = std::ptr::null_mut();
        let mut sd: HANDLE = std::ptr::null_mut();
        let status = GetNamedSecurityInfoW(
            path_w.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut old_acl,
            &mut sd,
        );
        if status != 0 {
            return Err(ProcessError::ContainmentFailed(format!(
                "GetNamedSecurityInfoW failed: {status}"
            )));
        }

        let mut access = std::mem::zeroed::<EXPLICIT_ACCESS_W>();
        access.grfAccessPermissions = permissions;
        access.grfAccessMode = GRANT_ACCESS;
        access.grfInheritance = inheritance;
        access.Trustee.TrusteeForm = TRUSTEE_IS_SID;
        access.Trustee.TrusteeType = TRUSTEE_IS_UNKNOWN;
        access.Trustee.ptstrName = sid.cast::<u16>();

        let mut new_acl: *mut ACL = std::ptr::null_mut();
        let err = SetEntriesInAclW(1, &access, old_acl, &mut new_acl);
        if err != 0 {
            LocalFree(sd);
            return Err(ProcessError::ContainmentFailed(format!(
                "SetEntriesInAclW failed: {err}"
            )));
        }
        let status = SetNamedSecurityInfoW(
            path_w.as_ptr() as *mut u16,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            new_acl,
            std::ptr::null_mut(),
        );
        LocalFree(new_acl.cast());
        LocalFree(sd);
        if status != 0 {
            return Err(ProcessError::ContainmentFailed(format!(
                "SetNamedSecurityInfoW failed: {status}"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// W1 §2.5 授予/拒绝矩阵前置: SID 获取必须成 (真机), 失败带 HRESULT 报出。
    #[test]
    fn sandbox_acquire_is_idempotent() {
        if let Err(e) = AppContainerSandbox::acquire() {
            panic!("AppContainerSandbox::acquire 失败: {e}");
        }
        // 幂等: 同名 profile 二次获取 (ALREADY_EXISTS → 派生) 也必须成。
        if let Err(e) = AppContainerSandbox::acquire() {
            panic!("二次 acquire 失败 (幂等路径): {e}");
        }
    }

    /// 探针口径一致性: 不实测不称 Enforced —— 报告值与真实探针必须一致。
    #[test]
    fn probe_result_matches_capability_report() {
        let level = appcontainer_launch_capability();
        let Ok(sandbox) = AppContainerSandbox::acquire() else {
            assert_eq!(level, EnforcementLevel::Unsupported);
            return;
        };
        assert_eq!(
            level == EnforcementLevel::Enforced,
            probe_appcontainer_launch(&sandbox),
            "能力报告与真实探针不一致 (0 装红线)"
        );
    }

    /// 分步探针必须在可实施主机上通过 (失败报出具体步骤, 供 0 装归因)。
    #[test]
    fn probe_appcontainer_launch_succeeds_with_step_attribution() {
        let sandbox = AppContainerSandbox::acquire().expect("acquire");
        probe_appcontainer_launch_detailed(&sandbox).expect("分步探针失败 (含步骤名)");
    }

    /// W1 §2.5 授予矩阵: 工作区目录授予必须成功且幂等 (合并 ACE, 不清原表)。
    #[test]
    fn grant_directory_succeeds_and_is_idempotent() {
        let Ok(sandbox) = AppContainerSandbox::acquire() else {
            return;
        };
        let dir = std::env::temp_dir().join(format!("apeireth-ac-grant-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        sandbox.grant_directory(&dir).expect("授予应成功");
        sandbox.grant_directory(&dir).expect("重复授予应幂等");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
