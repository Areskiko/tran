use std::path::Path;

use errors::TranError;

pub mod errors;
pub mod config;
pub mod png;

pub type Color = str;

#[derive(Debug)]
pub struct ColorTransform<'a> {
    new_color: &'a Color,
    current_color: &'a Color,
}

impl<'a> ColorTransform<'a> {
    pub fn new(new_color: &'a Color, current_color: &'a Color) -> Self {
        ColorTransform {
            new_color,
            current_color
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
