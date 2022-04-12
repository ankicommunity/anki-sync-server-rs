use crate::error::ApplicationError;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    listen: ConfigAddr,
    paths: ConfigPaths,
    encryption: Option<ConfigCert>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            listen: ConfigAddr::default(),
            paths: ConfigPaths::default(),
            encryption: Some(ConfigCert::default()),
        }
    }
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ApplicationError> {
        let mut file = File::open(path)?;
        let mut config_string = String::new();
        file.read_to_string(&mut config_string)?;
        let c = toml::from_str(&config_string)?;
        Ok(c)
    }

    pub fn to_string(&self) -> Result<String, ApplicationError> {
        let s = toml::to_string(&self)?;
        Ok(s)
    }

    pub fn encryption_enabled(&self) -> bool {
        match &self.encryption {
            Some(e) => e.ssl_enable,
            None => false,
        }
    }

    pub fn listen_on(&self) -> String {
        format!("{}:{}", &self.listen.host, self.listen.port)
    }

    pub fn data_root_path(&self) -> String {
        format!("{}/collections/", self.paths.root_dir)
    }

    pub fn auth_db_path(&self) -> String {
        format!("{}/auth.db", self.paths.root_dir)
    }

    pub fn session_db_path(&self) -> String {
        format!("{}/session.db", self.paths.root_dir)
    }

    pub fn encryption_config(&self) -> Option<&ConfigCert> {
        self.encryption.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigAddr {
    pub host: String,
    pub port: u16,
}

impl Default for ConfigAddr {
    fn default() -> Self {
        ConfigAddr {
            host: "0.0.0.0".to_string(),
            port: 27701,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPaths {
    root_dir: String,
}

impl Default for ConfigPaths {
    fn default() -> Self {
        ConfigPaths {
            root_dir: ".".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigCert {
    ssl_enable: bool,
    pub cert_file: String,
    pub key_file: String,
}

impl Default for ConfigCert {
    fn default() -> Self {
        ConfigCert {
            ssl_enable: false,
            cert_file: "".to_string(),
            key_file: "".to_string(),
        }
    }
}
