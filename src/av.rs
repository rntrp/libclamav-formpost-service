use chrono::{DateTime, Utc};
use std::{
    error,
    ffi::CStr,
    fmt, str,
    sync::Once,
    time::{Duration, UNIX_EPOCH},
};

use crate::{
    av_bindings::{self, cl_error_t, cl_strerror},
    av_engine::Engine,
    av_settings::AvSettings,
};

fn init() -> Result<(), AvError> {
    static O: Once = Once::new();
    static mut VAL: cl_error_t = av_bindings::cl_error_t_CL_SUCCESS;
    unsafe {
        O.call_once(|| VAL = av_bindings::cl_init(av_bindings::CL_INIT_DEFAULT));
        match VAL {
            av_bindings::cl_error_t_CL_SUCCESS => Ok(()),
            _ => Err(AvError::new(VAL)),
        }
    }
}

fn version() -> Option<String> {
    match unsafe { av_bindings::cl_retver() } {
        v if v.is_null() => None,
        v => Some(unsafe { std::ffi::CStr::from_ptr(v).to_string_lossy().to_string() }),
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct AvError {
    code: u32,
}

impl AvError {
    pub fn new(native_err: cl_error_t) -> Self {
        AvError {
            code: native_err as u32,
        }
    }

    pub fn string_error(&self) -> String {
        unsafe {
            let ptr = cl_strerror(self.code);
            let bytes = CStr::from_ptr(ptr).to_bytes();
            str::from_utf8(bytes).ok().unwrap().to_string()
        }
    }

    pub fn code(&self) -> u32 {
        self.code
    }
}

impl fmt::Display for AvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "cl_error {}: {}", self.code(), self.string_error())
    }
}

impl fmt::Debug for AvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl error::Error for AvError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

pub struct AvContext {
    pub clamav_version: String,
    pub db_version: u32,
    pub db_sig_count: u32,
    pub db_date: DateTime<Utc>,
    pub engine: Engine,
    pub settings: AvSettings,
}

impl fmt::Display for AvContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            concat!(
                "\tlibclamav version: {}\n",
                "\tDB version: {}\n",
                "\tDB signature count: {}\n",
                "\tDB date: {}",
            ),
            self.clamav_version, self.db_version, self.db_sig_count, self.db_date,
        )
    }
}

pub fn load_context() -> AvContext {
    init().unwrap();
    let engine = Engine::new();
    let stats = engine.load_db("/var/lib/clamav").unwrap();
    engine.compile().unwrap();
    AvContext {
        clamav_version: version().unwrap(),
        db_version: engine.db_version().unwrap(),
        db_sig_count: stats.signo,
        db_date: DateTime::<Utc>::from(
            UNIX_EPOCH + Duration::from_secs(engine.db_timestamp().unwrap() as u64),
        ),
        engine,
        settings: AvSettings::default(),
    }
}
