use std::fs;

use crate::{entities::Config, error::CloudError};

pub fn setup(profile_name: &str, file_path: &str) -> Result<(), CloudError> {
    let home_dir = dirs::home_dir().expect("Could not find home directory");
    let config_dir = home_dir.join(".config").join("pompilius");
    let config_path = config_dir.join("config.toml");

    fs::create_dir_all(&config_dir)?;

    let mut config = if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        toml::from_str(&content).unwrap_or_default()
    } else {
        Config::default()
    };

    config.profiles.retain(|_, v| v != file_path);

    config
        .profiles
        .insert(profile_name.to_string(), file_path.to_string());

    let toml_string = toml::to_string_pretty(&config)?;
    fs::write(config_path, toml_string)?;

    Ok(())
}
