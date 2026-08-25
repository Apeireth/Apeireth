use winapi::shared::minwindef::{BOOL, FALSE, LPARAM, TRUE};
use winapi::shared::windef::HWND;
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winuser::{
    AttachThreadInput, BringWindowToTop, CloseDesktop, CloseWindowStation, EnumDesktopWindows,
    GetClassNameW, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    OpenDesktopW, OpenWindowStationW, SendInput, SetForegroundWindow, SetProcessWindowStation,
    SetThreadDesktop, ShowWindow, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, SW_RESTORE, SW_SHOW, VK_CONTROL, VK_RETURN, WINSTA_ALL_ACCESS,
};
use std::thread;
use std::time::Duration;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn attach_desktop() {
    let winsta = OpenWindowStationW(to_wide("WinSta0").as_ptr(), FALSE, WINSTA_ALL_ACCESS);
    if !winsta.is_null() {
        SetProcessWindowStation(winsta);
    }
    let desk = OpenDesktopW(to_wide("Default").as_ptr(), 0, FALSE, 0x01FF);
    if !desk.is_null() {
        SetThreadDesktop(desk);
    }
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
#[ignore = "invasive e2e: takes over desktop, focuses Edge, types live2d bilibili URL via SendInput"]
fn test_live_navigate_foreground_edge() {
    unsafe {
        attach_desktop();

        let desk_name = to_wide("Default");
        let desk = OpenDesktopW(desk_name.as_ptr(), 0, FALSE, 0x01FF);

        struct EdgeFinder {
            hwnd: Option<HWND>,
            title: String,
        }

        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let finder = &mut *(lparam as *mut EdgeFinder);
            let mut class_buf = [0u16; 256];
            let clen = GetClassNameW(hwnd, class_buf.as_mut_ptr(), 256);
            let class = if clen > 0 { String::from_utf16_lossy(&class_buf[..clen as usize]) } else { String::new() };

            if class == "Chrome_WidgetWin_1" {
                let mut title_buf = [0u16; 512];
                let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 512);
                let title = if len > 0 { String::from_utf16_lossy(&title_buf[..len as usize]) } else { String::new() };
                if title.contains("Edge") || title.contains("CSDN") || title.contains("bilibili") || title.contains("Live2d") {
                    finder.hwnd = Some(hwnd);
                    finder.title = title;
                    return FALSE;
                }
            }
            TRUE
        }

        let mut finder = EdgeFinder { hwnd: None, title: String::new() };
        EnumDesktopWindows(desk, Some(enum_proc), &mut finder as *mut _ as LPARAM);
        CloseDesktop(desk);

        println!("Found Edge: HWND={:?}, Title=\"{}\"", finder.hwnd, finder.title);

        if let Some(hwnd) = finder.hwnd {
            // 1. Focus Edge
            let fg_hwnd = GetForegroundWindow();
            let fg_thread = GetWindowThreadProcessId(fg_hwnd, std::ptr::null_mut());
            let target_thread = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
            let cur_thread = GetCurrentThreadId();

            AttachThreadInput(cur_thread, fg_thread, TRUE);
            AttachThreadInput(cur_thread, target_thread, TRUE);
            ShowWindow(hwnd, SW_RESTORE);
            ShowWindow(hwnd, SW_SHOW);
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
            AttachThreadInput(cur_thread, fg_thread, FALSE);
            AttachThreadInput(cur_thread, target_thread, FALSE);

            println!("Edge brought to physical focus. Waiting 400ms...");
            thread::sleep(Duration::from_millis(400));

            // 2. Open new tab with Ctrl+T
            println!("Sending Ctrl+T to create a new tab...");
            send_hotkey_ctrl_t();
            thread::sleep(Duration::from_millis(600));

            // 3. Type Bilibili Live2D search URL
            let search_url = "https://search.bilibili.com/all?keyword=live2d";
            println!("Typing URL: {}", search_url);
            send_text(search_url);
            thread::sleep(Duration::from_millis(300));

            // 4. Press Enter
            println!("Sending Enter key...");
            send_enter();

            println!("Navigation completed!");
        }
    }
}
