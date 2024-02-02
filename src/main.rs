use std::collections::HashMap;
use std::fs;
use crate::config::*;

mod config;
mod database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config {
        services: HashMap::from([
             ("mysql-r1".to_string(), ServiceConfigEnum::MySQL(MySQLConnectionConfig {
                 host: Some("127.0.0.1".to_string()),
                 port: Some(3306),
                 username: Some("root".to_string()),
                 password: Some("123456".to_string()),
                defaults_file: None,
            })),
            ("mysql-r2".to_string(), ServiceConfigEnum::MySQL(MySQLConnectionConfig {
                host: None,
                port: None,
                username: None,
                password: None,
                defaults_file: Some("/etc/mysql/default2.conf".to_string()),
            })),
            ("mysql-r3".to_string(), ServiceConfigEnum::MySQL(MySQLConnectionConfig {
                host: None,
                port: None,
                username: None,
                password: None,
                defaults_file: Some("/etc/mysql/default3.conf".to_string()),
            })),
        ]),
        backup: BackupConfig {
            basedir: "/opt/backup".to_string(),
            services: HashMap::from([
                ("mysql-r1".to_string(), BackupConfigEnum::MySQL(MySQLBackupConfig {
                    encrypt: Some(XtrabackupEncryptConfig {
                        enabled: true,
                        key_file: "/etc/mysql/backup.key".to_string(),
                        threads: Some(16),
                    }),
                    databases: Some(vec!["auth".to_string(), "wordpress".to_string()]),
                    databases_exclude: None,
                    incremental: Some(true),
                    incremental_basedir: Some("/etc/mysql/incremental".to_string()),
                    parallel_threads: None,
                    rsync: None,
                }))
            ]),
        }
    };

    let result = toml::to_string(&config).unwrap();
    fs::write("output.toml", result).unwrap();
    
    //let config = Config::new("config.toml");
    println!("ok");

    Ok(())
}
