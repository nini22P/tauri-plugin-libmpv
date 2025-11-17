use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::ffi::c_void;

use crate::wrapper::MpvHandle;

#[derive(Debug, Clone, Copy)]
pub struct MpvInstance {
    pub handle: *mut MpvHandle,
    pub event_userdata: *mut c_void,
}

unsafe impl Send for MpvInstance {}
unsafe impl Sync for MpvInstance {}

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
