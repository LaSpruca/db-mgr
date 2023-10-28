use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Display,
    fs::{self, File},
};

const DEFAULT_CONFIG: &str = include_str!("../config.yaml");

#[derive(Serialize, Deserialize)]
pub struct ConfigFile {
    pub databases: Vec<DatabaseConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DatabaseConfig {
    pub name: String,
    pub image: String,
    pub icon_url: String,
    pub tags: Vec<String>,
    pub variables: HashMap<String, String>,
    pub volumes: HashMap<String, String>,
}

impl Display for DatabaseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub fn get_file() -> Option<File> {
    let project_dirs = directories::ProjectDirs::from("nz", "laspruca", "db-mgr")?;
    let config_path = project_dirs.config_dir();

    if !config_path.exists() {
        if let Err(ex) = fs::create_dir_all(config_path) {
            eprintln!("Could not create config directory {ex}");
            return None;
        }
    }

    let config_file = config_path.join("config.yaml");

    if !config_file.exists() {
        if let Err(ex) = fs::write(&config_file, DEFAULT_CONFIG) {
            eprintln!("Could not create and write config file {ex}");
            return None;
        }
    }

    match File::open(config_file) {
        Err(ex) => {
            eprintln!("Could not open config file {ex}");
            None
        }
        Ok(s) => Some(s),
    }
}

pub fn read_config_file() -> ConfigFile {
    match get_file().map(serde_yaml::from_reader) {
        None => serde_yaml::from_str(DEFAULT_CONFIG).unwrap(),
        Some(Err(ex)) => {
            eprintln!("Could not open read file {ex}");
            serde_yaml::from_str(DEFAULT_CONFIG).unwrap()
        }
        Some(Ok(file)) => file,
    }
}
