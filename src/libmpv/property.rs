use scopeguard::defer;
use std::ffi::CString;
use tauri_plugin_libmpv_sys as libmpv_sys;

use crate::libmpv::{utils::cstr_to_string, utils::error_string, Error, Mpv, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

fn format_to_string(format_code: libmpv_sys::mpv_format) -> String {
    match format_code {
        libmpv_sys::mpv_format_MPV_FORMAT_NONE => "MPV_FORMAT_NONE".to_string(),
        libmpv_sys::mpv_format_MPV_FORMAT_STRING => "MPV_FORMAT_STRING".to_string(),
        libmpv_sys::mpv_format_MPV_FORMAT_OSD_STRING => "MPV_FORMAT_OSD_STRING".to_string(),
        libmpv_sys::mpv_format_MPV_FORMAT_FLAG => "MPV_FORMAT_FLAG".to_string(),
        libmpv_sys::mpv_format_MPV_FORMAT_INT64 => "MPV_FORMAT_INT64".to_string(),
        libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE => "MPV_FORMAT_DOUBLE".to_string(),
        libmpv_sys::mpv_format_MPV_FORMAT_NODE => "MPV_FORMAT_NODE".to_string(),
        libmpv_sys::mpv_format_MPV_FORMAT_NODE_ARRAY => "MPV_FORMAT_NODE_ARRAY".to_string(),
        libmpv_sys::mpv_format_MPV_FORMAT_NODE_MAP => "MPV_FORMAT_NODE_MAP".to_string(),
        libmpv_sys::mpv_format_MPV_FORMAT_BYTE_ARRAY => "MPV_FORMAT_BYTE_ARRAY".to_string(),
        unknown_code => format!("Unknown format code ({})", unknown_code),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MpvFormat {
    String,
    Flag,
    Int64,
    Double,
    Node,
}

impl From<MpvFormat> for libmpv_sys::mpv_format {
    fn from(format: MpvFormat) -> Self {
        match format {
            MpvFormat::String => libmpv_sys::mpv_format_MPV_FORMAT_STRING,
            MpvFormat::Flag => libmpv_sys::mpv_format_MPV_FORMAT_FLAG,
            MpvFormat::Int64 => libmpv_sys::mpv_format_MPV_FORMAT_INT64,
            MpvFormat::Double => libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE,
            MpvFormat::Node => libmpv_sys::mpv_format_MPV_FORMAT_NODE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PropertyValue {
    String(String),
    Flag(bool),
    Int64(i64),
    Double(f64),
    Node(MpvNode),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MpvNode {
    None,
    String(String),
    Flag(bool),
    Int64(i64),
    Double(f64),
    NodeArray(Vec<MpvNode>),
    NodeMap(IndexMap<String, MpvNode>),
    ByteArray(Vec<u8>),
}

impl MpvNode {
    pub(crate) unsafe fn from_node(node: *const libmpv_sys::mpv_node) -> Result<Self> {
        match (*node).format {
            libmpv_sys::mpv_format_MPV_FORMAT_NONE => Ok(MpvNode::None),
            libmpv_sys::mpv_format_MPV_FORMAT_STRING => {
                Ok(MpvNode::String(cstr_to_string((*node).u.string)))
            }
            libmpv_sys::mpv_format_MPV_FORMAT_FLAG => Ok(MpvNode::Flag((*node).u.flag != 0)),
            libmpv_sys::mpv_format_MPV_FORMAT_INT64 => Ok(MpvNode::Int64((*node).u.int64)),
            libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE => Ok(MpvNode::Double((*node).u.double_)),
            libmpv_sys::mpv_format_MPV_FORMAT_NODE_ARRAY => {
                let list = &*(*node).u.list;
                let mut vec = Vec::with_capacity(list.num as usize);
                for i in 0..list.num {
                    let child_node = Self::from_node(list.values.add(i as usize))?;
                    vec.push(child_node);
                }
                Ok(MpvNode::NodeArray(vec))
            }
            libmpv_sys::mpv_format_MPV_FORMAT_NODE_MAP => {
                let list = &*(*node).u.list;
                let mut map = IndexMap::with_capacity(list.num as usize);
                for i in 0..list.num {
                    let key = cstr_to_string(*list.keys.add(i as usize));
                    let value_node = Self::from_node(list.values.add(i as usize))?;
                    map.insert(key, value_node);
                }
                Ok(MpvNode::NodeMap(map))
            }
            libmpv_sys::mpv_format_MPV_FORMAT_BYTE_ARRAY => {
                let ba = &*(*node).u.ba;
                let bytes = std::slice::from_raw_parts(ba.data as *const u8, ba.size).to_vec();
                Ok(MpvNode::ByteArray(bytes))
            }
            format => Err(Error::NodeConversion(format!(
                "Unsupported mpv_node format: {}",
                format_to_string(format)
            ))),
        }
    }

    pub(crate) unsafe fn from_property(property: libmpv_sys::mpv_event_property) -> Result<Self> {
        match property.format {
            libmpv_sys::mpv_format_MPV_FORMAT_NONE => Ok(MpvNode::None),
            libmpv_sys::mpv_format_MPV_FORMAT_STRING
            | libmpv_sys::mpv_format_MPV_FORMAT_OSD_STRING => {
                let str_ptr = *(property.data as *const *const std::os::raw::c_char);
                Ok(MpvNode::String(cstr_to_string(str_ptr)))
            }
            libmpv_sys::mpv_format_MPV_FORMAT_FLAG => {
                Ok(MpvNode::Flag(*(property.data as *const i32) != 0))
            }
            libmpv_sys::mpv_format_MPV_FORMAT_INT64 => {
                Ok(MpvNode::Int64(*(property.data as *const i64)))
            }
            libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE => {
                Ok(MpvNode::Double(*(property.data as *const f64)))
            }
            libmpv_sys::mpv_format_MPV_FORMAT_NODE => {
                Self::from_node(property.data as *const libmpv_sys::mpv_node)
            }
            format => Err(Error::PropertyConversion(format!(
                "Unsupported mpv_event_property format: {}",
                format_to_string(format)
            ))),
        }
    }
}

macro_rules! get_property_impl {
    ($fn_name:ident, $ret_type:ty, $mpv_format:expr, $data_type:ty, $converter:expr) => {
        pub fn $fn_name(&self, name: &str) -> Result<$ret_type> {
            let c_name = CString::new(name)?;
            let mut data: $data_type = Default::default();

            let err = unsafe {
                libmpv_sys::mpv_get_property(
                    self.handle.inner(),
                    c_name.as_ptr(),
                    $mpv_format,
                    &mut data as *mut _ as *mut _,
                )
            };

            if err < 0 {
                return Err(Error::GetProperty {
                    name: name.to_string(),
                    message: error_string(err),
                });
            }

            Ok($converter(data))
        }
    };
}

macro_rules! get_property_ptr_impl {
    (
        $fn_name:ident,
        $ret_type:ty,
        $mpv_format:expr,
        $ptr_type:ty,
        $free_fn:path,
        $null_ret:expr,
        $converter:expr
    ) => {
        pub fn $fn_name(&self, name: &str) -> Result<$ret_type> {
            let c_name = CString::new(name)?;
            let mut data: $ptr_type = std::ptr::null_mut();

            let err = unsafe {
                libmpv_sys::mpv_get_property(
                    self.handle.inner(),
                    c_name.as_ptr(),
                    $mpv_format,
                    &mut data as *mut _ as *mut _,
                )
            };

            defer! {
                if !data.is_null() {
                    unsafe { $free_fn(data as *mut _) };
                }
            }

            if err < 0 {
                return Err(Error::GetProperty {
                    name: name.to_string(),
                    message: error_string(err),
                });
            }

            if data.is_null() {
                return Ok($null_ret);
            }

            unsafe { $converter(data) }
        }
    };
}

impl Mpv {
    get_property_ptr_impl!(
        get_property_string,
        String,
        libmpv_sys::mpv_format_MPV_FORMAT_STRING,
        *mut std::os::raw::c_char,
        libmpv_sys::mpv_free,
        String::new(),
        |data| Ok(std::ffi::CStr::from_ptr(data)
            .to_string_lossy()
            .into_owned())
    );

    get_property_impl!(
        get_property_flag,
        bool,
        libmpv_sys::mpv_format_MPV_FORMAT_FLAG,
        std::os::raw::c_int,
        |d| d != 0
    );

    get_property_impl!(
        get_property_int64,
        i64,
        libmpv_sys::mpv_format_MPV_FORMAT_INT64,
        i64,
        |d| d
    );

    get_property_impl!(
        get_property_double,
        f64,
        libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE,
        f64,
        |d| d
    );

    get_property_ptr_impl!(
        get_property_node,
        MpvNode,
        libmpv_sys::mpv_format_MPV_FORMAT_NODE,
        *mut libmpv_sys::mpv_node,
        libmpv_sys::mpv_free_node_contents,
        MpvNode::None,
        |data| MpvNode::from_node(data)
    );

    pub fn set_property(&self, name: &str, value: PropertyValue) -> Result<()> {
        let c_name = CString::new(name)?;

        let err = unsafe {
            match value {
                PropertyValue::String(s) => {
                    let c_value = CString::new(s)?;
                    libmpv_sys::mpv_set_property_string(
                        self.handle.inner(),
                        c_name.as_ptr(),
                        c_value.as_ptr(),
                    )
                }
                PropertyValue::Flag(b) => {
                    let mut val: std::os::raw::c_int = if b { 1 } else { 0 };
                    libmpv_sys::mpv_set_property(
                        self.handle.inner(),
                        c_name.as_ptr(),
                        libmpv_sys::mpv_format_MPV_FORMAT_FLAG,
                        &mut val as *mut _ as *mut _,
                    )
                }
                PropertyValue::Int64(mut i) => libmpv_sys::mpv_set_property(
                    self.handle.inner(),
                    c_name.as_ptr(),
                    libmpv_sys::mpv_format_MPV_FORMAT_INT64,
                    &mut i as *mut _ as *mut _,
                ),
                PropertyValue::Double(mut f) => libmpv_sys::mpv_set_property(
                    self.handle.inner(),
                    c_name.as_ptr(),
                    libmpv_sys::mpv_format_MPV_FORMAT_DOUBLE,
                    &mut f as *mut _ as *mut _,
                ),
                PropertyValue::Node(_) => {
                    return Err(Error::Unsupported(
                        "Setting a property with a Node value is not supported".to_string(),
                    ));
                }
            }
        };

        if err < 0 {
            return Err(Error::SetProperty {
                name: name.to_string(),
                message: error_string(err),
            });
        }

        Ok(())
    }
}
