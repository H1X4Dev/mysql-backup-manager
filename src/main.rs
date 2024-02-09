use std::collections::HashMap;
use std::fs;
use crate::config::*;

mod config;
mod database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config {
        backup: BackupConfig {
            basedir:  "".to_string()
        },
        services: HashMap::from([
             ("mysql-r1".to_string(), ServiceConfigEnum::MySQL(MySQLConnectionConfig {
                host: Some("127.0.0.1".to_string()),
                port: Some(3306),
                username: Some("root".to_string()),
                password: Some("123456".to_string()),
                defaults_file: None,
                backup: Some(MySQLBackupConfig {
                    backup_type: MySQLBackupType::xtrabackup(XtraBackupConfig {
                        encrypt: Some(XtrabackupEncryptConfig {
                            key_file: "/etc/mysql/backup.key".to_string(),
                            threads: Some(16),
                        }),
                        incremental: Some(XtraBackupIncrementalConfig {
                            enabled: true,
                            basedir: "/home".to_string()
                        }),
                        parallel_threads: Some(16),
                    }),
                    databases: Some(vec!["auth".to_string(), "wordpress".to_string()]),
                    databases_exclude: None,
                    timer: TimerConfig {
                        interval: "* * * * *".to_string(),
                        keep_last: 7,
                    }
                }),
            }))
        ])
    };

    let result = toml::to_string(&config).unwrap();
    fs::write("output.toml", result).unwrap();
    
    //let config = Config::new("config.toml");
    println!("ok");

    Ok(())
}
