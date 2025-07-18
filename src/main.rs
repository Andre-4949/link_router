#[macro_use]
extern crate rocket;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use rocket::response::Redirect;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::fs;
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
        move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if let Err(e) = tx.blocking_send(event) {
                        eprintln!("Failed to send file event: {}", e);
                    }
                }
                Err(e) => eprintln!("Watch error: {:?}", e),
            }
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
        
        config_guard.content.get(app)
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
    Redirect::to(DEFAULT_URL)
}

pub static DEFAULT_URL: &str = "https://dhbw-engineering.de";

pub static FILE_NAME: &str = "config.json";


pub static CONFIG: Lazy<Arc<RwLock<AppConfig>>> = Lazy::new(|| {
    let config = AppConfig::new(FILE_NAME).expect("Failed to load configuration file");
    Arc::new(RwLock::new(config))
});


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

    rocket::build()
        .mount("/", routes![home, index])
        .configure(rocket::Config {
            port: 80,
            ..rocket::Config::default()
        })
        .launch()
        .await
        .unwrap();
}
