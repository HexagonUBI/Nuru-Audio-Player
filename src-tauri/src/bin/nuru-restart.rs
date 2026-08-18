#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use windows_sys::core::PCWSTR;
use windows_sys::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect,
    InvalidateRect, RoundRect, SelectObject, SetBkMode, SetTextColor, CLEARTYPE_QUALITY,
    DT_LEFT, DT_SINGLELINE, FF_DONTCARE, FW_NORMAL, FW_SEMIBOLD, HBRUSH, PAINTSTRUCT,
    TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{
    OpenProcess, WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetSystemMetrics,
    KillTimer, LoadCursorW, PostQuitMessage, RegisterClassW, SetTimer, ShowWindow, TranslateMessage,
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, IDC_ARROW, MSG, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW,
    WM_DESTROY, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    WS_VISIBLE,
};

const WIDTH: i32 = 380;
const HEIGHT: i32 = 132;
const TIMER_ID: usize = 1;

const BG: COLORREF = 0x00_16_14_13;
const INK: COLORREF = 0x00_F6_F4_F3;
const DIM: COLORREF = 0x00_8A_86_84;
const ACCENT: COLORREF = 0x00_54_B4_FF;
const TRACK: COLORREF = 0x00_2A_26_24;

static PHASE: AtomicU32 = AtomicU32::new(0);
static TICK: AtomicU32 = AtomicU32::new(0);

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn phase_label(phase: u32) -> &'static str {
    match phase {
        0 => "Closing Nuru",
        1 => "Installing the update",
        2 => "Starting Nuru",
        _ => "Done",
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER => {
            TICK.fetch_add(1, Ordering::Relaxed);
            unsafe { InvalidateRect(hwnd, std::ptr::null(), 0) };
            0
        }
        WM_PAINT => {
            unsafe { paint(hwnd) };
            0
        }
        WM_DESTROY => {
            unsafe {
                KillTimer(hwnd, TIMER_ID);
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

unsafe fn paint(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };

    let full = RECT { left: 0, top: 0, right: WIDTH, bottom: HEIGHT };
    let bg = unsafe { CreateSolidBrush(BG) };
    unsafe { FillRect(hdc, &full, bg as HBRUSH) };
    unsafe { DeleteObject(bg as _) };

    unsafe { SetBkMode(hdc, TRANSPARENT as i32) };

    let title_font = unsafe {
        CreateFontW(
            -20, 0, 0, 0, FW_SEMIBOLD as i32, 0, 0, 0, 0, 0, 0,
            CLEARTYPE_QUALITY as u32, FF_DONTCARE as u32, wide("Segoe UI").as_ptr(),
        )
    };
    let small_font = unsafe {
        CreateFontW(
            -13, 0, 0, 0, FW_NORMAL as i32, 0, 0, 0, 0, 0, 0,
            CLEARTYPE_QUALITY as u32, FF_DONTCARE as u32, wide("Segoe UI").as_ptr(),
        )
    };

    let old = unsafe { SelectObject(hdc, title_font as _) };
    unsafe { SetTextColor(hdc, INK) };
    let mut r = RECT { left: 28, top: 30, right: WIDTH - 28, bottom: 58 };
    let mut title = wide("Updating Nuru");
    unsafe { DrawTextW(hdc, title.as_mut_ptr(), -1, &mut r, DT_LEFT | DT_SINGLELINE) };

    unsafe { SelectObject(hdc, small_font as _) };
    unsafe { SetTextColor(hdc, DIM) };
    let mut r2 = RECT { left: 28, top: 58, right: WIDTH - 28, bottom: 78 };
    let mut sub = wide(phase_label(PHASE.load(Ordering::Relaxed)));
    unsafe { DrawTextW(hdc, sub.as_mut_ptr(), -1, &mut r2, DT_LEFT | DT_SINGLELINE) };

    let bar_left = 28;
    let bar_right = WIDTH - 28;
    let bar_top = 94;
    let bar_bottom = 100;

    let track = unsafe { CreateSolidBrush(TRACK) };
    let old_brush = unsafe { SelectObject(hdc, track as _) };
    unsafe { RoundRect(hdc, bar_left, bar_top, bar_right, bar_bottom, 6, 6) };
    unsafe { SelectObject(hdc, old_brush) };
    unsafe { DeleteObject(track as _) };

    // An indeterminate sweep. There is no useful percentage here: the installer
    // does not report progress, so a moving band is honest where a filling bar
    // would not be.
    let span = bar_right - bar_left;
    let seg = span / 3;
    let t = (TICK.load(Ordering::Relaxed) % 90) as i32;
    let travel = span + seg;
    let x = bar_left - seg + (travel * t / 90);
    let x0 = x.max(bar_left);
    let x1 = (x + seg).min(bar_right);

    if x1 > x0 {
        let fill = unsafe { CreateSolidBrush(ACCENT) };
        let ob = unsafe { SelectObject(hdc, fill as _) };
        unsafe { RoundRect(hdc, x0, bar_top, x1, bar_bottom, 6, 6) };
        unsafe { SelectObject(hdc, ob) };
        unsafe { DeleteObject(fill as _) };
    }

    unsafe { SelectObject(hdc, old) };
    unsafe { DeleteObject(title_font as _) };
    unsafe { DeleteObject(small_font as _) };
    unsafe { EndPaint(hwnd, &ps) };
}

struct Args {
    installer: String,
    exe: String,
    pid: u32,
}

fn parse_args() -> Option<Args> {
    let mut installer = None;
    let mut exe = None;
    let mut pid = None;
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--installer" => installer = it.next(),
            "--exe" => exe = it.next(),
            "--pid" => pid = it.next().and_then(|v| v.parse().ok()),
            _ => {}
        }
    }
    Some(Args { installer: installer?, exe: exe?, pid: pid? })
}

fn run_update(args: Arc<Args>, hwnd: usize) {
    if args.pid != 0 {
        unsafe {
            let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, args.pid);
            if !handle.is_null() {
                WaitForSingleObject(handle, INFINITE);
            }
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(400));

    PHASE.store(1, Ordering::Relaxed);

    // /S only. This process owns the relaunch, so the installer must not also
    // try it with /R, or two starts race and one of them wins silently.
    let status = std::process::Command::new(&args.installer)
        .arg("/S")
        .arg("/UPDATE")
        .status();

    if let Err(e) = &status {
        eprintln!("installer failed: {e}");
    }

    PHASE.store(2, Ordering::Relaxed);
    std::thread::sleep(std::time::Duration::from_millis(500));

    let exe = std::path::Path::new(&args.exe);
    let target = if exe.is_file() {
        exe.to_path_buf()
    } else {
        let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
        std::path::Path::new(&local).join("Nuru").join("nuru.exe")
    };

    let _ = std::process::Command::new(&target).spawn();

    std::thread::sleep(std::time::Duration::from_millis(900));
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(hwnd as HWND, WM_DESTROY, 0, 0)
    };
}

fn main() {
    let Some(args) = parse_args() else {
        eprintln!("usage: nuru-restart --installer <path> --exe <path> --pid <pid>");
        std::process::exit(2);
    };
    let args = Arc::new(args);

    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class_name = wide("NuruRestartWindow");

        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW as PCWSTR),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        RegisterClassW(&class);

        let x = (GetSystemMetrics(SM_CXSCREEN) - WIDTH) / 2;
        let y = (GetSystemMetrics(SM_CYSCREEN) - HEIGHT) / 2;

        // WS_POPUP: no titlebar, no border, and nothing to drag a resize from.
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            wide("Updating Nuru").as_ptr(),
            WS_POPUP | WS_VISIBLE,
            if x > 0 { x } else { CW_USEDEFAULT },
            if y > 0 { y } else { CW_USEDEFAULT },
            WIDTH,
            HEIGHT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );

        ShowWindow(hwnd, SW_SHOW);
        SetTimer(hwnd, TIMER_ID, 16, None);

        let worker_hwnd = hwnd as usize;
        let worker_args = args.clone();
        std::thread::spawn(move || run_update(worker_args, worker_hwnd));

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
