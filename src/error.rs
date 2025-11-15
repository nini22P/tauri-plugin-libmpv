use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[cfg(mobile)]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error("Unsupported platform {0}")]
    UnsupportedPlatform(String),
    #[error("Not found window with label: '{0}'")]
    WindowNotFound(String),
    #[error("Failed to get window handle: {0}")]
    WindowHandle(#[from] raw_window_handle::HandleError),
    #[error("FFI error: {0}")]
    FFI(String),
    #[error("Failed to create mpv instance")]
    CreateInstance,
    #[error("mpv instance not found: {0}")]
    InstanceNotFound(String),
    #[error(transparent)]
    Libloading(#[from] libloading::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    NulError(#[from] std::ffi::NulError),
    #[error("Command failed for window '{window_label}': {message}")]
    Command {
        window_label: String,
        message: String,
    },
    #[error("Set Property failed for window '{window_label}': {message}")]
    SetProperty {
        window_label: String,
        message: String,
    },
    #[error("Get Property failed for window '{window_label}': {message}")]
    GetProperty {
        window_label: String,
        message: String,
    },
    #[error("Invalid value for property '{name}': {message}")]
    InvalidPropertyValue { name: String, message: String },
    #[error("Failed to destroy mpv instance: {0}")]
    Destroy(String),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
