use std::collections::HashMap;
use std::fs;
use crate::config::*;

mod config;
mod database;
mod service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::new("config.toml").await.unwrap();
    println!("config: {:?}", config);

    Ok(())
}
