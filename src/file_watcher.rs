use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::app_config::AppConfig;

pub async fn watch_config_file(
    file_name: &str,
    config: Arc<RwLock<AppConfig>>,
) -> Result<(), Box<dyn std::error::Error>> {
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

    watcher.watch(Path::new(file_name), RecursiveMode::NonRecursive)?;

    println!("Started watching {} for changes...", file_name);

    while let Some(event) = rx.recv().await {
        match event.kind {
            notify::EventKind::Modify(_) => {
                println!("Config file modified, reloading...");
                if let Ok(mut config) = config.write() {
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