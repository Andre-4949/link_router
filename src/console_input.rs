use std::{fs, io::{stdin, stdout, Write}, sync::{Arc, RwLock}};

use serde_json::Value;

use crate::app_config::AppConfig;

pub fn reload(config: Arc<RwLock<AppConfig>>) {
    println!("Reloading configuration...");
    match config.write() {
        Ok(mut config) => {
            if let Err(e) = config.reload() {
                eprintln!("Failed to reload config: {}", e);
            }
        }
        Err(e) => eprintln!("Failed to acquire write lock on config: {}", e),
    }
}

pub fn get_user_input() -> Result<(String, String), ()> {
    println!("Adding a new app...");
    let mut app_name = String::new();
    println!("Enter app name:");
    let _ = stdout().flush();
    stdin()
        .read_line(&mut app_name)
        .expect("Failed to read app name");
    let app_name = app_name.trim();
    let mut app_url = String::new();
    println!("Enter app URL:");
    let _ = stdout().flush();
    stdin()
        .read_line(&mut app_url)
        .expect("Failed to read app URL");
    if app_name.is_empty() || app_url.is_empty() {
        eprintln!("App name and URL cannot be empty.");
        return Err(());
    }
    if !app_url.starts_with("http://") && !app_url.starts_with("https://") {
        eprintln!("App URL must start with 'http://' or 'https://'.");
        return Err(());
    }
    return Ok((app_name.to_string(), app_url.to_string()));
}

pub fn save_config(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let content = serde_json::to_string_pretty(&config.content)?;
    fs::write(&config.file_path, content)?;
    println!("Configuration saved to {}", config.file_path);
    Ok(())
}

pub fn add_url(app_name: String, app_url: String, config: Arc<RwLock<AppConfig>>) {
    let app_url = app_url.trim();
    let mut config = config.write().unwrap();
    let mut content = config.get_content().clone();
    content[app_name.clone()] = Value::String(app_url.to_string());
    config.content = content;
    println!("App '{}' added with URL '{}'", app_name, app_url);
    println!("New configuration: {:?}", config.content);
    if let Err(e) = save_config(&config) {
        eprintln!("Failed to save configuration: {}", e);
    } else {
        println!("Configuration saved successfully.");
    }
}

pub fn print_help() {
    println!("Available commands:");
    println!("h - Show this help message");
    println!("a - Add a new app URL");
    println!("r - Reload the configuration file");
    println!("q - Quit the application");
}