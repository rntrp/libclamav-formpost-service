use std::ffi::CStr;
use std::ffi::CString;
use std::mem;
use std::os::raw::c_int;
use std::ptr;
use std::str;

use crate::av::AvError;
use crate::av_bindings::{
    self, cl_engine_field, cl_engine_get_num, cl_engine_get_str, cl_error_t, cl_load, CL_DB_STDOPT,
};
use crate::av_settings::AvSettings;

pub struct DBStats {
    pub signo: u32,
}

pub enum AvScanResult {
    Clean,
    Whitelisted,
    Virus(String),
}

#[derive(Debug, PartialEq)]
pub enum ValType {
    U32,
    U64,
    STR,
}

pub enum Val {
    U32(u32),
    U64(u64),
    STR(String),
}

pub struct Engine {
    handle: *mut av_bindings::cl_engine,
}

unsafe impl Send for Engine {}
unsafe impl Sync for Engine {}

impl Engine {
    pub fn new() -> Self {
        unsafe {
            let handle = av_bindings::cl_engine_new();
            Engine { handle }
        }
    }

    pub fn compile(&self) -> Result<(), AvError> {
        unsafe {
            let result = av_bindings::cl_engine_compile(self.handle);
            match result {
                av_bindings::cl_error_t_CL_SUCCESS => Ok(()),
                _ => Err(AvError::new(result)),
            }
        }
    }

    pub fn load_db(&self, db_dir_path: &str) -> Result<DBStats, AvError> {
        let raw_path = CString::new(db_dir_path).unwrap();
        unsafe {
            let mut signo: u32 = 0;
            let result = cl_load(raw_path.as_ptr(), self.handle, &mut signo, CL_DB_STDOPT);
            match result {
                av_bindings::cl_error_t_CL_SUCCESS => Ok(DBStats { signo }),
                _ => Err(AvError::new(result)),
            }
        }
    }

    pub fn scan(&self, path: &str, settings: &mut AvSettings) -> Result<AvScanResult, AvError> {
        let raw_path = CString::new(path).unwrap();
        unsafe {
            let mut virname: *const i8 = ptr::null();
            let result = av_bindings::cl_scanfile(
                raw_path.as_ptr(),
                &mut virname,
                ptr::null_mut(),
                self.handle,
                &mut settings.settings,
            );
            map_scan_result(result, virname)
        }
    }

    fn get(&self, field: cl_engine_field) -> Result<Val, AvError> {
        unsafe {
            match field_type(field) {
                ValType::U32 => {
                    let mut err: c_int = 0;
                    let val = cl_engine_get_num(self.handle, field, &mut err) as u32;
                    match err {
                        0 => Ok(Val::U32(val)),
                        _ => Err(AvError::new(mem::transmute(err))),
                    }
                }
                ValType::U64 => {
                    let mut err: c_int = 0;
                    let val = cl_engine_get_num(self.handle, field, &mut err) as u64;
                    match err {
                        0 => Ok(Val::U64(val)),
                        _ => Err(AvError::new(mem::transmute(err))),
                    }
                }
                ValType::STR => {
                    let mut err = 0;
                    let val = cl_engine_get_str(self.handle, field, &mut err);
                    match err {
                        0 => Ok(Val::STR(CStr::from_ptr(val).to_str().unwrap().to_string())),
                        _ => Err(AvError::new(mem::transmute(err))),
                    }
                }
            }
        }
    }

    pub fn db_version(&self) -> Result<u32, AvError> {
        match self.get(av_bindings::cl_engine_field_CL_ENGINE_DB_VERSION)? {
            Val::U32(val) => Ok(val),
            _ => Err(AvError::new(av_bindings::cl_error_t_CL_EARG)),
        }
    }

    pub fn db_timestamp(&self) -> Result<u32, AvError> {
        match self.get(av_bindings::cl_engine_field_CL_ENGINE_DB_TIME)? {
            Val::U32(val) => Ok(val),
            _ => Err(AvError::new(av_bindings::cl_error_t_CL_EARG)),
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            av_bindings::cl_engine_free(self.handle);
        }
    }
}

fn map_scan_result(result: cl_error_t, virname: *const i8) -> Result<AvScanResult, AvError> {
    match result {
        av_bindings::cl_error_t_CL_CLEAN => Ok(AvScanResult::Clean),
        av_bindings::cl_error_t_CL_BREAK => Ok(AvScanResult::Whitelisted),
        av_bindings::cl_error_t_CL_VIRUS => unsafe {
            let bytes = CStr::from_ptr(virname).to_bytes();
            let name = str::from_utf8(bytes).ok().unwrap_or_default().to_string();
            Ok(AvScanResult::Virus(name))
        },
        _ => Err(AvError::new(result)),
    }
}

fn field_type(field: cl_engine_field) -> ValType {
    match field {
        av_bindings::cl_engine_field_CL_ENGINE_MAX_SCANSIZE => ValType::U64,
        av_bindings::cl_engine_field_CL_ENGINE_MAX_FILESIZE => ValType::U64,
        av_bindings::cl_engine_field_CL_ENGINE_MAX_RECURSION => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_MAX_FILES => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_MIN_CC_COUNT => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_MIN_SSN_COUNT => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_PUA_CATEGORIES => ValType::STR,
        av_bindings::cl_engine_field_CL_ENGINE_DB_OPTIONS => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_DB_VERSION => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_DB_TIME => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_AC_ONLY => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_AC_MINDEPTH => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_AC_MAXDEPTH => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_TMPDIR => ValType::STR,
        av_bindings::cl_engine_field_CL_ENGINE_KEEPTMP => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_BYTECODE_SECURITY => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_BYTECODE_TIMEOUT => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_BYTECODE_MODE => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_MAX_EMBEDDEDPE => ValType::U64,
        av_bindings::cl_engine_field_CL_ENGINE_MAX_HTMLNORMALIZE => ValType::U64,
        av_bindings::cl_engine_field_CL_ENGINE_MAX_HTMLNOTAGS => ValType::U64,
        av_bindings::cl_engine_field_CL_ENGINE_MAX_SCRIPTNORMALIZE => ValType::U64,
        av_bindings::cl_engine_field_CL_ENGINE_MAX_ZIPTYPERCG => ValType::U64,
        av_bindings::cl_engine_field_CL_ENGINE_FORCETODISK => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_DISABLE_CACHE => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_DISABLE_PE_STATS => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_STATS_TIMEOUT => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_MAX_PARTITIONS => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_MAX_ICONSPE => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_MAX_RECHWP3 => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_MAX_SCANTIME => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_PCRE_MATCH_LIMIT => ValType::U64,
        av_bindings::cl_engine_field_CL_ENGINE_PCRE_RECMATCH_LIMIT => ValType::U64,
        av_bindings::cl_engine_field_CL_ENGINE_PCRE_MAX_FILESIZE => ValType::U64,
        av_bindings::cl_engine_field_CL_ENGINE_DISABLE_PE_CERTS => ValType::U32,
        av_bindings::cl_engine_field_CL_ENGINE_PE_DUMPCERTS => ValType::U32,
        _ => panic!("Unknown field"),
    }
}
