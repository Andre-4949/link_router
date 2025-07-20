use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::Value;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub content: Value,
    pub file_path: String,
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