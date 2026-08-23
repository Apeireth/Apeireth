use winapi::shared::minwindef::{BOOL, FALSE, LPARAM, TRUE};
use winapi::shared::windef::{HWND, RECT};
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::shellapi::ShellExecuteW;
use winapi::um::winuser::{
    AttachThreadInput, BringWindowToTop, CloseDesktop, CloseWindowStation, EnumDesktopWindows,
    GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible, OpenDesktopW, OpenWindowStationW, SendInput, SetForegroundWindow,
    SetProcessWindowStation, ShowWindow, ShowWindowAsync, INPUT, INPUT_KEYBOARD, KEYBDINPUT,
    KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, SW_MAXIMIZE, SW_RESTORE, SW_SHOWNORMAL, VK_CONTROL,
    VK_RETURN, WINSTA_ALL_ACCESS,
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
        thread::sleep(Duration::from_millis(10));
    }
}

unsafe fn send_hotkey_ctrl_l() {
    send_key(VK_CONTROL as u16, 0);
    thread::sleep(Duration::from_millis(50));
    send_key(b'L' as u16, 0);
    thread::sleep(Duration::from_millis(50));
    send_key(b'L' as u16, KEYEVENTF_KEYUP);
    thread::sleep(Duration::from_millis(50));
    send_key(VK_CONTROL as u16, KEYEVENTF_KEYUP);
    thread::sleep(Duration::from_millis(100));
}

unsafe fn send_enter() {
    send_key(VK_RETURN as u16, 0);
    thread::sleep(Duration::from_millis(50));
    send_key(VK_RETURN as u16, KEYEVENTF_KEYUP);
}

#[test]
fn test_pop_and_navigate_edge() {
    unsafe {
        // Step 1: Open URL via native ShellExecuteW
        let op = to_wide("open");
        let url_str = "https://search.bilibili.com/all?keyword=live2d";
        let url = to_wide(url_str);
        ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            url.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );

        thread::sleep(Duration::from_millis(1000));

        // Step 2: Enumerate Desktop and find Edge window
        let winsta_name = to_wide("WinSta0");
        let winsta = OpenWindowStationW(winsta_name.as_ptr(), FALSE, WINSTA_ALL_ACCESS);
        SetProcessWindowStation(winsta);
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
        CloseWindowStation(winsta);

        println!("Located Edge HWND: {:?}, Title: \"{}\"", finder.hwnd, finder.title);

        if let Some(hwnd) = finder.hwnd {
            // Step 3: Force unminimize and bring to front
            ShowWindow(hwnd, SW_RESTORE);
            ShowWindow(hwnd, SW_MAXIMIZE);
            ShowWindowAsync(hwnd, SW_RESTORE);
            ShowWindowAsync(hwnd, SW_MAXIMIZE);

            let fg_hwnd = GetForegroundWindow();
            let fg_thread = GetWindowThreadProcessId(fg_hwnd, std::ptr::null_mut());
            let edge_thread = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
            let cur_thread = GetCurrentThreadId();

            AttachThreadInput(cur_thread, fg_thread, TRUE);
            AttachThreadInput(cur_thread, edge_thread, TRUE);
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
            AttachThreadInput(cur_thread, fg_thread, FALSE);
            AttachThreadInput(cur_thread, edge_thread, FALSE);

            thread::sleep(Duration::from_millis(800));

            // Step 4: Focus address bar (Ctrl+L) and navigate to URL
            println!("Sending Ctrl+L to focus address bar...");
            send_hotkey_ctrl_l();
            thread::sleep(Duration::from_millis(300));

            println!("Typing URL...");
            send_text(url_str);
            thread::sleep(Duration::from_millis(300));

            println!("Sending Enter...");
            send_enter();

            println!("Step 4 completed!");
        }
    }
}
