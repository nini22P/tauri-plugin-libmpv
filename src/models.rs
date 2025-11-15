use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::ffi::c_void;

#[derive(Debug, Clone, Copy)]
pub struct MpvHandleWrapper(pub *mut c_void);

impl MpvHandleWrapper {
    pub fn inner(&self) -> *mut c_void {
        self.0
    }
}

unsafe impl Send for MpvHandleWrapper {}
unsafe impl Sync for MpvHandleWrapper {}

pub struct MpvInstance {
    pub handle: MpvHandleWrapper,
    pub event_userdata: MpvHandleWrapper,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvConfig {
    #[serde(default)]
    pub initial_options: IndexMap<String, serde_json::Value>,
    #[serde(default)]
    pub observed_properties: IndexMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoMarginRatio {
    pub left: Option<f64>,
    pub right: Option<f64>,
    pub top: Option<f64>,
    pub bottom: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FfiResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
