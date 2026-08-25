use winapi::shared::minwindef::{BOOL, FALSE, LPARAM, TRUE};
use winapi::shared::windef::HWND;
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winuser::{
    AttachThreadInput, BringWindowToTop, CloseDesktop, CloseWindowStation, EnumDesktopWindows,
    GetClassNameW, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    OpenDesktopW, OpenWindowStationW, SetForegroundWindow, SetProcessWindowStation, ShowWindow,
    SW_RESTORE, SW_SHOW, WINSTA_ALL_ACCESS,
};

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn focus_edge_window() -> bool {
    let winsta_name = to_wide("WinSta0");
    let winsta = OpenWindowStationW(winsta_name.as_ptr(), FALSE, WINSTA_ALL_ACCESS);
    if winsta.is_null() {
        return false;
    }
    SetProcessWindowStation(winsta);
    let desk_name = to_wide("Default");
    let desk = OpenDesktopW(desk_name.as_ptr(), 0, FALSE, 0x01FF);
    if desk.is_null() {
        CloseWindowStation(winsta);
        return false;
    }

    struct Finder {
        target: Option<HWND>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let finder = &mut *(lparam as *mut Finder);
        if IsWindowVisible(hwnd) != 0 {
            let mut title_buf = [0u16; 256];
            let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 256);
            if len > 0 {
                let title = String::from_utf16_lossy(&title_buf[..len as usize]);
                if title.contains("Edge") || title.contains("Microsoft​ Edge") || title.contains("bilibili") || title.contains("Live2d") {
                    finder.target = Some(hwnd);
                    return FALSE;
                }
            }
        }
        TRUE
    }

    let mut finder = Finder { target: None };
    EnumDesktopWindows(desk, Some(enum_proc), &mut finder as *mut _ as LPARAM);
    CloseDesktop(desk);
    CloseWindowStation(winsta);

    if let Some(hwnd) = finder.target {
        let fg_hwnd = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg_hwnd, std::ptr::null_mut());
        let cur_thread = GetCurrentThreadId();

        AttachThreadInput(cur_thread, fg_thread, TRUE);
        ShowWindow(hwnd, SW_RESTORE);
        ShowWindow(hwnd, SW_SHOW);
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
        AttachThreadInput(cur_thread, fg_thread, FALSE);
        println!("Successfully brought Edge (HWND {:?}) to foreground!", hwnd);
        true
    } else {
        println!("Edge window not found on desktop");
        false
    }
}

#[test]
#[ignore = "invasive e2e: takes over desktop focus"]
fn test_focus_edge() {
    unsafe {
        let res = focus_edge_window();
        println!("Focus Edge result: {}", res);
    }
}
