use std::{fs, path::Path};

use rand::prelude::*;
use serde::{Deserialize, Serialize};

const CYAN: &str = "#6EE2FF";

type Color = str;

enum TranError {
    ConfigError(String),
    FileReadError(String),
    FileNotFound(String),
    WritingConfigError(String),
}

impl std::fmt::Display for TranError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranError::ConfigError(reason) => write!(f, "Error reading config: {}", reason),
            TranError::FileReadError(contents) => write!(f, "Error reading file {}", contents),
            TranError::FileNotFound(file_name) => write!(f, "Could not find file {}", file_name),
            TranError::WritingConfigError(contents) => {
                write!(f, "Could not write config file {}", contents)
            }
        }
    }
}

impl std::fmt::Debug for TranError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranError::ConfigError(reason) => write!(f, "Error reading config: {}", reason),
            TranError::FileReadError(contents) => write!(f, "Error reading file {}", contents),
            TranError::FileNotFound(file_name) => write!(f, "Could not find file {}", file_name),
            TranError::WritingConfigError(contents) => {
                write!(f, "Could not write config file {}", contents)
            }
        }
    }
}

impl std::error::Error for TranError {}

impl From<std::io::Error> for TranError {
    fn from(value: std::io::Error) -> Self {
        TranError::FileReadError(value.to_string())
    }
}

impl From<toml::ser::Error> for TranError {
    fn from(value: toml::ser::Error) -> Self {
        TranError::WritingConfigError(value.to_string())
    }
}

impl From<toml::de::Error> for TranError {
    fn from(value: toml::de::Error) -> Self {
        TranError::ConfigError(value.to_string())
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct IncompleteConfig {
    target_files: Option<Vec<String>>,
    current_color: Option<String>,
    colors: Option<Vec<String>>,
}

impl Default for IncompleteConfig {
    fn default() -> Self {
        IncompleteConfig {
            target_files: Some(Vec::new()),
            colors: Some(vec![CYAN.to_string()]),
            current_color: Some(CYAN.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    target_files: Vec<String>,
    current_color: String,
    colors: Vec<String>,
}

impl TryFrom<IncompleteConfig> for Config {
    type Error = TranError;

    fn try_from(value: IncompleteConfig) -> Result<Self, Self::Error> {
        Ok(Config {
            target_files: match value.target_files {
                Some(v) => v,
                None => Vec::new(),
            },
            colors: match value.colors {
                Some(v) => v,
                None => Vec::new(),
            },
            current_color: match value.current_color {
                Some(v) => v,
                None => return Err(TranError::ConfigError("Current color not set".to_string())),
            },
        })
    }
}

fn recolor_textfile<T: AsRef<Path>>(
    target: T,
    new_color: &Color,
    current_color: &Color,
) -> Result<(), TranError> {
    if !target.as_ref().is_file() {
        return Err(TranError::FileNotFound(
            target.as_ref().to_string_lossy().to_string(),
        ));
    }

    let file_contents = std::fs::read_to_string(&target)?;
    let updated_file_contents = file_contents.replace(current_color, new_color);

    std::fs::write(target, updated_file_contents)?;

    Ok(())
}

const PNG_FORMAT_IDENTIFIER: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const IHDR_COLOR_TYPE_OFFSET: usize = 9;
const IHDR: u32 = 0x49484452;
const IEND: u32 = 0x49454E44;
const PLTE: u32 = 0x504C5445;
const IDAT: u32 = 0x49444154;

#[derive(Debug)]
enum PngColorType {
    Grayscale,      // 0
    RGB,            // 2
    Palette,        // 3
    GrayscaleAlpha, // 4
    RGBA,           // 6
}

impl TryFrom<u8> for PngColorType {
    type Error = TranError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PngColorType::Grayscale),
            2 => Ok(PngColorType::RGB),
            3 => Ok(PngColorType::Palette),
            4 => Ok(PngColorType::GrayscaleAlpha),
            6 => Ok(PngColorType::RGBA),
            _ => Err(TranError::FileReadError(format!(
                "PNG Color type is invalid {}",
                value
            ))),
        }
    }
}

impl TryFrom<&u8> for PngColorType {
    type Error = TranError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        PngColorType::try_from(*value)
    }
}

#[derive(Debug)]
struct ColorTransform<'a> {
    new_color: &'a Color,
    current_color: &'a Color,
}

impl<'a> ColorTransform<'a> {
    fn new_color_bytes(&self) -> Result<(u8, u8, u8), TranError> {
        let bytes: u32 = u32::from_str_radix(&self.new_color[1..], 16).map_err(|_| {
            TranError::ConfigError(format!("Color hex {} is invalid", self.new_color))
        })?;

        let red: u8 = ((bytes & 0xFF0000) >> 2 * 8) as u8;
        let green: u8 = ((bytes & 0x00FF00) >> 8) as u8;
        let blue: u8 = (bytes & 0x0000FF) as u8;

        Ok((red, green, blue))
    }
    fn current_color_bytes(&self) -> Result<(u8, u8, u8), TranError> {
        let bytes: u32 = u32::from_str_radix(&self.current_color[1..], 16).map_err(|_| {
            TranError::ConfigError(format!("Color hex {} is invalid", self.current_color))
        })?;

        let red: u8 = ((bytes & 0xFF0000) >> 2 * 8) as u8;
        let green: u8 = ((bytes & 0x00FF00) >> 8) as u8;
        let blue: u8 = (bytes & 0x0000FF) as u8;

        Ok((red, green, blue))
    }
}

struct Chunk<'a> {
    length: u32,
    chunk_type: u32,
    chunk_data: Vec<&'a mut u8>,
    crc: [&'a mut u8; 4],
}

fn read_chunk<'a>(png: &'a mut std::slice::IterMut<u8>) -> Result<Chunk<'a>, TranError> {
    let length = [
        png.next().cloned(),
        png.next().cloned(),
        png.next().cloned(),
        png.next().cloned(),
    ]
    .iter()
    .filter_map(|byte| *byte)
    .enumerate()
    .map(|(index, byte)| (byte as u32) << 8 * (3 - index))
    .reduce(|acc, byte| acc | byte)
    .ok_or(TranError::FileReadError(
        "Something went wrong while reducing length".to_string(),
    ))?;

    let chunk_type = [
        png.next().cloned(),
        png.next().cloned(),
        png.next().cloned(),
        png.next().cloned(),
    ]
    .iter()
    .filter_map(|byte| *byte)
    .enumerate()
    .map(|(index, byte)| (byte as u32) << 8 * (3 - index))
    .reduce(|acc, byte| acc | byte)
    .ok_or(TranError::FileReadError(
        "Something went wrong while reducing chunk type".to_string(),
    ))?;

    let mut chunk_data: Vec<&mut u8> = Vec::with_capacity(length as usize);
    for _ in 0..length {
        chunk_data.push(
            png.next()
                .ok_or(TranError::FileReadError("Ran out of bytes".to_string()))?,
        );
    }

    let crc = [
        png.next()
            .ok_or(TranError::FileReadError("Ran out of bytes".to_string()))?,
        png.next()
            .ok_or(TranError::FileReadError("Ran out of bytes".to_string()))?,
        png.next()
            .ok_or(TranError::FileReadError("Ran out of bytes".to_string()))?,
        png.next()
            .ok_or(TranError::FileReadError("Ran out of bytes".to_string()))?,
    ];

    Ok(Chunk {
        length,
        chunk_type,
        chunk_data,
        crc,
    })
}

fn crc(buf: &[&mut u8]) -> u32 {
    let mut crc_table: [u32; 256] = [0; 256];

    for n in 0..256 {
        let mut c: u32 = n;
        for _ in 0..8 {
            if (c & 1) != 0 {
                c = 0xEDB88320 ^ (c >> 1);
            } else {
                c = c >> 1;
            }
        }
        crc_table[n as usize] = c;
    }

    let mut c: u32 = 0xffffffff;
    for i in 0..buf.len() {
        c = crc_table[((c ^ (*(buf[i])) as u32) & 0xff) as usize] ^ (c >> 8);
    }
    c ^ 0xffffffff
}

fn recolor_png<T: AsRef<Path>>(target: T, transform: &[&ColorTransform]) -> Result<(), TranError> {
    if !target.as_ref().is_file() {
        return Err(TranError::FileNotFound(
            target.as_ref().to_string_lossy().to_string(),
        ));
    }

    let mut file = std::fs::read(&target)?;
    let mut png = file.iter_mut();

    for i in 0..8 {
        if let Some(read_byte) = png.next() {
            if *read_byte != PNG_FORMAT_IDENTIFIER[i] {
                return Err(TranError::FileReadError(format!(
                    "{} is not a png as {:x} != {:x}",
                    target.as_ref().to_string_lossy(),
                    *read_byte,
                    PNG_FORMAT_IDENTIFIER[i]
                )));
            }
        } else {
            return Err(TranError::FileReadError(format!(
                "{} is not a png as png next failed",
                target.as_ref().to_string_lossy()
            )));
        }
    }

    let ihdr = read_chunk(&mut png)?;
    if ihdr.chunk_type != IHDR {
        return Err(TranError::FileReadError(format!(
            "{} is not a png as it does not contain IHDR chunk {:x} != {:x}",
            target.as_ref().to_string_lossy(),
            ihdr.chunk_type,
            IHDR
        )));
    }

    let color_type: PngColorType = (**ihdr
        .chunk_data
        .get(IHDR_COLOR_TYPE_OFFSET)
        .ok_or(TranError::FileReadError("No color type".to_string()))?)
    .try_into()?;

    if let PngColorType::Grayscale | PngColorType::GrayscaleAlpha = color_type {
        return Ok(());
    }

    if let PngColorType::RGB | PngColorType::RGBA = color_type {
        return Err(TranError::FileReadError(
            "Can't decompress png of type RGB".to_string(),
        ));
    }

    loop {
        let mut chunk = read_chunk(&mut png)?;

        match color_type {
            PngColorType::Palette => {
                if chunk.chunk_type == PLTE {
                    let mut pixels = chunk.chunk_data.iter_mut();
                    for _ in 0..chunk.length / 3 {
                        let red = pixels.next().ok_or(TranError::FileReadError(
                            "Could not read next pixel".to_string(),
                        ))?;
                        let green = pixels.next().ok_or(TranError::FileReadError(
                            "Could not read next pixel".to_string(),
                        ))?;
                        let blue = pixels.next().ok_or(TranError::FileReadError(
                            "Could not read next pixel".to_string(),
                        ))?;

                        if let Some(mapping) = transform.iter().find(|color_transform| {
                            color_transform.current_color_bytes().unwrap().0 == **red
                                && color_transform.current_color_bytes().unwrap().1 == **green
                                && color_transform.current_color_bytes().unwrap().2 == **blue
                        }) {
                            let (new_red, new_green, new_blue) = mapping.new_color_bytes()?;
                            **red = mapping.new_color_bytes()?.0;
                            **green = mapping.new_color_bytes()?.1;
                            **blue = mapping.new_color_bytes()?.2;
                        }
                    }
                    // Recalculate CRC
                    let mut crc_data: Vec<&mut u8> = Vec::with_capacity(4 + chunk.chunk_data.len());
                    let mut chunk_type = (
                        (((chunk.chunk_type & 0xFF000000) >> (3 * 8)) as u8),
                        (((chunk.chunk_type & 0x00FF0000) >> (2 * 8)) as u8),
                        (((chunk.chunk_type & 0x0000FF00) >> (1 * 8)) as u8),
                        (((chunk.chunk_type & 0x000000FF) >> (0 * 8)) as u8),
                    );
                    crc_data.push(&mut chunk_type.0);
                    crc_data.push(&mut chunk_type.1);
                    crc_data.push(&mut chunk_type.2);
                    crc_data.push(&mut chunk_type.3);

                    crc_data.extend(chunk.chunk_data);

                    let new_crc = crc(crc_data.as_slice());
                    *chunk.crc[0] = ((new_crc & (0xFF000000)) >> (3 * 8)) as u8;
                    *chunk.crc[1] = ((new_crc & (0x00FF0000)) >> (2 * 8)) as u8;
                    *chunk.crc[2] = ((new_crc & (0x0000FF00)) >> (1 * 8)) as u8;
                    *chunk.crc[3] = ((new_crc & (0x000000FF)) >> (0 * 8)) as u8;
                }
            }
            PngColorType::RGB | PngColorType::RGBA => {
                if chunk.chunk_type == IDAT {
                    todo!()
                }
            }
            _ => unreachable!(),
        }

        if chunk.chunk_type == IEND {
            break;
        }
    }

    std::fs::write(&target, file)?;

    Ok(())
}

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

        if let Some(ext) = path.extension().map(|ext| ext.to_str()).flatten() {
            if ext == "png" {
                let trans = ColorTransform {
                    new_color: new_color,
                    current_color: &config.current_color,
                };
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
