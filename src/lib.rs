use std::path::Path;

use errors::TranError;

pub mod config;
pub mod errors;
pub mod png;

pub type Color = str;

pub enum ColorTransform<'a, 'b> {
    Map(&'b [ColorMap<'a>]),
    Gradient {
        primary: &'a Color,
        background: &'a Color,
    },
}

#[derive(Debug)]
pub struct ColorMap<'a> {
    new_color: &'a Color,
    current_color: &'a Color,
}

impl<'a> ColorMap<'a> {
    pub fn new(new_color: &'a Color, current_color: &'a Color) -> Self {
        ColorMap {
            new_color,
            current_color,
        }
    }
    fn new_color_bytes(&self) -> Result<(u8, u8, u8), TranError> {
        let bytes: u32 = u32::from_str_radix(&self.new_color[1..], 16).map_err(|_| {
            TranError::ConfigError(format!("Color hex {} is invalid", self.new_color))
        })?;

        let red: u8 = ((bytes & 0xFF0000) >> (2 * 8)) as u8;
        let green: u8 = ((bytes & 0x00FF00) >> 8) as u8;
        let blue: u8 = (bytes & 0x0000FF) as u8;

        Ok((red, green, blue))
    }
    fn current_color_bytes(&self) -> Result<(u8, u8, u8), TranError> {
        let bytes: u32 = u32::from_str_radix(&self.current_color[1..], 16).map_err(|_| {
            TranError::ConfigError(format!("Color hex {} is invalid", self.current_color))
        })?;

        let red: u8 = ((bytes & 0xFF0000) >> (2 * 8)) as u8;
        let green: u8 = ((bytes & 0x00FF00) >> 8) as u8;
        let blue: u8 = (bytes & 0x0000FF) as u8;

        Ok((red, green, blue))
    }

    pub fn get_new_color(&self) -> &str {
        self.new_color
    }

    pub fn get_current_color(&self) -> &str {
        self.current_color
    }
}

fn hex_to_bytes(hex: &str) -> Result<(u8, u8, u8), TranError> {
    let bytes: u32 = u32::from_str_radix(&hex[1..], 16)
        .map_err(|_| TranError::ConfigError(format!("Color hex {} is invalid", hex)))?;

    let red: u8 = ((bytes & 0xFF0000) >> (2 * 8)) as u8;
    let green: u8 = ((bytes & 0x00FF00) >> 8) as u8;
    let blue: u8 = (bytes & 0x0000FF) as u8;

    Ok((red, green, blue))
}

pub fn recolor_textfile<T: AsRef<Path>>(
    target: T,
    new_color: &Color,
    current_color: &Color,
) -> Result<(), TranError> {
    if !target.as_ref().is_file() {
        return Err(TranError::FileNotFoundError(
            target.as_ref().to_string_lossy().to_string(),
        ));
    }

    let file_contents = std::fs::read_to_string(&target)?;
    let updated_file_contents = file_contents.replace(current_color, new_color);

    std::fs::write(target, updated_file_contents)?;

    Ok(())
}
