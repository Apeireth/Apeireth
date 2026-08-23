use winapi::shared::minwindef::{BOOL, FALSE, LPARAM, TRUE};
use winapi::shared::windef::HWND;
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winuser::{
    AttachThreadInput, BringWindowToTop, CloseDesktop, CloseWindowStation, EnumDesktopWindows,
    GetForegroundWindow, GetWindowPlacement, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible, OpenDesktopW, OpenWindowStationW, SendInput, SetForegroundWindow,
    SetProcessWindowStation, SetWindowPlacement, ShowWindow, ShowWindowAsync, SwitchToThisWindow,
    INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, SW_RESTORE, SW_SHOWNORMAL,
    VK_CONTROL, VK_RETURN, WINDOWPLACEMENT, WINSTA_ALL_ACCESS,
};
use std::thread;
use std::time::Duration;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn send_key(vk: u16, flags: u32) {
    let mut input: INPUT = std::mem::zeroed();
    input.type_ = INPUT_KEYBOARD;
    let mut ki: KEYBDINPUT = std::mem::zeroed();
    ki.wVk = vk;
    ki.dwFlags = flags;
    *input.u.ki_mut() = ki;
    SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
}

unsafe fn send_char(c: u16) {
    let mut input: INPUT = std::mem::zeroed();
    input.type_ = INPUT_KEYBOARD;
    let mut ki: KEYBDINPUT = std::mem::zeroed();
    ki.wScan = c;
    ki.dwFlags = KEYEVENTF_UNICODE;
    *input.u.ki_mut() = ki;
    SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);

    let mut input_up: INPUT = std::mem::zeroed();
    input_up.type_ = INPUT_KEYBOARD;
    let mut ki_up: KEYBDINPUT = std::mem::zeroed();
    ki_up.wScan = c;
    ki_up.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;
    *input_up.u.ki_mut() = ki_up;
    SendInput(1, &mut input_up, std::mem::size_of::<INPUT>() as i32);
}

unsafe fn send_text(s: &str) {
    for c in s.encode_utf16() {
        send_char(c);
        thread::sleep(Duration::from_millis(5));
    }
}

unsafe fn send_hotkey_ctrl_t() {
    send_key(VK_CONTROL as u16, 0);
    thread::sleep(Duration::from_millis(30));
    send_key(b'T' as u16, 0);
    thread::sleep(Duration::from_millis(30));
    send_key(b'T' as u16, KEYEVENTF_KEYUP);
    thread::sleep(Duration::from_millis(30));
    send_key(VK_CONTROL as u16, KEYEVENTF_KEYUP);
    thread::sleep(Duration::from_millis(50));
}

unsafe fn send_enter() {
    send_key(VK_RETURN as u16, 0);
    thread::sleep(Duration::from_millis(30));
    send_key(VK_RETURN as u16, KEYEVENTF_KEYUP);
}

#[test]
fn test_restore_placement_edge() {
    unsafe {
        let winsta_name = to_wide("WinSta0");
        let winsta = OpenWindowStationW(winsta_name.as_ptr(), FALSE, WINSTA_ALL_ACCESS);
        assert!(!winsta.is_null(), "WinSta0");
        SetProcessWindowStation(winsta);

        let desk_name = to_wide("Default");
        let desk = OpenDesktopW(desk_name.as_ptr(), 0, FALSE, 0x01FF);
        assert!(!desk.is_null(), "Default Desktop");

        struct Finder {
            edge_hwnd: Option<HWND>,
            title: String,
        }

        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let finder = &mut *(lparam as *mut Finder);
            if IsWindowVisible(hwnd) != 0 {
                let mut title_buf = [0u16; 256];
                let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 256);
                if len > 0 {
                    let title = String::from_utf16_lossy(&title_buf[..len as usize]);
                    if title.contains("Edge") || title.contains("Microsoft​ Edge") {
                        finder.edge_hwnd = Some(hwnd);
                        finder.title = title;
                        return FALSE;
                    }
                }
            }
            TRUE
        }

        let mut finder = Finder { edge_hwnd: None, title: String::new() };
        EnumDesktopWindows(desk, Some(enum_proc), &mut finder as *mut _ as LPARAM);
        CloseDesktop(desk);
        CloseWindowStation(winsta);

        if let Some(hwnd) = finder.edge_hwnd {
            let mut wp: WINDOWPLACEMENT = std::mem::zeroed();
            wp.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
            GetWindowPlacement(hwnd, &mut wp);
            println!("Initial showCmd: {}, rcNormalPosition: ({}, {}, {}, {})", 
                wp.showCmd, wp.rcNormalPosition.left, wp.rcNormalPosition.top, wp.rcNormalPosition.right, wp.rcNormalPosition.bottom);

            let mut rect: winapi::shared::windef::RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut rect);
            println!("Initial WindowRect: ({}, {}, {}, {})", rect.left, rect.top, rect.right, rect.bottom);

            // Restore window placement
            wp.showCmd = SW_SHOWNORMAL as u32;
            SetWindowPlacement(hwnd, &wp);
            ShowWindow(hwnd, SW_RESTORE);
            ShowWindowAsync(hwnd, SW_RESTORE);
            SwitchToThisWindow(hwnd, TRUE);

            let fg_hwnd = GetForegroundWindow();
            let fg_thread = GetWindowThreadProcessId(fg_hwnd, std::ptr::null_mut());
            let cur_thread = GetCurrentThreadId();

            AttachThreadInput(cur_thread, fg_thread, TRUE);
            SetForegroundWindow(hwnd);
            BringWindowToTop(hwnd);
            AttachThreadInput(cur_thread, fg_thread, FALSE);

            thread::sleep(Duration::from_millis(800));

            GetWindowRect(hwnd, &mut rect);
            println!("After Restore WindowRect: ({}, {}, {}, {})", rect.left, rect.top, rect.right, rect.bottom);

            // Send Ctrl+T
            println!("Sending Ctrl+T...");
            send_hotkey_ctrl_t();
            thread::sleep(Duration::from_millis(600));

            // Type exact URL
            let search_url = "https://search.bilibili.com/all?keyword=live2d";
            println!("Typing URL: {}", search_url);
            send_text(search_url);
            thread::sleep(Duration::from_millis(400));

            // Press Enter
            println!("Sending Enter key...");
            send_enter();
            println!("Navigation completed!");
        }
    }
}
