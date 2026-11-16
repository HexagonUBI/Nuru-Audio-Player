use std::cell::{Cell, RefCell};
use std::ffi::c_void;

use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_NONE, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyMenu, DestroyWindow, FindWindowExW,
    GetCursorPos, GetSystemMetrics, LoadImageW, PostMessageW, RegisterClassExW, RegisterWindowMessageW, SetForegroundWindow,
    TrackPopupMenu, HICON, HWND_MESSAGE, IMAGE_ICON, LR_DEFAULTCOLOR, MF_GRAYED, MF_SEPARATOR, MF_STRING, SM_CXSMICON,
    SM_CYSMICON, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_CONTEXTMENU, WM_LBUTTONDBLCLK, WM_LBUTTONUP,
    WM_NULL, WM_RBUTTONUP, WNDCLASSEXW,
};

const CLASS: &str = "NuruTray";
const TRAY_MESSAGE: u32 = WM_APP + 1;
const SHOW_MESSAGE: u32 = WM_APP + 2;

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Open,
    TogglePlaying,
    Exit,
}

thread_local! {
    static HANDLER: RefCell<Option<Box<dyn Fn(Action)>>> = const { RefCell::new(None) };
    static DATA: RefCell<Option<NOTIFYICONDATAW>> = const { RefCell::new(None) };
    static PLAYING: Cell<bool> = const { Cell::new(false) };
    static HAS_LAYERS: Cell<bool> = const { Cell::new(false) };
    static RECREATED: Cell<u32> = const { Cell::new(0) };
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateMutexW(attributes: *const c_void, owner: i32, name: *const u16) -> *mut c_void;
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn fill(target: &mut [u16], text: &str) {
    let units: Vec<u16> = text.encode_utf16().take(target.len() - 1).collect();
    target.fill(0);
    target[..units.len()].copy_from_slice(&units);
}

pub fn claim_instance(key: &str) -> bool {
    let name = wide(&format!("Local\\{key}"));
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() || unsafe { GetLastError() } != ERROR_ALREADY_EXISTS {
        return true;
    }
    let class = wide(CLASS);
    let title = wide(key);
    let other = unsafe { FindWindowExW(HWND_MESSAGE, std::ptr::null_mut(), class.as_ptr(), title.as_ptr()) };
    if !other.is_null() {
        unsafe { PostMessageW(other, SHOW_MESSAGE, 0, 0) };
    }
    false
}

fn fire(action: Action) {
    HANDLER.with(|h| {
        if let Some(handler) = h.borrow().as_ref() {
            handler(action);
        }
    });
}

unsafe fn menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return;
    }
    let open = wide("Open Nuru");
    let toggle = wide(if PLAYING.with(Cell::get) { "Pause all" } else { "Play all" });
    let exit = wide("Exit");
    AppendMenuW(menu, MF_STRING, 1, open.as_ptr());
    AppendMenuW(menu, if HAS_LAYERS.with(Cell::get) { MF_STRING } else { MF_STRING | MF_GRAYED }, 2, toggle.as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
    AppendMenuW(menu, MF_STRING, 3, exit.as_ptr());
    let mut at = POINT { x: 0, y: 0 };
    GetCursorPos(&mut at);
    SetForegroundWindow(hwnd);
    let picked = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY, at.x, at.y, 0, hwnd, std::ptr::null());
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu);
    match picked {
        1 => fire(Action::Open),
        2 => fire(Action::TogglePlaying),
        3 => fire(Action::Exit),
        _ => {}
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        TRAY_MESSAGE => {
            match lparam as u32 {
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => fire(Action::Open),
                WM_RBUTTONUP | WM_CONTEXTMENU => menu(hwnd),
                _ => {}
            }
            0
        }
        SHOW_MESSAGE => {
            fire(Action::Open);
            0
        }
        m if m != 0 && m == RECREATED.with(Cell::get) => {
            DATA.with(|d| {
                if let Some(data) = d.borrow().as_ref() {
                    Shell_NotifyIconW(NIM_ADD, data);
                }
            });
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

pub struct Tray {
    hwnd: HWND,
    icon: HICON,
}

impl Tray {
    pub fn create(key: &str, handler: impl Fn(Action) + 'static) -> Option<Tray> {
        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let class = wide(CLASS);
            let mut wc: WNDCLASSEXW = std::mem::zeroed();
            wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
            wc.lpfnWndProc = Some(window_proc);
            wc.hInstance = instance;
            wc.lpszClassName = class.as_ptr();
            RegisterClassExW(&wc);
            let title = wide(key);
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            if hwnd.is_null() {
                log::warn!("the tray window could not be made");
                return None;
            }
            RECREATED.with(|r| r.set(RegisterWindowMessageW(wide("TaskbarCreated").as_ptr())));
            let icon = LoadImageW(
                instance,
                1 as _,
                IMAGE_ICON,
                GetSystemMetrics(SM_CXSMICON),
                GetSystemMetrics(SM_CYSMICON),
                LR_DEFAULTCOLOR,
            ) as HICON;
            let mut data = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: TRAY_MESSAGE,
                hIcon: icon,
                ..Default::default()
            };
            fill(&mut data.szTip, "Nuru");
            if Shell_NotifyIconW(NIM_ADD, &data) == 0 {
                log::warn!("the tray icon could not be added");
                DestroyWindow(hwnd);
                if !icon.is_null() {
                    DestroyIcon(icon);
                }
                return None;
            }
            DATA.with(|d| *d.borrow_mut() = Some(data));
            HANDLER.with(|h| *h.borrow_mut() = Some(Box::new(handler)));
            Some(Tray { hwnd, icon })
        }
    }

    pub fn set_state(&self, playing: bool, layers: usize) {
        PLAYING.with(|p| p.set(playing && layers > 0));
        HAS_LAYERS.with(|h| h.set(layers > 0));
        let tip = match (playing, layers) {
            (true, 1) => "Nuru, playing 1 sound".to_string(),
            (true, n) if n > 1 => format!("Nuru, playing {n} sounds"),
            _ => "Nuru".to_string(),
        };
        DATA.with(|d| {
            if let Some(data) = d.borrow_mut().as_mut() {
                data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
                fill(&mut data.szTip, &tip);
                unsafe { Shell_NotifyIconW(NIM_MODIFY, data) };
            }
        });
    }

    pub fn notify(&self, text: &str) {
        DATA.with(|d| {
            if let Some(data) = d.borrow().as_ref() {
                let mut note = *data;
                note.uFlags = NIF_INFO;
                fill(&mut note.szInfo, text);
                fill(&mut note.szInfoTitle, "");
                note.dwInfoFlags = NIIF_NONE;
                unsafe { Shell_NotifyIconW(NIM_MODIFY, &note) };
            }
        });
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        DATA.with(|d| {
            if let Some(data) = d.borrow_mut().take() {
                unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
            }
        });
        HANDLER.with(|h| h.borrow_mut().take());
        unsafe {
            DestroyWindow(self.hwnd);
            if !self.icon.is_null() {
                DestroyIcon(self.icon);
            }
        }
    }
}
