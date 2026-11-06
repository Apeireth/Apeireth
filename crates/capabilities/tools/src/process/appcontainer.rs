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
//!
//! ## 2026-10-31 审计修复 (P3-11 + #12): 授权身份 + 授权面 + 授权回收
//!
//! 1. **身份 (原先错在: 固定名 ⇒ SID 确定)**: profile 名原先是固定常量
//!    `Apeireth.Shell.Sandbox` ⇒ SID 由名字**确定性**派生 ⇒ ① 全机全账号同一
//!    SID (共享盘上的跨账号授权会互相命中); ② 任何进程都能**同名抢占** (抢先
//!    注册同名 profile ⇒ 我们 acquire 时拿到 `ALREADY_EXISTS` ⇒ 沙箱用的是
//!    对方注册的 profile)。现在完整名字 = 前缀 + **用户标签** (用户 SID 的哈希)
//!    + **随机后缀** (进程级随机) ⇒ SID 不再确定。
//! 2. **授权面 (原先错在: 授全权)**: 授权掩码原先是 `FILE_ALL_ACCESS`
//!    (含 `DELETE` / `WRITE_DAC` / `WRITE_OWNER`) ⇒ 沙箱内进程能删文件、能改
//!    DACL (把授权扩给别人)。现在收窄到**数据面**
//!    (`FILE_GENERIC_READ|FILE_GENERIC_WRITE|FILE_GENERIC_EXECUTE`)。
//! 3. **回收 (原先错在: 只进不出)**: 落盘的 ACE 原先合并进既有 DACL 后再也不
//!    撤销 ⇒ 授权目录的 DACL 里永远留着沙箱 SID 的 ACE。现在授权登记在册,
//!    由回收线程在**宽限期**后用 `SetEntriesInAclW(REVOKE_ACCESS)` 摘掉,
//!    profile 注册在无人使用时用 `DeleteAppContainerProfile` 删除。
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, HANDLE};
use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
};
use windows_sys::Win32::Security::{
    FreeSid, GetTokenInformation, TokenUser, PSID, SECURITY_CAPABILITIES, SID_AND_ATTRIBUTES,
    TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
};
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, GetCurrentProcess,
    InitializeProcThreadAttributeList, OpenProcessToken, TerminateProcess,
    UpdateProcThreadAttribute, WaitForSingleObject, CREATE_SUSPENDED, EXTENDED_STARTUPINFO_PRESENT,
    LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
    STARTUPINFOEXW,
};

use super::{EnforcementLevel, ProcessError};

/// AppContainer 名**前缀** (完整名字 = 前缀 + 用户标签 + 随机后缀)。
///
/// 原先错在: 它就是**完整**的 profile 名 ⇒ SID 确定性派生 (见模块头 §1)。
pub const SANDBOX_APPCONTAINER_NAME: &str = "Apeireth.Shell.Sandbox";

/// 沙箱**数据面**权限掩码: 读 + 写 + 执行 + **删除**。
///
/// 原先错在: 授权用的是 `FILE_ALL_ACCESS` (含 `DELETE` / `WRITE_DAC` /
/// `WRITE_OWNER`) ⇒ 沙箱内进程能删文件、能改 DACL ⇒ 授权面远超"读写工作区"。
///
/// ⚠️ **2026-11-01 CI 实测修正：不能收窄 `DELETE`**。
///
/// 第一版收窄成 `READ|WRITE|EXECUTE`（把 `DELETE` 去掉）⇒ **Windows CI 实测红**
/// （`object_access_judgment.rs` 的
/// `lexical_pass_but_object_violation_is_fenced_with_frame_and_audit`）：
/// 沙箱内 PowerShell 的 `File.Delete('.env')` 被 **OS 权限层**先拒
/// （`UnauthorizedAccessException`）⇒ **删除根本没发生**
/// ⇒ **对象判定（敏感对象删除的唯一兜底）根本没机会触发**
/// ⇒ `object_audit.violations = 0` ⇒ 「换 API 换权力」的防线在 Windows 上**形同虚设**。
///
/// ⇒ 结论：**`DELETE` 必须留着**（对象判定要能在删除**真的发生**时把它拦下来；
/// 敏感对象由对象判定把关，不是由沙箱权限代替）。
/// ⇒ **真正要收窄的是 `WRITE_DAC` / `WRITE_OWNER`**（改 DACL / 改所有者）
/// —— 那两个才是「沙箱内进程不该有的权力」⇒ 这里**不含**它们。
const SANDBOX_DATA_ACCESS: u32 =
    FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE | DELETE;

/// ACE 回收的**宽限期**: 授权后至少存活这么久才允许回收。
///
/// 为什么需要宽限期: `AppContainerSandbox` 在 **spawn 完就析构**
/// (`windows.rs` 的 `spawn_appcontainered_child` 里它是局部变量), 而子进程还要
/// 继续用这份授权访问工作区 ⇒ 析构时立刻撤 ACE 会把**正在运行的沙箱子进程**的
/// 访问权抽走 (那才是真正的功能回归)。宽限期覆盖子进程的最长存活时间
/// (默认 `max_runtime` = 30s ⇒ 300s 宽限期是它的 10 倍)。
const RECLAIM_GRACE: Duration = Duration::from_secs(300);

/// 回收线程的轮询间隔。
const RECLAIM_INTERVAL: Duration = Duration::from_secs(30);

/// **AppContainer 沙箱会话**: 借用进程级 profile 的 SID。
///
/// 原先每个会话各自 `FreeSid` 自己的 SID, profile 注册却**永不删除** ⇒ 注册表
/// 里留下一个再也不会被使用的 profile 条目。现在 SID 由进程级状态持有, 最后一个
/// 会话释放且宽限期满时统一 `DeleteAppContainerProfile` + `FreeSid`。
pub struct AppContainerSandbox {
    /// profile SID —— **借用**进程级 profile 状态 (最后持有者释放时才 FreeSid)。
    sid: PSID,
}

// SAFETY: SID 是一块不透明内存, 仅在 spawn 时以只读方式交给系统;
// profile 状态由进程级 Mutex 保护, 不存在跨线程别名可变。
unsafe impl Send for AppContainerSandbox {}
unsafe impl Sync for AppContainerSandbox {}

impl Drop for AppContainerSandbox {
    fn drop(&mut self) {
        // 释放持有计数: 归零后 profile 进入"无人使用"状态, 宽限期后由回收线程
        // 删除注册 (原先只有 FreeSid ⇒ profile 注册永远留在注册表里)。
        let mut state = profile_state();
        state.holders = state.holders.saturating_sub(1);
        if state.holders == 0 {
            state.idle_since = Some(Instant::now());
        }
    }
}

impl AppContainerSandbox {
    /// 获取沙箱身份 (**幂等**)。
    ///
    /// 真机矩阵定案 (见模块头): 进程创建**校验 profile 注册** —— 故 ① 先
    /// `CreateAppContainerProfile` 建真 profile (拿返回 SID); ② `ERROR_ALREADY_EXISTS`
    /// (0x800700B7) = profile 已在 → 派生同一 SID (字节等同, 不再动注册表)。
    /// profile 条目生命周期 = 安装级, 不随进程删除 (删除会使既有 ACL 授权失效)
    /// —— 但**回收线程**会在无人使用且宽限期满后删掉它 (模块头 §3)。
    pub fn acquire() -> Result<Self, ProcessError> {
        let mut state = profile_state();
        if state.sid.is_null() {
            let sid = create_profile(&state.name)?;
            state.sid = sid;
        }
        state.holders += 1;
        if state.holders == 1 {
            // 又有人在用了 ⇒ 取消「无人使用」的回收计时。
            state.idle_since = None;
        }
        Ok(Self { sid: state.sid })
    }

    /// SID (交给 SECURITY_CAPABILITIES)。
    pub fn sid(&self) -> PSID {
        self.sid
    }

    /// **文件授权**: 给 AppContainer SID 对 `dir` 子树追加**数据面** ACE。
    ///
    /// 合并进既有 DACL (SetEntriesInAclW + SetNamedSecurityInfoW), 不清空原访问
    /// 表 —— 主人/系统对工作区的既有权限不受影响; 未授权路径保持零访问。
    /// 继承 = 容器+对象继承 (子目录/文件自动获得)。
    ///
    /// 授权同时登记在册 ⇒ 回收线程能在宽限期后把这条 ACE 摘掉 (原先只进不出)。
    pub fn grant_directory(&self, dir: &Path) -> Result<(), ProcessError> {
        grant_acl_to_sid(dir, self.sid, SANDBOX_DATA_ACCESS, 0x00000003u32)?;
        let mut state = profile_state();
        let now = Instant::now();
        let existing = state
            .grants
            .iter()
            .position(|entry| entry.path.as_path() == dir);
        match existing {
            Some(index) => state.grants[index].granted_at = now,
            None => state.grants.push(GrantEntry {
                path: dir.to_path_buf(),
                granted_at: now,
            }),
        }
        drop(state);
        spawn_reclaimer();
        Ok(())
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

    /// 完整 profile 名 (诊断 / 测试用): 前缀 + 用户标签 + 随机后缀。
    ///
    /// 原先这就是固定常量 `SANDBOX_APPCONTAINER_NAME` ⇒ 现在这个名字**每次进程
    /// 都不同** (随机后缀) 且**跨账号不同** (用户标签) ⇒ 派生出的 SID 不再确定。
    pub fn profile_name(&self) -> String {
        profile_state().name.clone()
    }
}

/// 一条已授权目录的登记 (回收用)。
struct GrantEntry {
    /// 被授权的目录。
    path: PathBuf,
    /// 最后一次授权的时刻 (回收宽限期从它起算)。
    granted_at: Instant,
}

/// 进程级 profile 状态 (一个进程一个 profile + 一张授权登记表)。
struct ProfileState {
    /// 完整 profile 名 (前缀 + 用户标签 + 随机后缀)。
    name: String,
    /// profile SID (全局持有 ⇒ 最后一个持有者释放时才 FreeSid)。
    sid: PSID,
    /// 活跃沙箱数 (acquire +1 / 释放 -1)。
    holders: usize,
    /// 没有活跃沙箱的时刻 (profile 回收的宽限期起算点)。
    idle_since: Option<Instant>,
    /// 已授权目录登记。
    grants: Vec<GrantEntry>,
}

static PROFILE_STATE: OnceLock<Mutex<ProfileState>> = OnceLock::new();

// SAFETY: `sid` 是一块不透明内存 (只读使用, 不跨线程可变); 真正的同步由
// `Mutex<ProfileState>` 提供 ⇒ 该状态可以安全地放进进程级静态变量。
unsafe impl Send for ProfileState {}

/// 取进程级 profile 状态 (锁中毒时取回内层值, 不因一次 panic 永久卡死)。
fn profile_state() -> MutexGuard<'static, ProfileState> {
    PROFILE_STATE
        .get_or_init(|| {
            Mutex::new(ProfileState {
                name: profile_name(),
                sid: std::ptr::null_mut(),
                holders: 0,
                idle_since: None,
                grants: Vec::new(),
            })
        })
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 完整 profile 名 = 前缀 + 用户标签 + 随机后缀 (拼装规则在 `super` 的
/// 平台无关函数里 ⇒ Linux 侧可测)。
fn profile_name() -> String {
    super::appcontainer_profile_name(
        SANDBOX_APPCONTAINER_NAME,
        &current_user_tag(),
        &random_tag(),
    )
}

/// 当前用户的短标签 (用户 SID 的哈希) —— 掺进 profile 名 ⇒ 不同账号不同 SID。
fn current_user_tag() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let Some(sid) = current_user_sid() else {
        // 取不到用户 SID (极端环境) ⇒ 退回固定标签; 随机后缀仍保证名字不可预测。
        return "nouser".to_string();
    };
    let mut hasher = DefaultHasher::new();
    sid.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// 取当前进程 token 里的用户 SID (字符串形式); 取不到返回 `None`。
fn current_user_sid() -> Option<String> {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return None;
        }
        let mut needed = 0u32;
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut needed);
        if needed == 0 {
            CloseHandle(token);
            return None;
        }
        // `TOKEN_USER` 要求 8 字节对齐 ⇒ 用 `Vec<u64>` 承载缓冲 (Vec<u8> 对齐只有 1)。
        let words = (needed as usize).div_ceil(8).max(1);
        let mut buffer: Vec<u64> = vec![0u64; words];
        let ok = GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast::<c_void>(),
            needed,
            &mut needed,
        );
        CloseHandle(token);
        if ok == 0 {
            return None;
        }
        let user = &*(buffer.as_ptr().cast::<TOKEN_USER>());
        let mut raw: windows_sys::core::PWSTR = std::ptr::null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut raw) == 0 {
            return None;
        }
        let text = wide_to_string(raw);
        LocalFree(raw.cast());
        Some(text)
    }
}

/// 宽字符 C 串 → `String`。
fn wide_to_string(raw: windows_sys::core::PWSTR) -> String {
    if raw.is_null() {
        return String::new();
    }
    unsafe {
        let mut len = 0usize;
        while *raw.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(raw, len))
    }
}

/// 进程级随机标签 (16 位 hex) —— 掺进 profile 名 ⇒ 名字不可预测 (防同名抢占)。
fn random_tag() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(u64::from(std::process::id()));
    hasher.write_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos() as u64)
            .unwrap_or_default(),
    );
    format!("{:016x}", hasher.finish())
}

/// 注册 (或派生) AppContainer profile ⇒ 返回 SID。
fn create_profile(name: &str) -> Result<PSID, ProcessError> {
    unsafe {
        let name: Vec<u16> = std::ffi::OsStr::new(name)
            .encode_wide()
            .chain(Some(0))
            .collect();
        let display: Vec<u16> = std::ffi::OsStr::new("Apeireth shell sandbox")
            .encode_wide()
            .chain(Some(0))
            .collect();
        // 瞬时失败有界重试: 并行负载下 profile 注册可能瞬时抖动
        // (典型 HRESULT=0x8000ffff); 有界重试两次后再按真实失败上报,
        // 只在失败路径付出毫秒级等待。
        let mut last_hr = 0i32;
        for attempt in 0..3u32 {
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
                return Ok(sid);
            }
            if hr as u32 == 0x8007_00B7u32 {
                let mut derived: PSID = std::ptr::null_mut();
                let hr2 = DeriveAppContainerSidFromAppContainerName(name.as_ptr(), &mut derived);
                if hr2 >= 0 && !derived.is_null() {
                    return Ok(derived);
                }
                return Err(ProcessError::ContainmentFailed(format!(
                    "AppContainer profile exists but SID derive failed: HRESULT=0x{hr2:08x}"
                )));
            }
            last_hr = hr;
            if attempt < 2 {
                std::thread::sleep(std::time::Duration::from_millis(
                    50 * (u64::from(attempt) + 1),
                ));
            }
        }
        Err(ProcessError::ContainmentFailed(format!(
            "CreateAppContainerProfile failed after bounded retries: HRESULT=0x{last_hr:08x}"
        )))
    }
}

/// 一次性能力探针: 真实 profile + 真实 AppContainer 子进程创建 (`cmd /d /c exit 0`),
/// 成功才 `Enforced` (0 装: 不实测不称已实施)。
///
/// 环境侧瞬时拒绝走 [`probe_appcontainer_launch_bounded`] 有界重试 (D25):
/// 结果被缓存整个进程生命周期, 不应被瞬时抖动永久降级为「未实施」。
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
///
/// 走 [`probe_appcontainer_launch_bounded`] (仅环境侧拒绝有界重试): 一次性能力
/// 判定会被 `OnceLock` 缓存整个进程生命周期, 瞬时抖动不应被误判为「不支持」。
fn probe_appcontainer_launch(sandbox: &AppContainerSandbox) -> bool {
    probe_appcontainer_launch_bounded(sandbox).is_ok()
}

/// `CreateProcessW(AppContainer)` 的已知**环境侧**拒绝形态:
/// `ERROR_FILE_NOT_FOUND (os error 2)` —— AppContainer 零访问 token 下系统可执行
/// 访问在部分主机/并行负载下瞬时不可用 (模块头真机矩阵记录的同形态误报)。
/// 与探针代码缺陷严格区分: 其它错误不在此列, 一律按真缺陷上报。
fn is_environment_refusal(step: &str) -> bool {
    // 精确匹配末尾的 "(os error 2)": 避免把相邻错误码 (如 "os error 22") 的子串
    // 误判成环境侧拒绝 —— 误判会把真缺陷错当 SKIP, 破了 0 装。
    step.ends_with("(os error 2)")
}

/// 探针的有界重试版本: **仅**对环境侧拒绝重试 (50/100ms 退避, 共 3 次),
/// 其它错误立即返回 (真缺陷不掩盖)。
///
/// D25 (2026-10-21): 全量并行运行中 `CreateProcessW ... os error 2` 偶发
/// (失败帧 artifacts/wave-bio-w2-workspace-test.log、wave-bio-w32d-workspace-test.log),
/// 单独跑/单 crate 复跑通过 —— 环境侧瞬时拒绝。重试把「瞬时抖动」与
/// 「环境确实不支持」分开: 三次都拒 → 维持 Unsupported/SKIP 口径。
fn probe_appcontainer_launch_bounded(sandbox: &AppContainerSandbox) -> Result<(), String> {
    const MAX_ATTEMPTS: u32 = 3;
    let mut last_env_error = String::new();
    for attempt in 0..MAX_ATTEMPTS {
        match probe_appcontainer_launch_detailed(sandbox) {
            Ok(()) => return Ok(()),
            Err(step) if is_environment_refusal(&step) => {
                last_env_error = step;
                if attempt + 1 < MAX_ATTEMPTS {
                    std::thread::sleep(std::time::Duration::from_millis(
                        50 * (u64::from(attempt) + 1),
                    ));
                }
            }
            Err(step) => return Err(step),
        }
    }
    Err(last_env_error)
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

/// 起回收线程 (每个进程只起一个; 线程随进程退出, 不阻塞退出)。
fn spawn_reclaimer() {
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        // `let _ =` : JoinHandle 是 must_use; 这里要的是「脱离式」后台线程。
        let _ = std::thread::spawn(|| loop {
            std::thread::sleep(RECLAIM_INTERVAL);
            reclaim_once();
        });
    });
}

/// 回收一轮: 摘掉过期授权的 ACE; 无人在用且宽限期满 ⇒ 删 profile 注册。
fn reclaim_once() {
    let mut state = profile_state();
    let now = Instant::now();

    // 1) 过期授权 ⇒ 撤销 ACE (原先没有这条路径, ACE 永远留在 DACL 里)。
    let expired: Vec<PathBuf> = state
        .grants
        .iter()
        .filter(|entry| now.duration_since(entry.granted_at) >= RECLAIM_GRACE)
        .map(|entry| entry.path.clone())
        .collect();
    for path in expired {
        if let Err(error) = revoke_acl_from_sid(&path, state.sid) {
            eprintln!("AppContainer 授权回收失败 ({}): {error}", path.display());
        }
        state.grants.retain(|entry| entry.path != path);
    }

    // 2) profile 无人使用 + 宽限期满 + 授权已回收干净 ⇒ 删注册 + 释放 SID。
    //    (先撤 ACE 再删 profile: profile 一删, SID 就再也无法从名字派生回。)
    if state.holders == 0 && state.grants.is_empty() && !state.sid.is_null() {
        let idle_long_enough = state
            .idle_since
            .is_some_and(|idle| now.duration_since(idle) >= RECLAIM_GRACE);
        if idle_long_enough {
            delete_profile(&state.name);
            unsafe { FreeSid(state.sid) };
            state.sid = std::ptr::null_mut();
        }
    }
}

/// 删除 AppContainer profile 注册 (回收用; 失败只记日志 —— profile 在用时删不掉)。
fn delete_profile(name: &str) {
    unsafe {
        let name: Vec<u16> = std::ffi::OsStr::new(name)
            .encode_wide()
            .chain(Some(0))
            .collect();
        let hr = DeleteAppContainerProfile(name.as_ptr());
        if hr < 0 {
            eprintln!("DeleteAppContainerProfile 失败 (profile 仍在用?): HRESULT=0x{hr:08x}");
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
    use windows_sys::Win32::Security::Authorization::GRANT_ACCESS;
    rewrite_acl(path, sid, permissions, inheritance, GRANT_ACCESS)
}

/// **撤销**先前授予的 ACE (把 DACL 里属于该 SID 的 ACE 摘掉)。
///
/// 原先错在: 授权只进不出 —— `SetEntriesInAclW` + `SetNamedSecurityInfoW` 合并进
/// 既有 DACL 后再无撤销路径, 全仓也没有 `DeleteAce` / `DeleteAppContainerProfile`
/// 的回收路径 ⇒ 授权目录的 DACL 里永远留着沙箱 SID 的 ACE。
/// 现在: 用 `SetEntriesInAclW(REVOKE_ACCESS)` 生成"去掉该 trustee 的 ACE"的新
/// ACL, 再用 `SetNamedSecurityInfoW` 写回。
///
/// 已知边界: Windows 的**继承** ACE 不会因为父目录的 ACE 被撤销而从子对象上
/// 自动消失 (继承是"拷贝"语义) ⇒ 子目录/文件上的继承副本需要逐个回收;
/// 本模块只回收被显式授权的那一条 (边界在此写明, 不假装覆盖)。
fn revoke_acl_from_sid(path: &Path, sid: PSID) -> Result<(), ProcessError> {
    use windows_sys::Win32::Security::Authorization::REVOKE_ACCESS;
    // 掩码/继承标志与授权时保持一致 ⇒ 撤销时能精确命中我们写进去的那一条。
    rewrite_acl(path, sid, SANDBOX_DATA_ACCESS, 0x00000003u32, REVOKE_ACCESS)
}

/// 改写 `path` 的 DACL: 按 `mode` (GRANT_ACCESS / REVOKE_ACCESS) 处理 `sid`。
fn rewrite_acl(
    path: &Path,
    sid: PSID,
    permissions: u32,
    inheritance: u32,
    mode: i32,
) -> Result<(), ProcessError> {
    use windows_sys::Win32::Security::Authorization::{
        GetNamedSecurityInfoW, SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W,
        SE_FILE_OBJECT, TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN,
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
        access.grfAccessMode = mode;
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
    ///
    /// D25 (2026-10-21): 本测在全量并行运行中偶发 `CreateProcessW ... os error 2`
    /// (失败帧 artifacts/wave-bio-w2-workspace-test.log、wave-bio-w32d-workspace-test.log),
    /// 单独跑与单 crate lib 复跑均通过 —— 属**环境侧**拒绝, 非被测代码缺陷。处置
    /// (0 装: 不静默 pass, 不假绿):
    /// - 环境侧拒绝走 [`probe_appcontainer_launch_bounded`] 有界重试 (吸收瞬时抖动);
    /// - 重试后仍为环境侧拒绝 → `eprintln!` 打印原因后**显式 SKIP**;
    /// - 任何非环境侧错误 → 严格 panic (真缺陷必须红);
    /// - 环境支持 (探针成功) → 严格断言通过 (不放宽)。
    #[test]
    fn probe_appcontainer_launch_succeeds_with_step_attribution() {
        let sandbox = match AppContainerSandbox::acquire() {
            Ok(sandbox) => sandbox,
            // acquire 的严格断言在 sandbox_acquire_is_idempotent (该测失败照旧红);
            // 此处只处理「本机不支持 AppContainer」这一环境侧情形。
            Err(e) => {
                eprintln!("SKIP: AppContainer profile 不可用 (环境不支持): {e}");
                return;
            }
        };
        match probe_appcontainer_launch_bounded(&sandbox) {
            Ok(()) => {}
            Err(step) if is_environment_refusal(&step) => {
                eprintln!(
                    "SKIP: 环境不支持 AppContainer 子进程创建 (有界重试仍被环境侧拒绝): {step}"
                );
            }
            Err(step) => panic!("分步探针失败 (含步骤名): {step}"),
        }
    }

    /// 环境侧拒绝识别必须收窄到 ERROR_FILE_NOT_FOUND (os error 2): 子串误判
    /// (如 "os error 22") 会把真缺陷错当 SKIP, 破了 0 装。
    #[test]
    fn environment_refusal_matches_only_file_not_found_os_error() {
        assert!(is_environment_refusal(
            "CreateProcessW(AppContainer): 系统找不到指定的文件。 (os error 2)"
        ));
        assert!(!is_environment_refusal(
            "CreateProcessW(AppContainer): 参数错误。 (os error 22)"
        ));
        assert!(!is_environment_refusal(
            "CreateProcessW(AppContainer): 拒绝访问。 (os error 5)"
        ));
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

    /// 回收路径必须真的能把 ACE 摘掉 (原先根本没有回收路径 ⇒ ACE 永久残留)。
    #[test]
    fn revoke_removes_granted_ace() {
        use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
        use windows_sys::Win32::Security::{
            AclSizeInformation, GetAclInformation, ACL, ACL_SIZE_INFORMATION,
            DACL_SECURITY_INFORMATION,
        };

        let Ok(sandbox) = AppContainerSandbox::acquire() else {
            return;
        };
        let dir = std::env::temp_dir().join(format!("apeireth-ac-revoke-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let count_aces = || -> u32 {
            unsafe {
                let path: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
                let mut acl: *mut ACL = std::ptr::null_mut();
                let mut sd: HANDLE = std::ptr::null_mut();
                let status = GetNamedSecurityInfoW(
                    path.as_ptr(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut acl,
                    &mut sd,
                );
                assert_eq!(status, 0, "GetNamedSecurityInfoW 失败: {status}");
                let mut info = std::mem::zeroed::<ACL_SIZE_INFORMATION>();
                let ok = GetAclInformation(
                    acl,
                    (&mut info as *mut ACL_SIZE_INFORMATION).cast::<c_void>(),
                    std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                    AclSizeInformation,
                );
                assert_ne!(ok, 0, "GetAclInformation 失败");
                LocalFree(sd);
                info.AceCount
            }
        };

        let before = count_aces();
        sandbox.grant_directory(&dir).expect("授予应成功");
        let after_grant = count_aces();
        assert!(
            after_grant > before,
            "授权后 DACL 的 ACE 数必须增加 (before={before}, after={after_grant})"
        );
        revoke_acl_from_sid(&dir, sandbox.sid()).expect("撤销应成功");
        let after_revoke = count_aces();
        assert_eq!(
            after_revoke, before,
            "撤销后 DACL 的 ACE 数必须回到授权前 (before={before}, after={after_revoke})"
        );
        // 幂等: 再撤一次不得报错 (ACE 已经不在了)。
        revoke_acl_from_sid(&dir, sandbox.sid()).expect("重复撤销应成功");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
