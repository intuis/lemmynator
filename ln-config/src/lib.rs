use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    sync::OnceLock,
};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use toml::Table;
use xdg::BaseDirectories;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub connection: Connection,
}

#[derive(Serialize, Deserialize)]
pub struct Connection {
    pub instance: String,
    pub username: String,
    // TODO: make it more secure
    pub password: String,
}

const DEFAULT_CONFIG: &str = include_str!("../defaults/config.toml");
static XDG_DIRS: OnceLock<BaseDirectories> = OnceLock::new();
static CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

impl Config {
    pub fn init() -> Result<Self> {
        let table = match Self::table_from_home() {
            Ok(table) => table,
            Err(_) => Self::put_default_conf_in_home()?,
        };

        Self::table_config_verify(&table)?;

        Self::table_to_config(table)
    }

    fn table_to_config(table: Table) -> Result<Self> {
        let config_string = table.to_string();
        let config: Config = toml::from_str(&config_string)?;
        Ok(config)
    }

    fn table_config_verify(table: &Table) -> Result<()> {
        let Some(connection_table) = table.get("connection").unwrap().as_table() else {
            bail!("expected connection table")
        };

        Ok(())
    }

    fn table_from_home() -> Result<Table> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix("lemmynator")?;
        let config_path = xdg_dirs
            .find_config_file("config.toml")
            .ok_or_else(|| anyhow::anyhow!("config.toml not found"))?;

        let mut config_buf = String::new();
        let mut config_file = File::open(config_path).unwrap();
        config_file.read_to_string(&mut config_buf).unwrap();
        Ok(toml::from_str(&config_buf)?)
    }

    fn put_default_conf_in_home() -> Result<Table> {
        let config_path = Self::get_config_path();
        let mut config_file = File::create(config_path)?;
        config_file.write_all(DEFAULT_CONFIG.as_bytes())?;
        Ok(toml::from_str(DEFAULT_CONFIG)?)
    }

    fn get_xdg_dirs() -> &'static BaseDirectories {
        XDG_DIRS.get_or_init(|| xdg::BaseDirectories::with_prefix("lemmynator").unwrap())
    }

    fn get_config_path() -> &'static PathBuf {
        CONFIG_PATH.get_or_init(|| {
            Self::get_xdg_dirs()
                .place_config_file("config.toml")
                .unwrap()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invalid_config() -> Table {
        toml::toml! {
            [connection]
            instance = "bad_url"
        }
    }

    fn valid_config() -> Table {
        toml::toml! {
            [connection]
            instance = "lemmy.ml"
        }
    }

    #[test]
    fn validates_properly() {
        let valid_config = valid_config();
        assert!(Config::table_config_verify(&valid_config).is_ok());
    }

    #[test]
    fn invalidates_properly() {
        let invalid_config = invalid_config();
        assert!(Config::table_config_verify(&invalid_config).is_err());
    }
}
