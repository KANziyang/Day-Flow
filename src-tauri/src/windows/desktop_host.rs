//! Explorer integration is deliberately isolated here. A non-topmost window is
//! always reported as compatibility mode, never as a desktop attachment.
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
    static SAVED: Mutex<Option<(isize, isize, isize)>> = Mutex::new(None);
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
            let next = FindWindowExW(
                std::ptr::null_mut(),
                top,
                wide("WorkerW").as_ptr(),
                std::ptr::null(),
            );
            if !next.is_null() {
                *(param as *mut HWND) = next;
                return 0;
            }
        }
        1
    }
    pub fn attach(w: &tauri::WebviewWindow) -> ModeStatus {
        let result = (|| -> Result<()> {
            unsafe {
                let hwnd = w.hwnd()?.0 as HWND;
                let progman = FindWindowW(wide("Progman").as_ptr(), std::ptr::null());
                if progman.is_null() {
                    anyhow::bail!("未找到 Explorer 桌面");
                }
                let mut response = 0;
                if SendMessageTimeoutW(progman, 0x052c, 0, 0, SMTO_ABORTIFHUNG, 500, &mut response)
                    == 0
                {
                    anyhow::bail!("Explorer 桌面无响应");
                }
                let mut host: HWND = std::ptr::null_mut();
                EnumWindows(Some(find_host), &mut host as *mut HWND as LPARAM);
                if host.is_null() || IsWindow(host) == 0 {
                    anyhow::bail!("当前桌面结构不支持挂载");
                }
                if AreDpiAwarenessContextsEqual(
                    GetWindowDpiAwarenessContext(hwnd),
                    GetWindowDpiAwarenessContext(host),
                ) == 0
                {
                    anyhow::bail!("桌面与应用 DPI 上下文不同，已避免跨进程 DPI 重置");
                }
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                let parent = GetParent(hwnd);
                let mut rect = std::mem::zeroed();
                GetWindowRect(hwnd, &mut rect);
                let mut p = POINT {
                    x: rect.left,
                    y: rect.top,
                };
                MapWindowPoints(std::ptr::null_mut(), host, &mut p, 1);
                SetLastError(0);
                SetParent(hwnd, host);
                if GetParent(hwnd) != host {
                    anyhow::bail!("Windows 拒绝桌面挂载");
                }
                SetWindowLongPtrW(
                    hwnd,
                    GWL_STYLE,
                    (style & !(WS_POPUP as isize)) | WS_CHILD as isize,
                );
                if SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    p.x,
                    p.y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                ) == 0
                {
                    SetParent(hwnd, parent);
                    SetWindowLongPtrW(hwnd, GWL_STYLE, style);
                    anyhow::bail!("挂载定位失败，已撤销");
                }
                *SAVED
                    .lock()
                    .map_err(|_| anyhow::anyhow!("桌面状态锁异常"))? =
                    Some((host as isize, parent as isize, style));
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
        unsafe {
            if let Some((_, parent, style)) = SAVED
                .lock()
                .map_err(|_| anyhow::anyhow!("桌面状态锁异常"))?
                .take()
            {
                let hwnd = w.hwnd()?.0 as HWND;
                let mut r = std::mem::zeroed();
                GetWindowRect(hwnd, &mut r);
                SetParent(hwnd, parent as HWND);
                SetWindowLongPtrW(hwnd, GWL_STYLE, style);
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    r.left,
                    r.top,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }
            Ok(())
        }
    }
    pub fn host_lost() -> bool {
        SAVED
            .lock()
            .ok()
            .and_then(|s| *s)
            .is_some_and(|(host, _, _)| unsafe { IsWindow(host as HWND) == 0 })
    }
}
#[cfg(windows)]
pub use native::{attach, detach, host_lost};
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
