use winapi::um::winuser::{
    GetCursorPos, SetCursorPos, OpenDesktopW, OpenWindowStationW,
    SetProcessWindowStation, SetThreadDesktop, CloseDesktop, CloseWindowStation,
    WINSTA_ALL_ACCESS,
};
use winapi::shared::minwindef::FALSE;
use winapi::shared::windef::POINT;
use std::thread;
use std::time::Duration;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[test]
fn test_mouse_move_with_thread_desktop() {
    unsafe {
        let winsta_name = to_wide("WinSta0");
        let winsta = OpenWindowStationW(winsta_name.as_ptr(), FALSE, WINSTA_ALL_ACCESS);
        assert!(!winsta.is_null(), "WinSta0");
        SetProcessWindowStation(winsta);

        let desk_name = to_wide("Default");
        let desk = OpenDesktopW(desk_name.as_ptr(), 0, FALSE, 0x01FF);
        assert!(!desk.is_null(), "Default Desktop");
        
        let ok = SetThreadDesktop(desk);
        println!("SetThreadDesktop(desk) result: {}", ok);

        let mut pt: POINT = std::mem::zeroed();
        GetCursorPos(&mut pt);
        println!("Initial Mouse Position on Interactive Desktop: ({}, {})", pt.x, pt.y);

        println!("Moving mouse cursor to (853, 533)...");
        let res = SetCursorPos(853, 533);
        println!("SetCursorPos result: {}", res);
        thread::sleep(Duration::from_millis(100));

        GetCursorPos(&mut pt);
        println!("New Mouse Position: ({}, {})", pt.x, pt.y);

        CloseDesktop(desk);
        CloseWindowStation(winsta);
    }
}
