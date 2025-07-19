#[macro_use]
extern crate rocket;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use rocket::response::Redirect;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::fs;
use std::io::{Write, stdin, stdout};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    content: Value,
    file_path: String,
}

impl AppConfig {
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        Ok(AppConfig {
            content: serde_json::from_str(&content)?,
            file_path: path.to_string(),
        })
    }

    pub fn reload(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Reloading configuration from {}", self.file_path);
        let content = fs::read_to_string(&self.file_path)?;
        self.content = serde_json::from_str(&content)?;
        println!("Configuration reloaded successfully: {:?}", self.content);
        Ok(())
    }

    pub fn get_content(&self) -> &Value {
        &self.content
    }
}

async fn watch_config_file() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel(100);

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| match res {
            Ok(event) => {
                if let Err(e) = tx.blocking_send(event) {
                    eprintln!("Failed to send file event: {}", e);
                }
            }
            Err(e) => eprintln!("Watch error: {:?}", e),
        },
        Config::default().with_poll_interval(Duration::from_secs(60)),
    )?;

    watcher.watch(Path::new(FILE_NAME), RecursiveMode::NonRecursive)?;

    println!("Started watching {} for changes...", FILE_NAME);

    while let Some(event) = rx.recv().await {
        match event.kind {
            notify::EventKind::Modify(_) => {
                println!("Config file modified, reloading...");
                if let Ok(mut config) = CONFIG.write() {
                    if let Err(e) = config.reload() {
                        eprintln!("Failed to reload config: {}", e);
                    }
                } else {
                    eprintln!("Failed to acquire write lock on config");
                }
            }
            _ => {}
        }
    }

    Ok(())
}

#[get("/<app>")]
fn index(app: &str) -> Redirect {
    let redirect_url = {
        let config_guard = CONFIG.read().unwrap();
        println!("Current config: {:?}", config_guard.content);

        config_guard
            .content
            .get(app)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };

    match redirect_url {
        Some(url) => {
            println!("Redirecting to: {}", url);
            Redirect::to(url)
        }
        None => Redirect::to("/"),
    }
}

#[get("/")]
fn home() -> Redirect {
    Redirect::to(DEFAULT_URL.as_str())
}

pub static DEFAULT_URL: Lazy<String> = Lazy::new(|| {
    let config = CONFIG.read().unwrap();
    config
        .get_content()
        .get("default")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "https://example.com".to_string())
});

pub static FILE_NAME: &str = "config.json";
pub static CONFIG: Lazy<Arc<RwLock<AppConfig>>> = Lazy::new(|| {
    let config = AppConfig::new(FILE_NAME).expect("Failed to load configuration file");
    Arc::new(RwLock::new(config))
});

fn reload() {
    println!("Reloading configuration...");
    match CONFIG.write() {
        Ok(mut config) => {
            if let Err(e) = config.reload() {
                eprintln!("Failed to reload config: {}", e);
            }
        }
        Err(e) => eprintln!("Failed to acquire write lock on config: {}", e),
    }
}

fn get_user_input() -> Result<(String, String), ()> {
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

fn save_config(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let content = serde_json::to_string(&config.content)?;
    fs::write(&config.file_path, content)?;
    println!("Configuration saved to {}", config.file_path);
    Ok(())
}

fn add_url(app_name: String, app_url: String) {
    
    let app_url = app_url.trim();
    let mut config = CONFIG.write().unwrap();
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

fn print_help() {
    println!("Available commands:");
    println!("h - Show this help message");
    println!("a - Add a new app URL");
    println!("r - Reload the configuration file");
    println!("q - Quit the application");
}

#[rocket::main]
async fn main() {
    println!("Loading configuration...");
    {
        let config = CONFIG.read().unwrap();
        println!("Initial config loaded: {:?}", config.content);
    }

    tokio::spawn(async {
        if let Err(e) = watch_config_file().await {
            eprintln!("File watcher error: {}", e);
        }
    });

    tokio::spawn(async {
        rocket::build()
            .mount("/", routes![home, index])
            .configure(rocket::Config {
                port: 80,
                ..rocket::Config::default()
            })
            .launch()
            .await
            .expect("Rocket failed to launch");
    });

    loop {
        let mut action = String::new();
        println!("Press h for help");
        let _ = stdout().flush();
        match stdin().read_line(&mut action) {
            Ok(_) => {
                let trimmed = action.trim();
                match trimmed {
                    "h" => print_help(),
                    "a" => {
                        let (app_name, app_url) = match get_user_input() {
                            Ok((name, url)) => (name, url),
                            Err(_) => {
                                eprintln!("Failed to add app. Please try again.");
                                continue;
                            },
                        };
                        add_url(app_name, app_url);
                        reload();
                    }
                    "r" => {
                        reload();
                    }
                    "q" => {
                        println!("Exiting...");
                        break;
                    }
                    _ => {println!("Unknown command: {}", trimmed); print_help();},
                }
            }
            Err(e) => eprintln!("Failed to read line: {}", e),
        }
    }
}
