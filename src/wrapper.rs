use libloading::{Library, Symbol};
use log::error;
use std::ffi::{c_char, c_void, CStr};
use tauri::{AppHandle, Emitter, Runtime};

use crate::{Error, Result};

pub type EventCallback = unsafe extern "C" fn(event: *const c_char, userdata: *mut c_void);

type FnCreate = unsafe extern "C" fn(
    initial_options: *const c_char,
    observed_properties: *const c_char,
    event_callback: EventCallback,
    event_userdata: *mut c_void,
) -> *mut c_void;

type FnDestroy = unsafe extern "C" fn(mpv: *mut c_void);

type FnCommand =
    unsafe extern "C" fn(mpv: *mut c_void, name: *const c_char, args: *const c_char) -> *mut c_char;

type FnSetProperty = unsafe extern "C" fn(
    mpv: *mut c_void,
    name: *const c_char,
    value: *const c_char,
) -> *mut c_char;

type FnGetProperty = unsafe extern "C" fn(
    mpv: *mut c_void,
    name: *const c_char,
    format: *const c_char,
) -> *mut c_char;

type FnFreeString = unsafe extern "C" fn(s: *mut c_char);

pub unsafe extern "C" fn event_callback<R: Runtime>(event: *const c_char, userdata: *mut c_void) {
    if event.is_null() || userdata.is_null() {
        return;
    }

    let event_string = CStr::from_ptr(event).to_string_lossy().to_string();
    let (app, window_label) = (*(userdata as *const (AppHandle<R>, String))).clone();

    tauri::async_runtime::spawn(async move {
        match serde_json::from_str::<serde_json::Value>(&event_string) {
            Ok(event) => {
                let event_name = format!("mpv-event-{}", window_label);
                if let Err(e) = app.emit_to(&window_label, &event_name, &event) {
                    error!("Failed to emit mpv event to frontend: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to deserialize mpv FFI event: {}", e);
            }
        }
    });
}

pub struct Wrapper {
    _lib: &'static Library,
    pub mpv_create: Symbol<'static, FnCreate>,
    pub mpv_destroy: Symbol<'static, FnDestroy>,
    pub mpv_command: Symbol<'static, FnCommand>,
    pub mpv_set_property: Symbol<'static, FnSetProperty>,
    pub mpv_get_property: Symbol<'static, FnGetProperty>,
    pub mpv_free_string: Symbol<'static, FnFreeString>,
}

impl Wrapper {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "windows")]
        let lib_name = "libmpv_wrapper.dll";
        #[cfg(target_os = "macos")]
        let lib_name = "libmpv_wrapper.dylib";
        #[cfg(target_os = "linux")]
        let lib_name = "libmpv_wrapper.so";

        unsafe {
            let lib = Library::new(lib_name).map_err(|e| {
                let error = format!("Failed to load {:?} : {}", lib_name, e);
                error!("{}", error);
                Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, error))
            })?;

            let lib: &'static Library = Box::leak(Box::new(lib));

            let mpv_create = lib.get(b"mpv_wrapper_create")?;
            let mpv_destroy = lib.get(b"mpv_wrapper_destroy")?;
            let mpv_command = lib.get(b"mpv_wrapper_command")?;
            let mpv_set_property = lib.get(b"mpv_wrapper_set_property")?;
            let mpv_get_property = lib.get(b"mpv_wrapper_get_property")?;
            let mpv_free_string = lib.get(b"mpv_wrapper_free_string")?;

            Ok(Self {
                _lib: lib,
                mpv_create,
                mpv_destroy,
                mpv_command,
                mpv_set_property,
                mpv_get_property,
                mpv_free_string,
            })
        }
    }
}
