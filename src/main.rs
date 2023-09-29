use std::{fs, path::Path};

use rand::prelude::*;

use tran::{
    config::{Config, IncompleteConfig},
    errors::TranError,
    png::recolor_png,
    recolor_textfile, ColorTransform,
};

fn main() -> Result<(), TranError> {
    let mut config_path = dirs::config_dir().ok_or(TranError::ConfigError(
        "Could not find config dir".to_string(),
    ))?;
    config_path.push("tran");

    if !config_path.is_dir() {
        fs::create_dir(&config_path)?;
    }

    config_path.push("config.toml");

    if !config_path.is_file() {
        fs::write(&config_path, toml::to_string(&IncompleteConfig::default())?)?;
        eprintln!("Created empty config file, please fill it out");
        return Ok(());
    }

    let config_raw = fs::read_to_string(&config_path)?;
    let incomplete_config: IncompleteConfig = toml::from_str(&config_raw)?;
    let mut config: Config = incomplete_config.try_into()?;

    let mut rng = rand::thread_rng();
    let new_color = config
        .colors
        .get(rng.gen_range(0..config.colors.len()))
        .unwrap();

    for target_file in &config.target_files {
        let path = Path::new(&target_file);
        if !path.is_file() {
            eprintln!("File {} could not be found", target_file);
            continue;
        }

        if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
            if ext == "png" {
                let trans = ColorTransform::new(new_color, &config.current_color);
                let transform = vec![&trans];
                recolor_png(path, transform.as_slice())?;
                continue;
            }
        }

        if let Err(e) = recolor_textfile(path, new_color, &config.current_color) {
            eprintln!("Error recoloring {}: {}", target_file, e);
        }
    }

    config.current_color = new_color.to_owned();
    fs::write(config_path, toml::to_string(&config)?)?;

    Ok(())
}
