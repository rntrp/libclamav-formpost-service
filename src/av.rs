use chrono::{DateTime, Utc};
use std::fmt;

pub struct AvContext {
    pub clamav_version: String,
    pub db_version: u32,
    pub db_sig_count: u32,
    pub db_date: DateTime<Utc>,
    pub engine: clamav_async::engine::Engine,
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

pub async fn load_context() -> AvContext {
    clamav_async::initialize().unwrap();
    let engine = clamav_async::engine::Engine::new();
    let stats = engine.load_databases("/var/lib/clamav").await.unwrap();
    engine.compile().await.unwrap();
    AvContext {
        clamav_version: clamav_async::version(),
        db_version: engine.database_version().await.unwrap(),
        db_sig_count: stats.signature_count,
        db_date: DateTime::<Utc>::from(engine.database_timestamp().await.unwrap()),
        engine,
    }
}
