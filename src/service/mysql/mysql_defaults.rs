use std::error::Error;
use std::path::Path;
use ini::Ini;
use log::info;
use sqlx::mysql::MySqlConnectOptions;

pub trait MySqlDefaultsReader {
    fn from_defaults_file(defaults_file: &Path) -> Result<MySqlConnectOptions, Box<dyn Error>>;
}

impl MySqlDefaultsReader for MySqlConnectOptions {
    fn from_defaults_file(defaults_file: &Path) -> Result<MySqlConnectOptions, Box<dyn Error>> {
        let mut conf = Ini::load_from_file(defaults_file)?;
        let mut options = MySqlConnectOptions::new();
        if let Some(section) = conf.section(Some("client")) {
            if let Some(host) = section.get("host") {
                options = options.host(host);
            }
            if let Some(port) = section.get("port") {
                let port: u16 = port.parse()?;
                options = options.port(port);
            }
            if let Some(user) = section.get("user") {
                info!("reading username: {}", user);
                options = options.username(user);
            }
            if let Some(password) = section.get("password") {
                options = options.password(password);
            }
            if let Some(socket) = section.get("socket") {
                options = options.socket(socket);
            }
        }
        Ok(options)
    }
}