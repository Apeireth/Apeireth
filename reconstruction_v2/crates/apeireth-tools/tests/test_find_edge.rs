use winapi::shared::minwindef::{BOOL, FALSE, LPARAM, TRUE};
use winapi::shared::windef::{HWND, RECT};
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winuser::{
    AttachThreadInput, BringWindowToTop, CloseDesktop, CloseWindowStation, EnumDesktopWindows,
    GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible, OpenDesktopW, OpenWindowStationW, PostMessageW, SetForegroundWindow,
    SetProcessWindowStation, ShowWindow, ShowWindowAsync, SC_RESTORE, SW_MAXIMIZE, SW_RESTORE,
    SW_SHOW, VK_CONTROL, VK_RETURN, WM_SYSCOMMAND, WINSTA_ALL_ACCESS, SendInput, INPUT,
    INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
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

unsafe fn send_hotkey_ctrl_t() {
    send_key(VK_CONTROL as u16, 0);
    thread::sleep(Duration::from_millis(50));
    send_key(b'T' as u16, 0);
    thread::sleep(Duration::from_millis(50));
    send_key(b'T' as u16, KEYEVENTF_KEYUP);
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
fn test_live_dynamic_edge_locate_and_navigate() {
    unsafe {
        let winsta_name = to_wide("WinSta0");
        let winsta = OpenWindowStationW(winsta_name.as_ptr(), FALSE, WINSTA_ALL_ACCESS);
        assert!(!winsta.is_null());
        SetProcessWindowStation(winsta);

        let desk_name = to_wide("Default");
        let desk = OpenDesktopW(desk_name.as_ptr(), 0, FALSE, 0x01FF);
        assert!(!desk.is_null());

        struct DynamicFinder {
            candidates: Vec<(HWND, String, RECT)>,
        }

        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let finder = &mut *(lparam as *mut DynamicFinder);
            let is_vis = IsWindowVisible(hwnd);
            if is_vis != 0 {
                let mut title_buf = [0u16; 512];
                let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 512);
                let mut class_buf = [0u16; 256];
                let clen = GetClassNameW(hwnd, class_buf.as_mut_ptr(), 256);
                let mut rect: RECT = std::mem::zeroed();
                GetWindowRect(hwnd, &mut rect);

                let title = if len > 0 { String::from_utf16_lossy(&title_buf[..len as usize]) } else { String::new() };
                let class = if clen > 0 { String::from_utf16_lossy(&class_buf[..clen as usize]) } else { String::new() };

                if class == "Chrome_WidgetWin_1" && (title.contains("Edge") || title.contains("CSDN") || title.contains("bilibili") || title.contains("Live2d") || title.contains("新标签页")) {
                    finder.candidates.push((hwnd, title, rect));
                }
            }
            TRUE
        }

        let mut finder = DynamicFinder { candidates: Vec::new() };
        EnumDesktopWindows(desk, Some(enum_proc), &mut finder as *mut _ as LPARAM);
        CloseDesktop(desk);
        CloseWindowStation(winsta);

        println!("Found {} Edge candidate windows:", finder.candidates.len());
        for (h, t, r) in &finder.candidates {
            println!("- HWND={:?}, Rect=({},{},{},{}), Title=\"{}\"", h, r.left, r.top, r.right, r.bottom, t);
        }

        if let Some((hwnd, title, _rect)) = finder.candidates.first().cloned() {

            println!("\nTargeting Edge window HWND={:?} (\"{}\")", hwnd, title);

            PostMessageW(hwnd, WM_SYSCOMMAND, SC_RESTORE, 0);
            ShowWindow(hwnd, SW_RESTORE);
            ShowWindow(hwnd, SW_MAXIMIZE);
            ShowWindowAsync(hwnd, SW_RESTORE);
            ShowWindowAsync(hwnd, SW_SHOW);

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

            println!("Sending Ctrl+T...");
            send_hotkey_ctrl_t();
            thread::sleep(Duration::from_millis(600));

            let search_url = "https://search.bilibili.com/all?keyword=live2d";
            println!("Typing URL: {}", search_url);
            send_text(search_url);
            thread::sleep(Duration::from_millis(400));

            println!("Pressing Enter...");
            send_enter();
            thread::sleep(Duration::from_millis(1500));

            println!("Finished live dynamic navigation test.");
        }
    }
}
