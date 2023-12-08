use std::{fs, path::Path};

use tran::{
    config::{parse_config, write_config, Config},
    errors::TranError,
    png::recolor_png,
    recolor_textfile, ColorMap, ColorTransform,
};

fn get_config_path() -> Result<String, TranError> {
    let mut config_home = if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        config_home
    } else if let Ok(home) = std::env::var("HOME") {
        format!("{}/.config", home)
    } else {
        return Err(TranError::ConfigError(
            "Could not determine config directory".to_string(),
        ));
    };

    config_home.push_str("/tran/config");
    Ok(config_home)
}

fn main() -> Result<(), TranError> {
    let config_path = get_config_path()?;
    let config_path = std::path::Path::new(&config_path);

    if !config_path.is_file() {
        fs::write(config_path, "")?;
        eprintln!("Created empty config file, please fill it out");
        return Ok(());
    }

    let mut config = parse_config(config_path)?;

    let t = std::time::SystemTime::now();
    match &mut config {
        Config::GradientConfig(gc) => {
            let colors = gc.get_colors_scaled();
            let new_color = *colors
                .get(
                    t.duration_since(std::time::UNIX_EPOCH)
                        .expect("System time is before start of unix epoch")
                        .as_secs() as usize
                        % colors.len(),
                )
                .unwrap();

            let color_string = new_color.to_string();
            let old_color_string = gc.get_current_color().to_string();
            let trans = ColorTransform::Gradient {
                primary: &color_string,
                background: "#000000",
            };
            for target_file in gc.get_target_files() {
                let path = Path::new(&target_file);
                if !path.is_file() {
                    eprintln!("File {} could not be found", target_file);
                    continue;
                }

                if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                    if ext == "png" {
                        match gc.get_overwrite() {
                            true => recolor_png(path, path, &trans)?,
                            false => recolor_png(
                                path,
                                path.with_file_name(format!(
                                    "{}_{}",
                                    path.file_stem()
                                        .and_then(|p| p.to_str())
                                        .expect("Non utf-8 file name"),
                                    &new_color.to_string(),
                                ))
                                .with_extension("png"),
                                &trans,
                            )?,
                        }
                        continue;
                    }
                }

                if let Err(e) = recolor_textfile(path, &color_string, &old_color_string) {
                    eprintln!("Error recoloring {}: {}", target_file, e);
                }
            }
            gc.set_current_colors(new_color);
        }
        Config::MapConfig(mc) => {
            let colors = mc.get_colors_scaled();
            let new_color = colors
                .get(
                    t.duration_since(std::time::UNIX_EPOCH)
                        .expect("System time is before start of unix epoch")
                        .as_secs() as usize
                        % colors.len(),
                )
                .unwrap()
                .to_owned();

            let current_color = mc.get_current_colors();

            let store: Vec<(String, String)> = new_color
                .iter()
                .zip(current_color)
                .map(|(new, current)| (new.to_string(), current.to_string()))
                .collect();

            let map: Vec<ColorMap> = store
                .iter()
                .map(|(new, current)| ColorMap::new(new, current))
                .collect();
            let trans = ColorTransform::Map(&map);

            for target_file in mc.get_target_files() {
                let path = Path::new(&target_file);
                if !path.is_file() {
                    eprintln!("File {} could not be found", target_file);
                    continue;
                }

                if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                    if ext == "png" {
                        match mc.get_overwrite() {
                            true => recolor_png(path, path, &trans)?,
                            false => recolor_png(
                                path,
                                path.with_file_name(format!(
                                    "{}_{}",
                                    path.file_stem()
                                        .and_then(|p| p.to_str())
                                        .expect("Non utf-8 file name"),
                                    &new_color
                                        .get(1)
                                        .expect("No new color selectable")
                                        .to_string(),
                                ))
                                .with_extension("png"),
                                &trans,
                            )?,
                        }
                        continue;
                    }
                }

                for c in &map {
                    if let Err(e) = recolor_textfile(path, c.get_new_color(), c.get_current_color())
                    {
                        eprintln!("Error recoloring {}: {}", target_file, e);
                    }
                }
            }

            mc.set_current_colors(new_color.to_owned());
        }
    };

    write_config(config, config_path)?;

    Ok(())
}
