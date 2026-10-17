//! Interactive widgets share the desktop icon host, above SHELLDLL_DefView.
//! The WorkerW behind that host is a wallpaper surface and cannot receive input
//! through the icon view. Keep Explorer integration and child visibility here.
use anyhow::Result;
use serde::Serialize;
#[derive(Clone, Serialize)]
pub struct ModeStatus {
    pub mode: String,
    pub embedded: bool,
    pub message: String,
}
fn compatible(reason: &str) -> ModeStatus {
    ModeStatus {
        mode: "desktop".into(),
        embedded: false,
        message: format!("桌面兼容模式 · {reason}"),
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::sync::Mutex;
    use windows_sys::core::BOOL;
    use windows_sys::Win32::{
        Foundation::*,
        Graphics::Gdi::MapWindowPoints,
        UI::{HiDpi::*, WindowsAndMessaging::*},
    };
    // Integers avoid sending HWND raw pointers between threads. All operations are
    // dispatched by the Tauri main thread; only this saved state crosses threads.
    #[derive(Clone, Copy)]
    struct Attachment {
        host: isize,
        parent: isize,
        style: isize,
    }
    static SAVED: Mutex<Option<Attachment>> = Mutex::new(None);
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    unsafe extern "system" fn find_host(top: HWND, param: LPARAM) -> BOOL {
        let view = FindWindowExW(
            top,
            std::ptr::null_mut(),
            wide("SHELLDLL_DefView").as_ptr(),
            std::ptr::null(),
        );
        if !view.is_null() {
            *(param as *mut HWND) = top;
            return 0;
        }
        1
    }
    unsafe fn set_style(hwnd: HWND, style: isize) -> Result<()> {
        SetLastError(0);
        if SetWindowLongPtrW(hwnd, GWL_STYLE, style) == 0 && GetLastError() != 0 {
            anyhow::bail!("无法更新小组件窗口样式");
        }
        Ok(())
    }
    unsafe fn position(hwnd: HWND, parent: HWND, rect: &RECT, above_icons: bool) -> Result<()> {
        let mut p = POINT {
            x: rect.left,
            y: rect.top,
        };
        SetLastError(0);
        if MapWindowPoints(std::ptr::null_mut(), parent, &mut p, 1) == 0 && GetLastError() != 0 {
            anyhow::bail!("无法转换小组件窗口坐标");
        }
        let flags = SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED;
        if SetWindowPos(
            hwnd,
            HWND_TOP,
            p.x,
            p.y,
            0,
            0,
            flags | if above_icons { 0 } else { SWP_NOZORDER },
        ) == 0
        {
            anyhow::bail!("无法定位小组件窗口");
        }
        Ok(())
    }
    unsafe fn restore(hwnd: HWND, saved: Attachment) -> Result<bool> {
        let mut rect = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            anyhow::bail!("无法读取小组件窗口位置");
        }
        // Visibility belongs to the current window, not its pre-attachment state.
        let visible = GetWindowLongPtrW(hwnd, GWL_STYLE) & WS_VISIBLE as isize;
        SetLastError(0);
        if SetParent(hwnd, saved.parent as HWND).is_null() && GetLastError() != 0 {
            anyhow::bail!("Windows 拒绝解除桌面挂载");
        }
        set_style(hwnd, (saved.style & !(WS_VISIBLE as isize)) | visible)?;
        position(hwnd, saved.parent as HWND, &rect, false)?;
        Ok(visible != 0)
    }
    unsafe fn mount(hwnd: HWND, host: HWND) -> Result<Attachment> {
        if host.is_null() || IsWindow(host) == 0 {
            anyhow::bail!("未找到桌面图标宿主");
        }
        if AreDpiAwarenessContextsEqual(
            GetWindowDpiAwarenessContext(hwnd),
            GetWindowDpiAwarenessContext(host),
        ) == 0
        {
            anyhow::bail!("桌面与应用 DPI 上下文不同，已避免跨进程 DPI 重置");
        }
        let saved = Attachment {
            host: host as isize,
            parent: GetParent(hwnd) as isize,
            style: GetWindowLongPtrW(hwnd, GWL_STYLE),
        };
        let mut rect = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            anyhow::bail!("无法读取小组件窗口位置");
        }
        // SetParent does not set WS_CHILD. Set it before reparenting, as Win32 requires.
        set_style(
            hwnd,
            (saved.style & !(WS_POPUP as isize)) | WS_CHILD as isize,
        )?;
        SetParent(hwnd, host);
        if GetParent(hwnd) != host {
            set_style(hwnd, saved.style)?;
            anyhow::bail!("Windows 拒绝桌面挂载");
        }
        if let Err(error) = position(hwnd, host, &rect, true) {
            restore(hwnd, saved)?;
            return Err(error);
        }
        Ok(saved)
    }
    pub fn attach(w: &tauri::WebviewWindow) -> ModeStatus {
        let result = (|| -> Result<()> {
            let mut saved = SAVED
                .lock()
                .map_err(|_| anyhow::anyhow!("桌面状态锁异常"))?;
            unsafe {
                let hwnd = w.hwnd()?.0 as HWND;
                let mut host: HWND = std::ptr::null_mut();
                EnumWindows(Some(find_host), &mut host as *mut HWND as LPARAM);
                *saved = Some(mount(hwnd, host)?);
                Ok(())
            }
        })();
        match result {
            Ok(()) => ModeStatus {
                mode: "desktop".into(),
                embedded: true,
                message: "已挂载桌面宿主（需按设备验收）".into(),
            },
            Err(e) => compatible(&e.to_string()),
        }
    }
    pub fn detach(w: &tauri::WebviewWindow) -> Result<()> {
        let visible = {
            let mut saved = SAVED
                .lock()
                .map_err(|_| anyhow::anyhow!("桌面状态锁异常"))?;
            let Some(attachment) = *saved else {
                return Ok(());
            };
            let visible = unsafe { restore(w.hwnd()?.0 as HWND, attachment)? };
            *saved = None;
            visible
        };
        // Tao caches visibility and rebuilds GWL_STYLE on show/hide. Synchronize
        // only after detaching, when rebuilding a top-level style is safe again.
        if visible {
            w.show()?;
        } else {
            w.hide()?;
        }
        Ok(())
    }
    pub fn set_visible(w: &tauri::WebviewWindow, visible: bool) -> Result<bool> {
        let saved = SAVED
            .lock()
            .map_err(|_| anyhow::anyhow!("桌面状态锁异常"))?;
        if saved.is_none() {
            return Ok(false);
        }
        unsafe {
            let hwnd = w.hwnd()?.0 as HWND;
            // Tauri show/hide would erase WS_CHILD because its window was created
            // as top-level. Native visibility changes preserve the attachment.
            ShowWindow(hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
            if visible
                && SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                ) == 0
            {
                anyhow::bail!("无法显示桌面小组件");
            }
        }
        Ok(true)
    }
    pub fn host_lost() -> bool {
        SAVED
            .lock()
            .ok()
            .and_then(|s| *s)
            .is_some_and(|saved| unsafe { IsWindow(saved.host as HWND) == 0 })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{ptr::null_mut, sync::Once};

        // These are private, hidden test windows. No test enumerates, reparents,
        // sends messages to, or changes the visibility of Explorer windows.
        struct TestWindow(HWND);

        impl TestWindow {
            unsafe fn new(class: &str, style: u32, parent: HWND, x: i32, y: i32) -> Self {
                let hwnd = CreateWindowExW(
                    0,
                    wide(class).as_ptr(),
                    wide("DayFlow desktop host regression").as_ptr(),
                    style,
                    x,
                    y,
                    240,
                    180,
                    parent,
                    null_mut(),
                    null_mut(),
                    std::ptr::null(),
                );
                assert!(
                    !hwnd.is_null(),
                    "CreateWindowExW failed: {}",
                    GetLastError()
                );
                Self(hwnd)
            }

            unsafe fn rect(&self) -> (i32, i32, i32, i32) {
                let mut rect = RECT::default();
                assert_ne!(GetWindowRect(self.0, &mut rect), 0);
                (rect.left, rect.top, rect.right, rect.bottom)
            }

            unsafe fn style(&self) -> isize {
                GetWindowLongPtrW(self.0, GWL_STYLE)
            }
        }

        impl Drop for TestWindow {
            fn drop(&mut self) {
                unsafe {
                    if IsWindow(self.0) != 0 {
                        DestroyWindow(self.0);
                    }
                }
            }
        }

        fn register_shell_classes() {
            static REGISTER: Once = Once::new();
            REGISTER.call_once(|| unsafe {
                for class in ["SHELLDLL_DefView", "WorkerW"] {
                    let name = wide(class);
                    let definition = WNDCLASSW {
                        lpfnWndProc: Some(DefWindowProcW),
                        lpszClassName: name.as_ptr(),
                        ..Default::default()
                    };
                    assert_ne!(
                        RegisterClassW(&definition),
                        0,
                        "RegisterClassW({class}) failed: {}",
                        GetLastError()
                    );
                }
            });
        }

        #[test]
        fn discovers_icon_parent_instead_of_wallpaper_worker() {
            register_shell_classes();
            unsafe {
                let host = TestWindow::new("STATIC", WS_POPUP, null_mut(), 80, 120);
                let _icons = TestWindow::new("SHELLDLL_DefView", WS_CHILD, host.0, 0, 0);
                let wallpaper = TestWindow::new("WorkerW", WS_POPUP, null_mut(), 80, 120);
                assert_ne!(
                    SetWindowPos(
                        wallpaper.0,
                        host.0,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    ),
                    0
                );

                let mut found = null_mut();
                assert_eq!(find_host(host.0, &mut found as *mut HWND as LPARAM), 0);
                assert_eq!(found, host.0);
                assert_ne!(found, wallpaper.0);

                found = null_mut();
                assert_eq!(find_host(wallpaper.0, &mut found as *mut HWND as LPARAM), 1);
                assert!(found.is_null());
            }
        }

        #[test]
        fn mount_and_restore_popup_preserve_geometry_style_and_hidden_state() {
            register_shell_classes();
            unsafe {
                let host = TestWindow::new("STATIC", WS_POPUP, null_mut(), 70, 110);
                let _icons = TestWindow::new("SHELLDLL_DefView", WS_CHILD, host.0, 0, 0);
                let widget =
                    TestWindow::new("STATIC", WS_POPUP | WS_CLIPSIBLINGS, null_mut(), 330, 260);
                let original_rect = widget.rect();
                let original_style = widget.style();

                let attachment = mount(widget.0, host.0).unwrap();
                assert_eq!(GetParent(widget.0), host.0);
                assert_eq!(widget.rect(), original_rect);
                assert_eq!(
                    widget.style(),
                    (original_style & !(WS_POPUP as isize)) | WS_CHILD as isize
                );
                assert!(GetWindow(widget.0, GW_HWNDPREV).is_null());
                assert_eq!(IsWindowVisible(widget.0), 0);

                assert!(!restore(widget.0, attachment).unwrap());
                assert!(GetParent(widget.0).is_null());
                assert_eq!(widget.rect(), original_rect);
                assert_eq!(widget.style(), original_style);
                assert_eq!(IsWindowVisible(widget.0), 0);
            }
        }

        #[test]
        fn restore_child_preserves_current_visibility_and_parent_coordinates() {
            unsafe {
                for initially_visible in [false, true] {
                    let original_parent = TestWindow::new("STATIC", WS_POPUP, null_mut(), 60, 90);
                    let host = TestWindow::new("STATIC", WS_POPUP, null_mut(), 410, 340);
                    let widget = TestWindow::new(
                        "STATIC",
                        WS_CHILD | WS_CLIPSIBLINGS | if initially_visible { WS_VISIBLE } else { 0 },
                        original_parent.0,
                        35,
                        45,
                    );
                    let original_rect = widget.rect();
                    let original_style = widget.style();
                    let attachment = mount(widget.0, host.0).unwrap();
                    let visible = !initially_visible;
                    ShowWindow(widget.0, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
                    assert_eq!(widget.style() & WS_VISIBLE as isize != 0, visible);
                    // A visible child of a hidden parent never appears on the desktop.
                    assert_eq!(IsWindowVisible(widget.0), 0);

                    assert_eq!(restore(widget.0, attachment).unwrap(), visible);
                    assert_eq!(GetParent(widget.0), original_parent.0);
                    assert_eq!(widget.rect(), original_rect);
                    assert_eq!(
                        widget.style(),
                        (original_style & !(WS_VISIBLE as isize))
                            | if visible { WS_VISIBLE as isize } else { 0 }
                    );
                    assert_eq!(IsWindowVisible(widget.0), 0);
                }
            }
        }
    }
}
#[cfg(windows)]
pub use native::{attach, detach, host_lost, set_visible};
#[cfg(not(windows))]
pub fn attach(_: &tauri::WebviewWindow) -> ModeStatus {
    compatible("仅支持 Windows")
}
#[cfg(not(windows))]
pub fn detach(_: &tauri::WebviewWindow) -> Result<()> {
    Ok(())
}
#[cfg(not(windows))]
pub fn host_lost() -> bool {
    false
}
#[cfg(not(windows))]
pub fn set_visible(_: &tauri::WebviewWindow, _: bool) -> Result<bool> {
    Ok(false)
}
