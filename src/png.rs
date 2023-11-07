use std::path::Path;

use crate::{errors::TranError, hex_to_bytes, ColorTransform};

const PNG_FORMAT_IDENTIFIER: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const IHDR_COLOR_TYPE_OFFSET: usize = 9;
const IHDR: u32 = 0x49484452;
const IEND: u32 = 0x49454E44;
const PLTE: u32 = 0x504C5445;
const IDAT: u32 = 0x49444154;

#[derive(Debug)]
enum PngColorType {
    Grayscale,      // 0
    Rgb,            // 2
    Palette,        // 3
    GrayscaleAlpha, // 4
    Rgba,           // 6
}

impl TryFrom<u8> for PngColorType {
    type Error = TranError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PngColorType::Grayscale),
            2 => Ok(PngColorType::Rgb),
            3 => Ok(PngColorType::Palette),
            4 => Ok(PngColorType::GrayscaleAlpha),
            6 => Ok(PngColorType::Rgba),
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
    .map(|(index, byte)| (byte as u32) << (8 * (3 - index)))
    .reduce(|acc, byte| acc | byte)
    .ok_or_else(|| {
        TranError::FileReadError("Something went wrong while reducing length".to_string())
    })?;

    let chunk_type = [
        png.next().cloned(),
        png.next().cloned(),
        png.next().cloned(),
        png.next().cloned(),
    ]
    .iter()
    .filter_map(|byte| *byte)
    .enumerate()
    .map(|(index, byte)| (byte as u32) << (8 * (3 - index)))
    .reduce(|acc, byte| acc | byte)
    .ok_or_else(|| {
        TranError::FileReadError("Something went wrong while reducing chunk type".to_string())
    })?;

    let mut chunk_data: Vec<&mut u8> = Vec::with_capacity(length as usize);
    for _ in 0..length {
        chunk_data.push(
            png.next()
                .ok_or_else(|| TranError::FileReadError("Ran out of bytes".to_string()))?,
        );
    }

    let crc = [
        png.next()
            .ok_or_else(|| TranError::FileReadError("Ran out of bytes".to_string()))?,
        png.next()
            .ok_or_else(|| TranError::FileReadError("Ran out of bytes".to_string()))?,
        png.next()
            .ok_or_else(|| TranError::FileReadError("Ran out of bytes".to_string()))?,
        png.next()
            .ok_or_else(|| TranError::FileReadError("Ran out of bytes".to_string()))?,
    ];

    Ok(Chunk {
        length,
        chunk_type,
        chunk_data,
        crc,
    })
}

struct GeneratedColorMap {
    old_colors: (u8, u8, u8),
    new_colors: (u8, u8, u8),
}

pub fn recolor_png<S: AsRef<Path>, T: AsRef<Path>>(source: S, target: T, transform: &ColorTransform) -> Result<(), TranError> {
    if !source.as_ref().is_file() {
        return Err(TranError::FileNotFoundError(
            source.as_ref().to_string_lossy().to_string(),
        ));
    }

    let mut file = std::fs::read(&source)?;
    let mut png = file.iter_mut();

    for png_format_identifier_byte in PNG_FORMAT_IDENTIFIER {
        if let Some(read_byte) = png.next() {
            if *read_byte != png_format_identifier_byte {
                return Err(TranError::FileReadError(format!(
                    "{} is not a png as {:x} != {:x}",
                    source.as_ref().to_string_lossy(),
                    *read_byte,
                    png_format_identifier_byte
                )));
            }
        } else {
            return Err(TranError::FileReadError(format!(
                "{} is not a png as png next failed",
                source.as_ref().to_string_lossy()
            )));
        }
    }

    let ihdr = read_chunk(&mut png)?;
    if ihdr.chunk_type != IHDR {
        return Err(TranError::FileReadError(format!(
            "{} is not a png as it does not contain IHDR chunk {:x} != {:x}",
            source.as_ref().to_string_lossy(),
            ihdr.chunk_type,
            IHDR
        )));
    }

    let color_type: PngColorType = (**ihdr
        .chunk_data
        .get(IHDR_COLOR_TYPE_OFFSET)
        .ok_or_else(|| TranError::FileReadError("No color type".to_string()))?)
    .try_into()?;

    if let PngColorType::Grayscale | PngColorType::GrayscaleAlpha = color_type {
        return Ok(());
    }

    if let PngColorType::Rgb | PngColorType::Rgba = color_type {
        return Err(TranError::FileReadError(
            "Can't decompress png of type RGB".to_string(),
        ));
    }

    match color_type {
        PngColorType::Palette => {
            loop {
                let mut chunk = read_chunk(&mut png)?;
                if chunk.chunk_type == PLTE {
                    let mut pixels = chunk.chunk_data.iter_mut();
                    let mut colors = Vec::with_capacity((chunk.length / 3) as usize);
                    for _ in 0..chunk.length / 3 {
                        let red = pixels.next().ok_or_else(|| {
                            TranError::FileReadError("Could not read red pixel".to_string())
                        })?;
                        let green = pixels.next().ok_or_else(|| {
                            TranError::FileReadError("Could not read green pixel".to_string())
                        })?;
                        let blue = pixels.next().ok_or_else(|| {
                            TranError::FileReadError("Could not read blue pixel".to_string())
                        })?;

                        if (**red == 0 && **green == 0 && **blue == 0)
                            || (**red == 255 && **green == 255 && **blue == 255)
                        {
                            continue;
                        }

                        colors.push((red, green, blue));
                    }

                    match transform {
                        ColorTransform::Map(map) => {
                            for trans in map.iter() {
                                for color in colors.iter_mut() {
                                    if **color.0 == trans.current_color_bytes()?.0
                                        && **color.1 == trans.current_color_bytes()?.1
                                        && **color.2 == trans.current_color_bytes()?.2
                                    {
                                        **color.0 = trans.new_color_bytes()?.0;
                                        **color.1 = trans.new_color_bytes()?.1;
                                        **color.2 = trans.new_color_bytes()?.2;
                                    }
                                }
                            }
                        }
                        ColorTransform::Gradient {
                            primary,
                            background: _,
                        } => {
                            colors.sort_unstable_by(|a, b| {
                                (**b.0 as u64 + **b.1 as u64 + **b.2 as u64)
                                    .cmp(&(**a.0 as u64 + **a.1 as u64 + **a.2 as u64))
                            });
                            let mut map: Vec<GeneratedColorMap> = Vec::with_capacity(colors.len());
                            let first_color = colors.get(0).ok_or_else(|| {
                                TranError::PngFormatError("No colors".to_string())
                            })?;
                            map.push(GeneratedColorMap {
                                new_colors: hex_to_bytes(primary)?,
                                old_colors: (**first_color.0, **first_color.1, **first_color.2),
                            });

                            for i in 1..colors.len() {
                                let previous_new = map
                                    .get(i - 1)
                                    .ok_or_else(|| {
                                        TranError::PngFormatError("No colors".to_string())
                                    })?
                                    .new_colors;
                                let previous_old = colors.get(i - 1).ok_or_else(|| {
                                    TranError::PngFormatError("No colors".to_string())
                                })?;
                                let next_old = colors.get(i).ok_or_else(|| {
                                    TranError::PngFormatError("No colors".to_string())
                                })?;

                                let red_diff = (**next_old.0 as f64) / (**previous_old.0 as f64);
                                let grenn_diff = (**next_old.1 as f64) / (**previous_old.1 as f64);
                                let blue_diff = (**next_old.2 as f64) / (**previous_old.2 as f64);

                                let next_new = (
                                    ((previous_new.0 as f64) * red_diff) as u8,
                                    ((previous_new.1 as f64) * grenn_diff) as u8,
                                    ((previous_new.2 as f64) * blue_diff) as u8,
                                );
                                map.push(GeneratedColorMap {
                                    new_colors: next_new,
                                    old_colors: (**next_old.0, **next_old.1, **next_old.2),
                                });
                            }

                            for trans in map.iter() {
                                for color in colors.iter_mut() {
                                    if **color.0 == trans.old_colors.0
                                        && **color.1 == trans.old_colors.1
                                        && **color.2 == trans.old_colors.2
                                    {
                                        **color.0 = trans.new_colors.0;
                                        **color.1 = trans.new_colors.1;
                                        **color.2 = trans.new_colors.2;
                                    }
                                }
                            }
                        }
                    }

                    // Recalculate CRC
                    let mut crc_data: Vec<&mut u8> = Vec::with_capacity(4 + chunk.chunk_data.len());
                    let mut chunk_type = (
                        (((chunk.chunk_type & 0xFF000000) >> (3 * 8)) as u8),
                        (((chunk.chunk_type & 0x00FF0000) >> (2 * 8)) as u8),
                        (((chunk.chunk_type & 0x0000FF00) >> 8) as u8),
                        ((chunk.chunk_type & 0x000000FF) as u8),
                    );
                    crc_data.push(&mut chunk_type.0);
                    crc_data.push(&mut chunk_type.1);
                    crc_data.push(&mut chunk_type.2);
                    crc_data.push(&mut chunk_type.3);

                    crc_data.extend(chunk.chunk_data);

                    let new_crc = crc(crc_data.as_slice());
                    *chunk.crc[0] = ((new_crc & (0xFF000000)) >> (3 * 8)) as u8;
                    *chunk.crc[1] = ((new_crc & (0x00FF0000)) >> (2 * 8)) as u8;
                    *chunk.crc[2] = ((new_crc & (0x0000FF00)) >> 8) as u8;
                    *chunk.crc[3] = (new_crc & (0x000000FF)) as u8;
                }
                if chunk.chunk_type == IEND {
                    break;
                }
            }
        }
        PngColorType::Rgb | PngColorType::Rgba => {
            todo!()
        }
        _ => unreachable!(),
    }

    std::fs::write(&target, file)?;

    Ok(())
}

fn crc(buf: &[&mut u8]) -> u32 {
    let mut crc_table: [u32; 256] = [0; 256];

    for n in 0..256 {
        let mut c: u32 = n;
        for _ in 0..8 {
            if (c & 1) != 0 {
                c = 0xEDB88320 ^ (c >> 1);
            } else {
                c >>= 1;
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
