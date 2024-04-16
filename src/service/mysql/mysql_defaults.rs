use std::error::Error;
use std::path::Path;
use ini::Ini;
use sqlx::mysql::MySqlConnectOptions;

pub trait MySqlDefaultsReader {
    fn from_defaults_file(defaults_file: &Path) -> Result<MySqlConnectOptions, Box<dyn Error>>;
}

impl MySqlDefaultsReader for MySqlConnectOptions {
    fn from_defaults_file(defaults_file: &Path) -> Result<MySqlConnectOptions, Box<dyn Error>> {
        let conf = Ini::load_from_file(defaults_file)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_from_defaults_file() {
        // Create a temporary file with sample MySQL defaults
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[client]").unwrap();
        writeln!(file, "host = localhost").unwrap();
        writeln!(file, "port = 3306").unwrap();
        writeln!(file, "user = testuser").unwrap();
        writeln!(file, "password = testpass").unwrap();
        writeln!(file, "socket = /tmp/mysql.sock").unwrap();

        // Call the from_defaults_file function
        let options = MySqlConnectOptions::from_defaults_file(file.path()).unwrap();

        // Assert the expected values
        assert_eq!(options.host(), Some("localhost"));
        assert_eq!(options.port(), Some(3306));
        assert_eq!(options.username(), Some("testuser"));
        assert_eq!(options.password(), Some("testpass"));
        assert_eq!(options.socket(), Some("/tmp/mysql.sock"));
    }

    #[test]
    fn test_from_defaults_file_missing_fields() {
        // Create a temporary file with missing fields
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[client]").unwrap();
        writeln!(file, "host = localhost").unwrap();

        // Call the from_defaults_file function
        let options = MySqlConnectOptions::from_defaults_file(file.path()).unwrap();

        // Assert the expected values
        assert_eq!(options.host(), Some("localhost"));
        assert_eq!(options.port(), None);
        assert_eq!(options.username(), None);
        assert_eq!(options.password(), None);
        assert_eq!(options.socket(), None);
    }

    #[test]
    fn test_from_defaults_file_invalid_port() {
        // Create a temporary file with an invalid port
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[client]").unwrap();
        writeln!(file, "port = invalid").unwrap();

        // Call the from_defaults_file function and assert the error
        let result = MySqlConnectOptions::from_defaults_file(file.path());
        assert!(result.is_err());
    }
}