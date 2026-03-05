use std::ffi::{CString, c_char, c_void};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

type HDigDriver = *mut c_void;
type HEventSystem = *mut c_void;
type HSoundBank = *mut c_void;

#[cfg(all(target_os = "windows", miles_audio_link))]
mod ffi {
    use super::{HDigDriver, HEventSystem, HSoundBank, c_char};
    use std::ffi::c_void;

    #[link(name = "mss64")]
    unsafe extern "C" {
        pub fn AIL_startup() -> i32;
        pub fn AIL_shutdown();
        pub fn AIL_last_error() -> *const c_char;
        pub fn AIL_open_digital_driver(
            frequency: u32,
            bits: i32,
            channel: i32,
            flags: u32,
        ) -> HDigDriver;
        pub fn AIL_close_digital_driver(dig: HDigDriver);
        pub fn AIL_set_redist_directory(dir: *const c_char) -> *mut c_char;

        #[link_name = "MilesStartupEventSystem"]
        pub fn miles_startup_event_system(
            dig: HDigDriver,
            command_buf_len: i32,
            memory_buf: *mut c_char,
            memory_len: i32,
        ) -> HEventSystem;

        #[link_name = "MilesShutdownEventSystem"]
        pub fn miles_shutdown_event_system();

        #[link_name = "MilesAddSoundBank"]
        pub fn miles_add_soundbank(filename: *const c_char, name: *const c_char) -> HSoundBank;

        #[link_name = "MilesReleaseSoundBank"]
        pub fn miles_release_soundbank(bank: HSoundBank) -> i32;

        #[link_name = "MilesEnqueueEventByName"]
        pub fn miles_enqueue_event_by_name(name: *const c_char) -> u64;

        #[link_name = "MilesBeginEventQueueProcessing"]
        pub fn miles_begin_event_queue_processing() -> i32;

        #[link_name = "MilesCompleteEventQueueProcessing"]
        pub fn miles_complete_event_queue_processing() -> i32;

        #[link_name = "MilesSetEventErrorCallback"]
        pub fn miles_set_event_error_callback(callback: Option<extern "C" fn(i64, *const c_char)>);

        #[allow(dead_code)]
        #[link_name = "MilesSetVarF"]
        pub fn miles_set_var_f(context: usize, name: *const c_char, value: f32);

        #[allow(dead_code)]
        #[link_name = "MilesSetVarI"]
        pub fn miles_set_var_i(context: usize, name: *const c_char, value: i32);

        #[allow(dead_code)]
        pub fn AIL_set_listener_3D_position(dig: HDigDriver, x: f32, y: f32, z: f32);

        #[allow(dead_code)]
        pub fn AIL_set_listener_3D_orientation(
            dig: HDigDriver,
            x_face: f32,
            y_face: f32,
            z_face: f32,
            x_up: f32,
            y_up: f32,
            z_up: f32,
        );

        #[allow(dead_code)]
        pub fn AIL_set_3D_rolloff_factor(dig: HDigDriver, factor: f32);

        #[allow(dead_code)]
        pub fn AIL_sleep(ms: u32);

        #[allow(dead_code)]
        pub fn AIL_set_file_callbacks(
            open_cb: *const c_void,
            close_cb: *const c_void,
            seek_cb: *const c_void,
            read_cb: *const c_void,
            dir_cb: *const c_void,
        );
    }
}

#[cfg(all(target_os = "windows", miles_audio_link))]
extern "C" fn miles_error_callback(relevant_id: i64, resource: *const c_char) {
    let resource_text = if resource.is_null() {
        String::from("<none>")
    } else {
        // SAFETY: Miles provides a nul-terminated string pointer when non-null.
        unsafe { std::ffi::CStr::from_ptr(resource) }
            .to_string_lossy()
            .to_string()
    };
    eprintln!("Miles error callback: id={relevant_id}, resource={resource_text}");
}

fn main() {
    #[cfg(not(all(target_os = "windows", miles_audio_link)))]
    {
        eprintln!(
            "miles_smoke is unavailable: build with --features miles_audio on Windows with Miles libs present"
        );
        return;
    }

    #[cfg(all(target_os = "windows", miles_audio_link))]
    {
        run_smoke();
    }
}

#[cfg(all(target_os = "windows", miles_audio_link))]
fn run_smoke() {
    let base_dir =
        PathBuf::from(std::env::current_dir().expect("working directory should be available"));
    let bank_path = find_msscmp_bank_path(&base_dir).unwrap_or_else(|| {
        panic!("could not locate Minecraft.msscmp from runtime/assets/legacy candidates")
    });
    let redist_dir = find_miles_redist_path(&base_dir);

    println!("Miles smoke using bank: {}", bank_path.display());
    if let Some(path) = redist_dir.as_ref() {
        println!("Miles redist path: {}", path.display());
    } else {
        println!("Miles redist path: <none>");
    }

    // SAFETY: All FFI calls follow Miles API requirements and null checks.
    unsafe {
        if let Some(path) = redist_dir {
            if let Some(c_dir) = to_cstring(path.as_os_str().to_string_lossy().as_ref()) {
                let _ = ffi::AIL_set_redist_directory(c_dir.as_ptr());
            }
        }

        let startup_ok = ffi::AIL_startup();
        if startup_ok == 0 {
            panic!("AIL_startup failed: {}", miles_last_error());
        }

        let dig = ffi::AIL_open_digital_driver(44_100, 16, 2, 0);
        if dig.is_null() {
            ffi::AIL_shutdown();
            panic!("AIL_open_digital_driver failed: {}", miles_last_error());
        }

        ffi::miles_set_event_error_callback(Some(miles_error_callback));
        ffi::AIL_set_3D_rolloff_factor(dig, 1.0);
        ffi::AIL_set_listener_3D_position(dig, 0.0, 0.0, 0.0);
        ffi::AIL_set_listener_3D_orientation(dig, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0);

        let event_system =
            ffi::miles_startup_event_system(dig, 1024 * 20, std::ptr::null_mut(), 1024 * 128);
        if event_system.is_null() {
            ffi::AIL_close_digital_driver(dig);
            ffi::AIL_shutdown();
            panic!("MilesStartupEventSystem failed: {}", miles_last_error());
        }

        let bank_c = to_cstring(bank_path.to_string_lossy().as_ref())
            .expect("bank path should be representable as C string");
        let bank = ffi::miles_add_soundbank(bank_c.as_ptr(), std::ptr::null());
        if bank.is_null() {
            ffi::miles_shutdown_event_system();
            ffi::AIL_close_digital_driver(dig);
            ffi::AIL_shutdown();
            panic!("MilesAddSoundBank failed: {}", miles_last_error());
        }

        enqueue_event("Minecraft/CacheSounds");
        tick_event_queue(45);

        enqueue_event("Minecraft/random/click");
        tick_event_queue(20);
        enqueue_event("Minecraft/dig/stone");
        tick_event_queue(40);
        enqueue_event("Minecraft/dig/grass");
        tick_event_queue(40);
        enqueue_event("Minecraft/UI/press");
        tick_event_queue(45);

        let _ = ffi::miles_release_soundbank(bank);
        ffi::miles_shutdown_event_system();
        ffi::AIL_close_digital_driver(dig);
        ffi::AIL_shutdown();
    }
}

#[cfg(all(target_os = "windows", miles_audio_link))]
unsafe fn enqueue_event(name: &str) {
    let Some(name_c) = to_cstring(name) else {
        eprintln!("failed to encode event name: {name}");
        return;
    };
    let id = unsafe { ffi::miles_enqueue_event_by_name(name_c.as_ptr()) };
    println!("queued event {name} -> id {id}");
}

#[cfg(all(target_os = "windows", miles_audio_link))]
unsafe fn tick_event_queue(ticks: usize) {
    for _ in 0..ticks {
        let begin_ok = unsafe { ffi::miles_begin_event_queue_processing() };
        if begin_ok == 0 {
            eprintln!(
                "MilesBeginEventQueueProcessing error: {}",
                miles_last_error()
            );
        }

        let complete_ok = unsafe { ffi::miles_complete_event_queue_processing() };
        if complete_ok == 0 {
            eprintln!(
                "MilesCompleteEventQueueProcessing error: {}",
                miles_last_error()
            );
        }

        thread::sleep(Duration::from_millis(16));
    }
}

#[cfg(all(target_os = "windows", miles_audio_link))]
unsafe fn miles_last_error() -> String {
    let ptr = unsafe { ffi::AIL_last_error() };
    if ptr.is_null() {
        return String::from("<unknown>");
    }

    // SAFETY: Miles returns a nul-terminated error pointer.
    unsafe { std::ffi::CStr::from_ptr(ptr) }
        .to_string_lossy()
        .to_string()
}

fn to_cstring(value: &str) -> Option<CString> {
    CString::new(value.as_bytes()).ok()
}

fn find_msscmp_bank_path(base_dir: &Path) -> Option<PathBuf> {
    let candidates = [
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("banks")
            .join("minecraft.msscmp"),
        base_dir
            .join("assets")
            .join("Common")
            .join("Durango")
            .join("Sound")
            .join("Minecraft.msscmp"),
        base_dir
            .join("assets")
            .join("Commons")
            .join("Durango")
            .join("Sound")
            .join("Minecraft.msscmp"),
        base_dir.join("assets").join("Minecraft.msscmp"),
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Durango")
            .join("Sound")
            .join("Minecraft.msscmp"),
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Minecraft.msscmp"),
    ];

    candidates.into_iter().find(|path| path.exists())
}

fn find_miles_redist_path(base_dir: &Path) -> Option<PathBuf> {
    let candidates = [
        base_dir
            .join("assets")
            .join("runtime")
            .join("audio")
            .join("miles")
            .join("redist64"),
        base_dir
            .join("assets")
            .join("Common")
            .join("Windows64")
            .join("Miles")
            .join("lib")
            .join("redist64"),
        base_dir
            .join("assets")
            .join("Commons")
            .join("Windows64")
            .join("Miles")
            .join("lib")
            .join("redist64"),
        base_dir
            .join("..")
            .join("..")
            .join("LCE-Original")
            .join("Minecraft.Client")
            .join("Windows64")
            .join("Miles")
            .join("lib")
            .join("redist64"),
    ];

    candidates.into_iter().find(|path| path.exists())
}
