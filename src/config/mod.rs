mod conf;
pub mod data;
pub use conf::{all, cache_dir, app_name, init, is_first_run, model, preference, save};

#[cfg(feature = "database")]
pub use conf::db_path;
