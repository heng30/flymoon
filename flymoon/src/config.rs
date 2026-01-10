use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use platform_dirs::AppDirs;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Mutex};
use uuid::Uuid;

const CARGO_TOML: &str = include_str!("../Cargo.toml");
static CONFIG: Lazy<Mutex<Config>> = Lazy::new(|| Mutex::new(Config::default()));

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Config {
    #[serde(skip)]
    pub config_path: PathBuf,

    #[serde(skip)]
    pub db_path: PathBuf,

    #[serde(skip)]
    pub cache_dir: PathBuf,

    #[serde(skip)]
    pub is_first_run: bool,

    #[serde(skip)]
    pub app_name: String,

    #[serde(default = "appid_default")]
    pub appid: String,

    pub preference: Preference,

    pub model: Model,
}

#[derive(Serialize, Deserialize, Debug, Clone, Derivative)]
#[derivative(Default)]
pub struct Preference {
    #[derivative(Default(value = "600"))]
    pub win_width: u32,

    #[derivative(Default(value = "800"))]
    pub win_height: u32,

    #[derivative(Default(value = "16"))]
    pub font_size: u32,

    #[derivative(Default(value = "\"Source Han Sans CN\".to_string()"))]
    pub font_family: String,

    #[derivative(Default(value = "\"en\".to_string()"))]
    pub language: String,

    #[derivative(Default(value = "false"))]
    pub always_on_top: bool,

    #[derivative(Default(value = "true"))]
    pub no_frame: bool,

    pub is_dark: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ChatModel {
    pub api_base_url: String,
    pub model_name: String,
    pub reasoner_model_name: String,
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Model {
    pub chat: ChatModel,
}

impl Config {
    pub fn init(&mut self) -> Result<()> {
        let metadata = toml::from_str::<toml::Table>(CARGO_TOML).expect("Parse Cargo.toml error");

        self.app_name = metadata
            .get("package")
            .unwrap()
            .get("name")
            .unwrap()
            .to_string()
            .trim_matches('"')
            .to_string();

        let pkg_name = self.app_name.clone();

        let app_dirs = AppDirs::new(Some(&pkg_name), true).unwrap();
        self.init_config(&app_dirs)?;
        self.load().with_context(|| "load config file failed")?;
        log::info!("{:?}", self);
        Ok(())
    }

    fn init_config(&mut self, app_dirs: &AppDirs) -> Result<()> {
        self.db_path = app_dirs.data_dir.join(format!("{}.db", self.app_name));
        self.config_path = app_dirs.config_dir.join(format!("{}.toml", self.app_name));
        self.cache_dir = app_dirs.data_dir.join("cache");

        if self.appid.is_empty() {
            self.appid = appid_default();
        }

        fs::create_dir_all(&app_dirs.data_dir)?;
        fs::create_dir_all(&app_dirs.config_dir)?;
        fs::create_dir_all(&self.cache_dir)?;

        Ok(())
    }

    fn load(&mut self) -> Result<()> {
        match fs::read_to_string(&self.config_path) {
            Ok(text) => match toml::from_str::<Config>(&text) {
                Ok(c) => {
                    self.appid = c.appid;
                    self.preference = c.preference;
                    self.model = c.model;
                    Ok(())
                }
                Err(_) => {
                    self.is_first_run = true;

                    if let Some(bak_file) = &self.config_path.as_os_str().to_str() {
                        let bak_file = format!("{}.bak", bak_file);
                        _ = fs::copy(&self.config_path, &bak_file);
                    }

                    match toml::to_string_pretty(self) {
                        Ok(text) => Ok(fs::write(&self.config_path, text)?),
                        Err(e) => Err(e.into()),
                    }
                }
            },
            Err(_) => {
                self.is_first_run = true;

                if let Some(bak_file) = &self.config_path.as_os_str().to_str() {
                    let bak_file = format!("{}.bak", bak_file);
                    _ = fs::copy(&self.config_path, &bak_file);
                }

                match toml::to_string_pretty(self) {
                    Ok(text) => Ok(fs::write(&self.config_path, text)?),
                    Err(e) => Err(e.into()),
                }
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        match toml::to_string_pretty(self) {
            Ok(text) => Ok(fs::write(&self.config_path, text)
                .with_context(|| "save config failed".to_string())?),
            Err(e) => anyhow::bail!(format!("convert config from toml format failed. {e:?}")),
        }
    }
}

pub fn init() {
    if let Err(e) = CONFIG.lock().unwrap().init() {
        log::error!("{e:?}");
        panic!("{:?}", e);
    }
}

#[allow(dead_code)]
pub fn appid() -> String {
    CONFIG.lock().unwrap().appid.clone()
}

#[allow(dead_code)]
pub fn app_name() -> String {
    CONFIG.lock().unwrap().app_name.clone()
}

pub fn is_first_run() -> bool {
    CONFIG.lock().unwrap().is_first_run
}

pub fn all() -> Config {
    CONFIG.lock().unwrap().clone()
}

#[allow(dead_code)]
pub fn reset(mut conf: Config) {
    let mut c = CONFIG.lock().unwrap();

    conf.config_path.clone_from(&c.config_path);
    conf.db_path.clone_from(&c.db_path);
    conf.cache_dir.clone_from(&c.cache_dir);
    conf.is_first_run = c.is_first_run;

    *c = conf;
    _ = c.save();
}

pub fn preference() -> Preference {
    CONFIG.lock().unwrap().preference.clone()
}

pub fn model() -> Model {
    CONFIG.lock().unwrap().model.clone()
}

#[cfg(feature = "database")]
#[allow(dead_code)]
pub fn db_path() -> PathBuf {
    CONFIG.lock().unwrap().db_path.clone()
}

#[allow(dead_code)]
pub fn cache_dir() -> PathBuf {
    CONFIG.lock().unwrap().cache_dir.clone()
}

pub fn save(conf: Config) -> Result<()> {
    let mut config = CONFIG.lock().unwrap();
    *config = conf;
    config.save()
}

fn appid_default() -> String {
    Uuid::new_v4().to_string()
}
