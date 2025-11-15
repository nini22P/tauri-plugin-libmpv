use log::{info, trace, warn};
use raw_window_handle::HasWindowHandle;
use scopeguard::defer;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::ffi::{c_void, CStr, CString};
use std::sync::Mutex;
use tauri::{plugin::PluginApi, AppHandle, Manager, Runtime};

use crate::bridge::{event_callback, MpvBridge};
use crate::models::*;
use crate::utils::get_wid;
use crate::Result;

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<Mpv<R>> {
    info!("Initializing libmpv-wrapper bridge...");
    let bridge = MpvBridge::new()?;
    info!("Plugin registered.");
    let mpv = Mpv {
        app: app.clone(),
        instances: Mutex::new(HashMap::new()),
        bridge: Mutex::new(bridge),
    };
    Ok(mpv)
}

pub struct Mpv<R: Runtime> {
    app: AppHandle<R>,
    pub instances: Mutex<HashMap<String, MpvInstance>>,
    pub bridge: Mutex<MpvBridge>,
}

impl<R: Runtime> Mpv<R> {
    pub fn init(&self, mpv_config: MpvConfig, window_label: &str) -> Result<String> {
        self.init_wid_mode(mpv_config, window_label)?;
        Ok(window_label.to_string())
    }

    fn init_wid_mode(&self, mpv_config: MpvConfig, window_label: &str) -> Result<String> {
        let app = self.app.clone();
        let bridge = self.bridge.lock().unwrap();

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
            (bridge.mpv_create)(
                c_initial_options.as_ptr(),
                c_observed_properties.as_ptr(),
                event_callback::<R>,
                event_userdata,
            )
        };

        if mpv_handle.is_null() {
            let _ = unsafe { Box::from_raw(event_userdata as *mut (AppHandle<R>, String)) };
            return Err(crate::Error::CreateInstance);
        }

        info!("mpv instance initialized for window '{}'.", window_label);

        let instance = MpvInstance {
            handle: MpvHandleWrapper(mpv_handle),
            event_userdata: MpvHandleWrapper(event_userdata),
        };

        instances_lock.insert(window_label.to_string(), instance);

        info!("Wid mode initialized for window '{}'.", window_label);

        Ok(window_label.to_string())
    }

    pub fn destroy(&self, window_label: &str) -> Result<()> {
        if let Some(instance) = self.remove_instance(window_label)? {
            let bridge = self.bridge.lock().unwrap();

            unsafe {
                (bridge.mpv_destroy)(instance.handle.inner());
            }

            let _ = unsafe {
                Box::from_raw(instance.event_userdata.inner() as *mut (AppHandle<R>, String))
            };

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
            let bridge = self.bridge.lock().unwrap();

            let args_string = serde_json::to_string(&args)?;

            let c_name = CString::new(name)?;
            let c_args = CString::new(args_string)?;

            let result_ptr = unsafe {
                (bridge.mpv_command)(instance.handle.inner(), c_name.as_ptr(), c_args.as_ptr())
            };

            if result_ptr.is_null() {
                return Err(crate::Error::FFI("Call returned null pointer".into()));
            }

            defer! {
                unsafe { (bridge.mpv_free_string)(result_ptr) };
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
            let bridge = self.bridge.lock().unwrap();

            let value_string = serde_json::to_string(value)?;

            let c_name = CString::new(name)?;
            let c_value = CString::new(value_string)?;

            let result_ptr = unsafe {
                (bridge.mpv_set_property)(
                    instance.handle.inner(),
                    c_name.as_ptr(),
                    c_value.as_ptr(),
                )
            };

            if result_ptr.is_null() {
                return Err(crate::Error::FFI("Call returned null pointer".into()));
            }

            defer! {
                unsafe { (bridge.mpv_free_string)(result_ptr) };
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
            let bridge = self.bridge.lock().unwrap();

            let c_name = CString::new(name.clone())?;
            let c_format = CString::new(format.as_str())?;

            let result_ptr = unsafe {
                (bridge.mpv_get_property)(
                    instance.handle.inner(),
                    c_name.as_ptr(),
                    c_format.as_ptr(),
                )
            };

            defer! {
                unsafe { (bridge.mpv_free_string)(result_ptr) };
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
}
