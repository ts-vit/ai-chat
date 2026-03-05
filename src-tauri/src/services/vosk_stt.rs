use std::ffi::{c_char, c_float, c_int, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use libloading::Library;

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::SetDllDirectoryW;

type VoskModelPtr = *mut c_void;
type VoskRecognizerPtr = *mut c_void;

type FnSetLogLevel = unsafe extern "C" fn(c_int);
type FnModelNew = unsafe extern "C" fn(*const c_char) -> VoskModelPtr;
type FnModelFree = unsafe extern "C" fn(VoskModelPtr);
type FnRecognizerNew = unsafe extern "C" fn(VoskModelPtr, c_float) -> VoskRecognizerPtr;
type FnRecognizerFree = unsafe extern "C" fn(VoskRecognizerPtr);
type FnAcceptWaveform =
    unsafe extern "C" fn(VoskRecognizerPtr, *const c_char, c_int) -> c_int;
type FnFinalResult = unsafe extern "C" fn(VoskRecognizerPtr) -> *const c_char;

struct VoskLibraryInner {
    _lib: Library,
    set_log_level: FnSetLogLevel,
    model_new: FnModelNew,
    model_free: FnModelFree,
    recognizer_new: FnRecognizerNew,
    recognizer_free: FnRecognizerFree,
    accept_waveform: FnAcceptWaveform,
    final_result: FnFinalResult,
}

// Safety: Vosk FFI functions are thread-safe per documentation
unsafe impl Send for VoskLibraryInner {}
unsafe impl Sync for VoskLibraryInner {}

pub struct VoskLibrary {
    inner: Arc<VoskLibraryInner>,
}

impl Clone for VoskLibrary {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

static LOADED_LIB: OnceLock<Mutex<Option<VoskLibrary>>> = OnceLock::new();

fn lib_cache() -> &'static Mutex<Option<VoskLibrary>> {
    LOADED_LIB.get_or_init(|| Mutex::new(None))
}

struct CachedVoskModel {
    path: String,
    model_ptr: VoskModelPtr,
    lib: VoskLibrary,
}

impl Drop for CachedVoskModel {
    fn drop(&mut self) {
        if !self.model_ptr.is_null() {
            unsafe { (self.lib.inner.model_free)(self.model_ptr) };
        }
    }
}

// Safety: VoskModel is thread-safe per Vosk docs
unsafe impl Send for CachedVoskModel {}
unsafe impl Sync for CachedVoskModel {}

static CACHED_MODEL: OnceLock<Mutex<Option<CachedVoskModel>>> = OnceLock::new();

fn model_cache() -> &'static Mutex<Option<CachedVoskModel>> {
    CACHED_MODEL.get_or_init(|| Mutex::new(None))
}

/// Load the library. On Windows, adds the DLL directory to the search path first
/// so dependencies (libwinpthread, libgcc, etc.) are found in the same folder.
fn load_library_with_dll_dir(dll_path: &Path) -> Result<Library, String> {
    #[cfg(target_os = "windows")]
    {
        let dll_dir = dll_path
            .parent()
            .ok_or_else(|| "Invalid DLL path (no parent)".to_string())?;
        let wide: Vec<u16> = dll_dir
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            SetDllDirectoryW(wide.as_ptr());
        }
        if let Ok(entries) = std::fs::read_dir(&dll_dir) {
            for entry in entries.flatten() {
                log::info!("Vosk dir file: {:?}", entry.file_name());
            }
        }
        let result = unsafe { Library::new(dll_path) };
        unsafe {
            SetDllDirectoryW(std::ptr::null());
        }
        match result {
            Ok(lib) => return Ok(lib),
            Err(e) => {
                let os_err = std::io::Error::last_os_error();
                log::error!("Failed to load libvosk.dll: {}, OS error: {}", e, os_err);
                return Err(format!(
                    "Failed to load Vosk library from {:?}: {} (OS error: {} [code {}])",
                    dll_path,
                    e,
                    os_err,
                    os_err.raw_os_error().unwrap_or(-1)
                ));
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        unsafe { Library::new(dll_path) }
            .map_err(|e| format!("Failed to load Vosk library from {:?}: {}", dll_path, e))
    }
}

impl VoskLibrary {
    pub fn load(dll_path: &Path) -> Result<Self, String> {
        let lib = load_library_with_dll_dir(dll_path)?;

        unsafe {
            let fn_set_log_level: FnSetLogLevel = *lib
                .get::<FnSetLogLevel>(b"vosk_set_log_level\0")
                .map_err(|e| format!("Symbol vosk_set_log_level not found: {}", e))?;
            let fn_model_new: FnModelNew = *lib
                .get::<FnModelNew>(b"vosk_model_new\0")
                .map_err(|e| format!("Symbol vosk_model_new not found: {}", e))?;
            let fn_model_free: FnModelFree = *lib
                .get::<FnModelFree>(b"vosk_model_free\0")
                .map_err(|e| format!("Symbol vosk_model_free not found: {}", e))?;
            let fn_recognizer_new: FnRecognizerNew = *lib
                .get::<FnRecognizerNew>(b"vosk_recognizer_new\0")
                .map_err(|e| format!("Symbol vosk_recognizer_new not found: {}", e))?;
            let fn_recognizer_free: FnRecognizerFree = *lib
                .get::<FnRecognizerFree>(b"vosk_recognizer_free\0")
                .map_err(|e| format!("Symbol vosk_recognizer_free not found: {}", e))?;
            let fn_accept_waveform: FnAcceptWaveform = *lib
                .get::<FnAcceptWaveform>(b"vosk_recognizer_accept_waveform\0")
                .map_err(|e| format!("Symbol vosk_recognizer_accept_waveform not found: {}", e))?;
            let fn_final_result: FnFinalResult = *lib
                .get::<FnFinalResult>(b"vosk_recognizer_final_result\0")
                .map_err(|e| format!("Symbol vosk_recognizer_final_result not found: {}", e))?;

            fn_set_log_level(-1);

            let inner = VoskLibraryInner {
                _lib: lib,
                set_log_level: fn_set_log_level,
                model_new: fn_model_new,
                model_free: fn_model_free,
                recognizer_new: fn_recognizer_new,
                recognizer_free: fn_recognizer_free,
                accept_waveform: fn_accept_waveform,
                final_result: fn_final_result,
            };

            Ok(Self {
                inner: Arc::new(inner),
            })
        }
    }
}

/// Resolve the platform-specific library filename inside the given directory.
pub fn vosk_dll_path(vosk_dir: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        vosk_dir.join("libvosk.dll")
    }
    #[cfg(target_os = "linux")]
    {
        vosk_dir.join("libvosk.so")
    }
    #[cfg(target_os = "macos")]
    {
        vosk_dir.join("libvosk.dylib")
    }
}

/// Try to load the Vosk shared library from disk into the global cache.
pub fn load_vosk_from_disk(vosk_dir: &Path) -> Result<(), String> {
    let dll = vosk_dll_path(vosk_dir);
    if !dll.exists() {
        return Err(format!("Vosk library not found at {:?}", dll));
    }
    let library = VoskLibrary::load(&dll)?;
    let mut guard = lib_cache()
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    *guard = Some(library);
    log::info!("Vosk library loaded from {:?}", dll);
    Ok(())
}

pub fn is_vosk_loaded() -> bool {
    lib_cache()
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false)
}

/// Unload Vosk library and cached model from memory (e.g. before deleting the vosk directory).
pub fn unload_vosk() {
    if let Ok(mut guard) = model_cache().lock() {
        *guard = None;
    }
    if let Ok(mut guard) = lib_cache().lock() {
        *guard = None;
    }
    log::info!("Vosk library unloaded from memory");
}

fn get_library() -> Result<VoskLibrary, String> {
    let guard = lib_cache()
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    guard
        .as_ref()
        .cloned()
        .ok_or_else(|| "Vosk library is not loaded. Install Vosk in Settings → Audio.".to_string())
}

fn get_or_load_model(lib: &VoskLibrary, model_path: &str) -> Result<VoskModelPtr, String> {
    let mut cached = model_cache()
        .lock()
        .map_err(|e| format!("Model cache lock error: {}", e))?;

    if let Some(ref entry) = *cached {
        if entry.path == model_path {
            log::debug!("Using cached Vosk model from: {}", model_path);
            return Ok(entry.model_ptr);
        }
    }

    log::info!("Loading Vosk model from: {}", model_path);
    let c_path = CString::new(model_path)
        .map_err(|_| "Invalid model path (contains null byte)".to_string())?;
    let ptr = unsafe { (lib.inner.model_new)(c_path.as_ptr()) };
    if ptr.is_null() {
        return Err(format!("Failed to load Vosk model from: {}", model_path));
    }

    *cached = Some(CachedVoskModel {
        path: model_path.to_string(),
        model_ptr: ptr,
        lib: lib.clone(),
    });

    Ok(ptr)
}

pub struct VoskStt {
    lib: VoskLibrary,
    model_ptr: VoskModelPtr,
}

impl VoskStt {
    pub fn new(model_path: &str) -> Result<Self, String> {
        let lib = get_library()?;
        let model_ptr = get_or_load_model(&lib, model_path)?;
        Ok(Self { lib, model_ptr })
    }

    pub fn transcribe(&self, samples: &[i16], sample_rate: f32) -> Result<String, String> {
        let rec = unsafe {
            (self.lib.inner.recognizer_new)(self.model_ptr, sample_rate as c_float)
        };
        if rec.is_null() {
            return Err("Failed to create Vosk recognizer".to_string());
        }

        let chunk_size = 4000_usize;
        let data_ptr = samples.as_ptr() as *const c_char;
        let total_bytes = samples.len() * std::mem::size_of::<i16>();

        for offset in (0..total_bytes).step_by(chunk_size * std::mem::size_of::<i16>()) {
            let remaining = total_bytes - offset;
            let len = remaining.min(chunk_size * std::mem::size_of::<i16>());
            unsafe {
                (self.lib.inner.accept_waveform)(
                    rec,
                    data_ptr.add(offset),
                    len as c_int,
                );
            }
        }

        let result_ptr = unsafe { (self.lib.inner.final_result)(rec) };
        let text = if result_ptr.is_null() {
            String::new()
        } else {
            let c_str = unsafe { CStr::from_ptr(result_ptr) };
            let json_str = c_str.to_str().unwrap_or("");
            parse_vosk_text(json_str)
        };

        unsafe { (self.lib.inner.recognizer_free)(rec) };

        Ok(text)
    }
}

/// Parse the `{"text": "..."}` JSON returned by vosk_recognizer_final_result.
fn parse_vosk_text(json: &str) -> String {
    #[derive(serde::Deserialize)]
    struct VoskResult {
        text: Option<String>,
    }
    serde_json::from_str::<VoskResult>(json)
        .ok()
        .and_then(|r| r.text)
        .unwrap_or_default()
        .trim()
        .to_string()
}
