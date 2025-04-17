use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

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

    #[derivative(Default(value = "\"cn\".to_string()"))]
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

#[derive(Serialize, Deserialize, Debug, Clone, Derivative)]
#[derivative(Default)]
pub struct GoogleSearch {
    pub cx: String,
    pub api_key: String,

    #[derivative(Default(value = "5"))]
    pub num: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Model {
    pub chat: ChatModel,
    pub google_search: GoogleSearch,
}

pub fn appid_default() -> String {
    Uuid::new_v4().to_string()
}
