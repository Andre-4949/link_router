#[macro_use]
extern crate rocket;

mod app_config;
mod console_input;
mod file_watcher;

use app_config::AppConfig;
use console_input::*;
use file_watcher::*;

use rocket::State;
use rocket::response::Redirect;
use std::io::{Write, stdin, stdout};
use std::sync::{Arc, RwLock};

#[get("/<app>")]
fn index(app: &str, config: &State<Arc<RwLock<AppConfig>>>) -> Redirect {
    let redirect_url = {
        let config_guard = match config.read() {
            Ok(guard) => guard,
            Err(_) => return Redirect::to("/"),
        };

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
fn home(config: &State<Arc<RwLock<AppConfig>>>) -> Redirect {
    let default_url = {
        let config_guard = match config.read() {
            Ok(guard) => guard,
            Err(_) => return Redirect::to("https://example.com"),
        };
        config_guard
            .get_content()
            .get("default")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "https://example.com".to_string())
    };

    Redirect::to(default_url)
}

pub static FILE_NAME: &str = "config.json";

#[rocket::main]
async fn main() {
    println!("Loading configuration...");
    let config = Arc::new(RwLock::new(
        AppConfig::new(FILE_NAME).expect("Failed to load configuration file"),
    ));

    let config_for_watcher = config.clone();
    tokio::spawn(async move {
        if let Err(e) = watch_config_file(FILE_NAME, config_for_watcher).await {
            eprintln!("File watcher error: {}", e);
        }
    });

    let config_for_rocket = config.clone();
    tokio::spawn(async move {
        rocket::build()
            .manage(config_for_rocket) // Attach the config variable as managed state
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
                            }
                        };
                        add_url(app_name, app_url, config.clone());
                        reload(config.clone());
                    }
                    "r" => {
                        reload(config.clone());
                    }
                    "q" => {
                        println!("Exiting...");
                        break;
                    }
                    _ => {
                        println!("Unknown command: {}", trimmed);
                        print_help();
                    }
                }
            }
            Err(e) => eprintln!("Failed to read line: {}", e),
        }
    }
}
