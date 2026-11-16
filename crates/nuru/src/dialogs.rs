use std::ffi::c_void;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

#[repr(C)]
struct OpenFileName {
    struct_size: u32,
    owner: *mut c_void,
    instance: *mut c_void,
    filter: *const u16,
    custom_filter: *mut u16,
    max_custom_filter: u32,
    filter_index: u32,
    file: *mut u16,
    max_file: u32,
    file_title: *mut u16,
    max_file_title: u32,
    initial_dir: *const u16,
    title: *const u16,
    flags: u32,
    file_offset: u16,
    file_extension: u16,
    default_ext: *const u16,
    custom_data: isize,
    hook: *mut c_void,
    template_name: *const u16,
    reserved_ptr: *mut c_void,
    reserved: u32,
    flags_ex: u32,
}

#[link(name = "comdlg32")]
extern "system" {
    fn GetOpenFileNameW(ofn: *mut OpenFileName) -> i32;
}

#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(reserved: *const c_void, model: u32) -> i32;
    fn CoUninitialize();
}

const OFN_NOCHANGEDIR: u32 = 0x8;
const OFN_PATHMUSTEXIST: u32 = 0x800;
const OFN_FILEMUSTEXIST: u32 = 0x1000;
const OFN_EXPLORER: u32 = 0x80000;
const COINIT_APARTMENTTHREADED: u32 = 0x2;

fn open_file() -> Option<PathBuf> {
    let filter: Vec<u16> = "Audio\0*.flac;*.wav;*.ogg;*.mp3\0\0".encode_utf16().collect();
    let mut file = vec![0u16; 4096];
    let mut ofn = OpenFileName {
        struct_size: std::mem::size_of::<OpenFileName>() as u32,
        owner: std::ptr::null_mut(),
        instance: std::ptr::null_mut(),
        filter: filter.as_ptr(),
        custom_filter: std::ptr::null_mut(),
        max_custom_filter: 0,
        filter_index: 1,
        file: file.as_mut_ptr(),
        max_file: file.len() as u32,
        file_title: std::ptr::null_mut(),
        max_file_title: 0,
        initial_dir: std::ptr::null(),
        title: std::ptr::null(),
        flags: OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        file_offset: 0,
        file_extension: 0,
        default_ext: std::ptr::null(),
        custom_data: 0,
        hook: std::ptr::null_mut(),
        template_name: std::ptr::null(),
        reserved_ptr: std::ptr::null_mut(),
        reserved: 0,
        flags_ex: 0,
    };
    let picked = unsafe { GetOpenFileNameW(&mut ofn) } != 0;
    if !picked {
        return None;
    }
    let end = file.iter().position(|&c| c == 0).unwrap_or(file.len());
    Some(PathBuf::from(std::ffi::OsString::from_wide(&file[..end])))
}

pub fn pick_audio(done: impl FnOnce(Option<PathBuf>) + Send + 'static) {
    let started = std::thread::Builder::new().name("nuru-picker".into()).spawn(move || {
        let com = unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED) } >= 0;
        let picked = open_file();
        if com {
            unsafe { CoUninitialize() };
        }
        done(picked);
    });
    if let Err(e) = started {
        log::warn!("the file picker could not start: {e}");
    }
}
