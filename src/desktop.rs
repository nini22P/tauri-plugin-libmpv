use log::{error, info, trace, warn};
use raw_window_handle::HasWindowHandle;
use scopeguard::defer;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tauri::Emitter;
use tauri::{AppHandle, Manager, Runtime, plugin::PluginApi};

use crate::Error;
use crate::Result;
use crate::models::*;
use crate::utils::get_wid;
use crate::wrapper::LibmpvWrapper;

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<Mpv<R>> {
    info!("Plugin registered.");
    let mpv = Mpv {
        app: app.clone(),
        instances: Mutex::new(HashMap::new()),
        wrapper: OnceLock::new(),
    };
    Ok(mpv)
}

pub unsafe extern "C" fn event_callback<R: Runtime>(event: *const c_char, userdata: *mut c_void) {
    if event.is_null() || userdata.is_null() {
        return;
    }

    let event_string = unsafe { CStr::from_ptr(event).to_string_lossy().to_string() };
    let (app, window_label) = unsafe { (*(userdata as *const (AppHandle<R>, String))).clone() };

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

pub struct Mpv<R: Runtime> {
    app: AppHandle<R>,
    pub instances: Mutex<HashMap<String, MpvInstance>>,
    pub wrapper: OnceLock<Result<LibmpvWrapper>>,
}

impl<R: Runtime> Mpv<R> {
    pub fn init(&self, mpv_config: MpvConfig, window_label: &str) -> Result<String> {
        self.init_wid_mode(mpv_config, window_label)?;
        Ok(window_label.to_string())
    }

    fn init_wid_mode(&self, mpv_config: MpvConfig, window_label: &str) -> Result<String> {
        let app = self.app.clone();

        let wrapper = self.get_wrapper()?;

        let mut initial_options = mpv_config.initial_options.clone();

        if !initial_options.contains_key("wid") {
            let window = self
                .app
                .get_webview_window(window_label)
                .ok_or_else(|| crate::Error::WindowNotFound(window_label.to_string()))?;
            let window_handle = window.window_handle()?;
            let raw_window_handle = window_handle.as_raw();
            let wid = get_wid(raw_window_handle)?;
            initial_options.insert("wid".to_string(), serde_json::json!(wid));
        }

        let Some(mut instances_lock) = self.lock_and_check_existence(window_label)? else {
            return Ok(window_label.to_string());
        };

        let initial_options_string = serde_json::to_string(&initial_options)?;
        let observed_properties_string = serde_json::to_string(&mpv_config.observed_properties)?;

        let c_initial_options = CString::new(initial_options_string)?;
        let c_observed_properties = CString::new(observed_properties_string)?;

        let event_callback_data = Box::new((app.clone(), window_label.to_string()));
        let event_userdata = Box::into_raw(event_callback_data) as *mut c_void;

        let mpv_handle = unsafe {
            wrapper.mpv_wrapper_create(
                c_initial_options.as_ptr(),
                c_observed_properties.as_ptr(),
                Some(event_callback::<R>),
                event_userdata,
            )
        };

        if mpv_handle.is_null() {
            let _ = unsafe { Box::from_raw(event_userdata as *mut (AppHandle<R>, String)) };
            return Err(crate::Error::CreateInstance);
        }

        info!("mpv instance initialized for window '{}'.", window_label);

        let instance = MpvInstance {
            handle: mpv_handle,
            event_userdata: event_userdata,
        };

        instances_lock.insert(window_label.to_string(), instance);

        info!("Wid mode initialized for window '{}'.", window_label);

        Ok(window_label.to_string())
    }

    pub fn destroy(&self, window_label: &str) -> Result<()> {
        if let Some(instance) = self.remove_instance(window_label)? {
            let wrapper = self.get_wrapper()?;

            unsafe {
                wrapper.mpv_wrapper_destroy(instance.handle);
            }

            let _ =
                unsafe { Box::from_raw(instance.event_userdata as *mut (AppHandle<R>, String)) };

            info!(
                "mpv instance for window '{}' has been destroyed.",
                window_label,
            );
        } else {
            trace!(
                "No running mpv instance found for window '{}' to destroy.",
                window_label
            );
        }
        Ok(())
    }

    pub fn command(
        &self,
        name: &str,
        args: &Vec<serde_json::Value>,
        window_label: &str,
    ) -> Result<()> {
        if args.is_empty() {
            trace!("COMMAND '{}'", name);
        } else {
            trace!("COMMAND '{}' '{:?}'", name, args);
        }

        self.with_instance(window_label, |instance| {
            let wrapper = self.get_wrapper()?;

            let args_string = serde_json::to_string(&args)?;

            let c_name = CString::new(name)?;
            let c_args = CString::new(args_string)?;

            let result_ptr = unsafe {
                wrapper.mpv_wrapper_command(instance.handle, c_name.as_ptr(), c_args.as_ptr())
            };

            if result_ptr.is_null() {
                return Err(crate::Error::FFI("Call returned null pointer".into()));
            }

            defer! {
                unsafe { wrapper.mpv_wrapper_free_string(result_ptr) };
            }

            let response_str = unsafe { CStr::from_ptr(result_ptr).to_string_lossy() };
            let response: FfiResponse = serde_json::from_str(&response_str)?;

            if let Some(err) = response.error {
                Err(crate::Error::Command {
                    window_label: window_label.to_string(),
                    message: err,
                })
            } else {
                Ok(())
            }
        })
    }

    pub fn set_property(
        &self,
        name: &str,
        value: &serde_json::Value,
        window_label: &str,
    ) -> crate::Result<()> {
        trace!("SET PROPERTY '{}' '{:?}'", name, value);

        self.with_instance(window_label, |instance| {
            let wrapper = self.get_wrapper()?;

            let value_string = serde_json::to_string(value)?;

            let c_name = CString::new(name)?;
            let c_value = CString::new(value_string)?;

            let result_ptr = unsafe {
                wrapper.mpv_wrapper_set_property(instance.handle, c_name.as_ptr(), c_value.as_ptr())
            };

            if result_ptr.is_null() {
                return Err(crate::Error::FFI("Call returned null pointer".into()));
            }

            defer! {
                unsafe { wrapper.mpv_wrapper_free_string(result_ptr) };
            }

            let response_str = unsafe { CStr::from_ptr(result_ptr).to_string_lossy() };
            let response: FfiResponse = serde_json::from_str(&response_str)?;

            if let Some(err) = response.error {
                Err(crate::Error::SetProperty {
                    window_label: window_label.to_string(),
                    message: err,
                })
            } else {
                Ok(())
            }
        })
    }

    pub fn get_property(
        &self,
        name: String,
        format: String,
        window_label: &str,
    ) -> crate::Result<serde_json::Value> {
        self.with_instance(window_label, |instance| {
            let wrapper = self.get_wrapper()?;

            let c_name = CString::new(name.clone())?;
            let c_format = CString::new(format.as_str())?;

            let result_ptr = unsafe {
                wrapper.mpv_wrapper_get_property(
                    instance.handle,
                    c_name.as_ptr(),
                    c_format.as_ptr(),
                )
            };

            defer! {
                unsafe { wrapper.mpv_wrapper_free_string(result_ptr) };
            }

            let response_str = unsafe {
                if result_ptr.is_null() {
                    return Err(crate::Error::GetProperty {
                        window_label: window_label.to_string(),
                        message: "FFI call returned null pointer".into(),
                    });
                }
                CStr::from_ptr(result_ptr).to_string_lossy()
            };

            let response: FfiResponse = serde_json::from_str(&response_str)?;

            if let Some(err) = response.error {
                return Err(crate::Error::GetProperty {
                    window_label: window_label.to_string(),
                    message: err,
                });
            }

            let value = response.data.ok_or_else(|| crate::Error::GetProperty {
                window_label: window_label.to_string(),
                message: "FFI response contained no data".to_string(),
            })?;

            trace!("GET PROPERTY '{}' '{:?}'", name, value);
            Ok(value)
        })
    }

    pub fn set_video_margin_ratio(
        &self,
        ratio: VideoMarginRatio,
        window_label: &str,
    ) -> Result<()> {
        trace!("SET VIDEO MARGIN RATIO '{:?}'", ratio);

        let margins = [
            ("video-margin-ratio-left", ratio.left),
            ("video-margin-ratio-right", ratio.right),
            ("video-margin-ratio-top", ratio.top),
            ("video-margin-ratio-bottom", ratio.bottom),
        ];

        for (property, value_option) in margins {
            if let Some(value) = value_option {
                self.set_property(property, &serde_json::json!(value), window_label)?;
            }
        }
        Ok(())
    }

    fn lock_and_check_existence<'a>(
        &'a self,
        window_label: &str,
    ) -> Result<Option<std::sync::MutexGuard<'a, HashMap<String, MpvInstance>>>> {
        let instances_lock = match self.instances.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if instances_lock.contains_key(window_label) {
            info!(
                "mpv instance for window '{}' already exists. Skipping initialization.",
                window_label
            );
            Ok(None)
        } else {
            Ok(Some(instances_lock))
        }
    }

    fn with_instance<F, T>(&self, window_label: &str, operation: F) -> Result<T>
    where
        F: FnOnce(&MpvInstance) -> Result<T>,
    {
        let instances_lock = match self.instances.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Mutex was poisoned, recovering.");
                poisoned.into_inner()
            }
        };

        let instance = instances_lock.get(window_label).ok_or_else(|| {
            crate::Error::InstanceNotFound(format!(
                "mpv instance for window label '{}' not found",
                window_label
            ))
        })?;

        operation(instance)
    }

    fn remove_instance(&self, window_label: &str) -> Result<Option<MpvInstance>> {
        let mut instances_lock = match self.instances.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Mutex was poisoned, recovering.");
                poisoned.into_inner()
            }
        };
        Ok(instances_lock.remove(window_label))
    }

    fn get_wrapper(&self) -> Result<&LibmpvWrapper> {
        let result = self.wrapper.get_or_init(|| {
            info!("libmpv-wrapper not initialized. Trying to load libmpv-wrapper now...");

            #[cfg(target_os = "windows")]
            let lib_name = "libmpv-wrapper.dll";
            #[cfg(target_os = "macos")]
            let lib_name = "libmpv-wrapper.dylib";
            #[cfg(target_os = "linux")]
            let lib_name = "libmpv-wrapper.so";

            let mut search_paths: Vec<PathBuf> = Vec::new();

            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    search_paths.push(exe_dir.to_path_buf());
                }
            }

            search_paths.push(PathBuf::new());

            for path in &search_paths {
                let full_lib_path: String = if path.as_os_str().is_empty() {
                    lib_name.to_string()
                } else {
                    path.join(lib_name).to_string_lossy().into_owned()
                };

                let load_result = unsafe { LibmpvWrapper::new(&full_lib_path) };

                if load_result.is_ok() {
                    info!("Successfully loaded libmpv-wrapper from: {}", full_lib_path);
                    return load_result.map_err(Into::into);
                }
            }

            Err(Error::FFI(format!(
                "Failed to load libmpv-wrapper. Tried the following paths: {}",
                search_paths
                    .iter()
                    .map(|p| p.join(lib_name).to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join(", ")
            )))
            .map_err(Into::into)
        });

        match result {
            Ok(wrapper) => Ok(wrapper),
            Err(e) => Err(Error::FFI(format!(
                "Failed to get wrapper (it may have failed to load on first attempt): {}",
                e
            ))),
        }
    }
}
