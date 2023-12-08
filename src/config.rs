use std::{io::Write, path::Path};

use crate::errors::TranError;

#[derive(PartialEq)]
enum ParseState {
    Start,
    BraceOpen,
    BraceClosed,
    NewLine,
    Text,
}

#[derive(PartialEq)]
pub enum Section {
    Mode,
    CurrentColor,
    Colors,
    TargetFiles,
    Overwrite,
}

impl TryFrom<&str> for Section {
    type Error = TranError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "mode" => Ok(Self::Mode),
            "colors" => Ok(Self::Colors),
            "target_files" => Ok(Self::TargetFiles),
            "current_color" => Ok(Self::CurrentColor),
            "overwrite" => Ok(Self::Overwrite),
            _ => Err(TranError::ConfigError(format!("Unrecognized section'{}', valid sections are 'mode', 'current_color', 'colors', and 'target_files'", value)))
        }
    }
}

pub enum Mode {
    Gradient,
    Map,
}

impl TryFrom<&str> for Mode {
    type Error = TranError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "map" => Ok(Mode::Map),
            "gradient" => Ok(Mode::Gradient),
            _ => Err(TranError::ConfigError(format!(
                "Unrecognized mode '{}', valid modes are 'map' and 'gradient'",
                value
            ))),
        }
    }
}

enum ColorOrMap {
    Color(Color),
    Map(Vec<Color>),
}

enum ColorOrMapVec {
    Color(Vec<Color>),
    Map(Vec<Vec<Color>>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

impl Color {
    pub fn black() -> Self {
        Color {
            red: 0,
            green: 0,
            blue: 0,
        }
    }
    pub fn white() -> Self {
        Color {
            red: 255,
            green: 255,
            blue: 255,
        }
    }

    pub fn from_bytes(red: u8, green: u8, blue: u8) -> Self {
        Color { red, green, blue }
    }

    pub fn bytes(&self) -> (u8, u8, u8) {
        (self.red, self.green, self.blue)
    }

    pub fn try_from_hex_str<S: AsRef<str>>(s: S) -> Result<Self, TranError> {
        let s = s.as_ref();
        let (r, g, b) = if s.len() == 6 {
            // No preceding #
            (s.get(0..2), s.get(2..4), s.get(4..6))
        } else if s.len() == 7 {
            // Preceding #
            (s.get(1..3), s.get(3..5), s.get(5..7))
        } else {
            return Err(TranError::ConfigError(format!(
                "Could not interpret {} as hex color",
                s
            )));
        };

        if let (Some(r), Some(g), Some(b)) = (r, g, b) {
            Ok(Color::from_bytes(
                u8::from_str_radix(r, 16)?,
                u8::from_str_radix(g, 16)?,
                u8::from_str_radix(b, 16)?,
            ))
        } else {
            Err(TranError::ConfigError(format!(
                "Something went wrong while parsing {}",
                s
            )))
        }
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }
}

impl TryFrom<&str> for Color {
    type Error = TranError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Color::try_from_hex_str(value)
    }
}

impl From<Color> for String {
    fn from(value: Color) -> Self {
        value.to_string()
    }
}

#[derive(Debug)]
pub enum Config {
    GradientConfig(GradientConfig),
    MapConfig(MapConfig),
}

impl Config {
    pub fn get_target_files(&self) -> &[String] {
        match self {
            Config::GradientConfig(gc) => gc.get_target_files(),
            Config::MapConfig(mc) => mc.get_target_files(),
        }
    }

    pub fn get_mode(&self) -> &str {
        match self {
            Config::GradientConfig(_) => "gradient",
            Config::MapConfig(_) => "map",
        }
    }
}

#[derive(Debug)]
pub struct GradientConfig {
    current_color: Color,
    colors: Vec<Color>,
    weights: Vec<usize>,
    target_files: Vec<String>,
    overwrite: bool,
}

impl GradientConfig {
    pub fn set_current_colors(&mut self, color: Color) {
        self.current_color = color
    }

    pub fn get_current_color(&self) -> &Color {
        &self.current_color
    }

    pub fn get_colors(&self) -> &[Color] {
        &self.colors
    }

    pub fn get_colors_scaled(&self) -> Vec<Color> {
        let mut output = Vec::new();

        for (i, color) in self.get_colors().iter().enumerate() {
            if color == self.get_current_color() {
                continue;
            }

            let w = match self.weights.get(i) {
                Some(w) => *w,
                None => 1,
            };

            for _ in 0..w {
                output.push(*color)
            }
        }

        output
    }

    pub fn get_target_files(&self) -> &[String] {
        &self.target_files
    }

    pub fn get_overwrite(&self) -> bool {
        self.overwrite
    }
}

#[derive(Debug)]
pub struct MapConfig {
    current_color: Vec<Color>,
    colors: Vec<Vec<Color>>,
    weights: Vec<usize>,
    target_files: Vec<String>,
    overwrite: bool,
}

impl MapConfig {
    pub fn set_current_colors(&mut self, color: Vec<Color>) {
        self.current_color = color
    }

    pub fn get_current_colors(&self) -> &[Color] {
        &self.current_color
    }

    pub fn get_colors(&self) -> &[Vec<Color>] {
        &self.colors
    }

    pub fn get_colors_scaled(&self) -> Vec<&Vec<Color>> {
        let mut output = Vec::new();

        for (i, color) in self.get_colors().iter().enumerate() {
            let w = match self.weights.get(i) {
                Some(w) => *w,
                None => 1,
            };
            for _ in 0..w {
                output.push(color)
            }
        }

        output
    }

    pub fn get_target_files(&self) -> &[String] {
        &self.target_files
    }

    pub fn get_overwrite(&self) -> bool {
        self.overwrite
    }
}

const BUFF_SIZE: usize = 50;

pub fn parse_config<T: AsRef<Path>>(target: T) -> Result<Config, TranError> {
    let contents = std::fs::read_to_string(target)?;
    let chars = contents.trim().chars();
    let mut state = ParseState::Start;
    let mut section = Section::Mode;
    let mut buff = String::with_capacity(BUFF_SIZE);

    let mut mode: Option<Mode> = None;
    let mut current_color: ColorOrMap = ColorOrMap::Color(Color::black());
    let mut colors: Option<ColorOrMapVec> = None;
    let mut target_files: Vec<String> = Vec::new();
    let mut overwrite: bool = false;
    let mut weights: Vec<usize> = Vec::new();

    for char in chars {
        match state {
            ParseState::Start => {
                if char != '[' {
                    return Err(TranError::ConfigError(format!(
                        "Expected config to start with '[', found {}",
                        char
                    )));
                }
                state = ParseState::BraceOpen;
            }
            ParseState::BraceOpen => {
                if char == ']' {
                    section = buff.as_str().try_into()?;
                    buff.clear();
                    state = ParseState::BraceClosed;
                } else {
                    buff.push(char);
                }
            }
            ParseState::BraceClosed => {
                if char != '\n' {
                    return Err(TranError::ConfigError(format!(
                        "Expected newline after section declaration, found {}",
                        char
                    )));
                } else {
                    state = ParseState::NewLine;
                }
            }
            ParseState::Text => {
                if char == '\n' {
                    // Add contents from buff to propper storage
                    match section {
                        Section::Mode => {
                            mode = Some(buff.as_str().try_into()?);
                            buff.clear();
                        }
                        Section::Colors => {
                            if let Some(m) = &mode {
                                match m {
                                    Mode::Gradient => match &mut colors {
                                        Some(c) => {
                                            if let ColorOrMapVec::Color(v) = c {
                                                let mut entire = buff.split('#');

                                                weights.push(
                                                    entire
                                                        .next()
                                                        .and_then(|c| {
                                                            usize::from_str_radix(c, 10).ok()
                                                        })
                                                        .unwrap_or(1),
                                                );
                                                v.push(Color::try_from_hex_str(
                                                    &entire.next().ok_or_else(|| {
                                                        TranError::ConfigError(
                                                            "Failed to parse color value"
                                                                .to_string(),
                                                        )
                                                    })?,
                                                )?);
                                                buff.clear();
                                            } else {
                                                return Err(TranError::ConfigError(
                                                    "Inconsistent state".to_string(),
                                                ));
                                            }
                                        }
                                        None => {
                                            let mut entire = buff.split('#');

                                            weights.push(
                                                entire
                                                    .next()
                                                    .and_then(|c| usize::from_str_radix(c, 10).ok())
                                                    .unwrap_or(1),
                                            );
                                            colors = Some(ColorOrMapVec::Color(vec![
                                                Color::try_from_hex_str(
                                                    &entire.next().ok_or_else(|| {
                                                        TranError::ConfigError(
                                                            "Failed to parse color value"
                                                                .to_string(),
                                                        )
                                                    })?,
                                                )?,
                                            ]));
                                            buff.clear();
                                        }
                                    },
                                    Mode::Map => {
                                        let mut entire = buff.split('#');
                                        weights.push(
                                            entire
                                                .next()
                                                .and_then(|c| usize::from_str_radix(c, 10).ok())
                                                .unwrap_or(1),
                                        );
                                        let color_map = entire
                                            .map(Color::try_from_hex_str)
                                            .collect::<Result<Vec<Color>, TranError>>(
                                        )?;
                                        match &mut colors {
                                            Some(c) => {
                                                if let ColorOrMapVec::Map(v) = c {
                                                    v.push(color_map);
                                                } else {
                                                    return Err(TranError::ConfigError(
                                                        "Inconsistent state".to_string(),
                                                    ));
                                                }
                                            }
                                            None => {
                                                colors = Some(ColorOrMapVec::Map(vec![color_map]));
                                            }
                                        }
                                        buff.clear();
                                    }
                                }
                            } else {
                                return Err(TranError::ConfigError("Found color section before mode section. Can't determine color format".to_string()));
                            }
                        }
                        Section::CurrentColor => {
                            if let Some(m) = &mode {
                                match m {
                                    Mode::Gradient => {
                                        current_color =
                                            ColorOrMap::Color(Color::try_from_hex_str(&buff)?);
                                        buff.clear();
                                    }
                                    Mode::Map => {
                                        current_color = ColorOrMap::Map(
                                            buff.split('#')
                                                .map(Color::try_from_hex_str)
                                                .collect::<Result<Vec<Color>, TranError>>()?,
                                        );
                                    }
                                }
                            } else {
                                return Err(TranError::ConfigError("Found color section before mode section. Can't determine color format".to_string()));
                            }
                        }
                        Section::TargetFiles => {
                            target_files.push(buff);
                            buff = String::with_capacity(BUFF_SIZE);
                        }
                        Section::Overwrite => {
                            if buff == "true" {
                                overwrite = true;
                            }
                            buff.clear()
                        }
                    }
                    state = ParseState::NewLine;
                } else {
                    buff.push(char);
                }
            }
            ParseState::NewLine => {
                if char == '[' {
                    state = ParseState::BraceOpen;
                } else {
                    buff.push(char);
                    state = ParseState::Text;
                }
            }
        }
    }

    if buff.len() != 0 {
        match section {
            Section::Mode => {
                mode = Some(buff.as_str().try_into()?);
                buff.clear();
            }
            Section::Colors => {
                if let Some(m) = &mode {
                    match m {
                        Mode::Gradient => match &mut colors {
                            Some(c) => {
                                if let ColorOrMapVec::Color(v) = c {
                                    v.push(Color::try_from_hex_str(&buff)?);
                                    buff.clear();
                                } else {
                                    return Err(TranError::ConfigError(
                                        "Inconsistent state".to_string(),
                                    ));
                                }
                            }
                            None => {
                                colors = Some(ColorOrMapVec::Color(vec![Color::try_from_hex_str(
                                    &buff,
                                )?]));
                                buff.clear();
                            }
                        },
                        Mode::Map => {
                            let color_map = buff
                                .split('#')
                                .map(Color::try_from_hex_str)
                                .collect::<Result<Vec<Color>, TranError>>()?;
                            match &mut colors {
                                Some(c) => {
                                    if let ColorOrMapVec::Map(v) = c {
                                        v.push(color_map);
                                    } else {
                                        return Err(TranError::ConfigError(
                                            "Inconsistent state".to_string(),
                                        ));
                                    }
                                }
                                None => {
                                    colors = Some(ColorOrMapVec::Map(vec![color_map]));
                                }
                            }
                            buff.clear();
                        }
                    }
                } else {
                    return Err(TranError::ConfigError(
                        "Found color section before mode section. Can't determine color format"
                            .to_string(),
                    ));
                }
            }
            Section::CurrentColor => {
                if let Some(m) = &mode {
                    match m {
                        Mode::Gradient => {
                            current_color = ColorOrMap::Color(Color::try_from_hex_str(&buff)?);
                            buff.clear();
                        }
                        Mode::Map => {
                            let c = buff.split('#');
                            current_color =
                                ColorOrMap::Map(c.map(Color::try_from_hex_str).collect::<Result<
                                    Vec<Color>,
                                    TranError,
                                >>(
                                )?);
                        }
                    }
                } else {
                    return Err(TranError::ConfigError(
                        "Found color section before mode section. Can't determine color format"
                            .to_string(),
                    ));
                }
            }
            Section::TargetFiles => {
                target_files.push(buff);
            }
            Section::Overwrite => {
                if buff == "true" {
                    overwrite = true;
                }
            }
        }
    }

    match (
        mode.ok_or(TranError::ConfigError("Missing mode".to_string()))?,
        current_color,
        colors.ok_or(TranError::ConfigError("Missing colors".to_string()))?,
    ) {
        (Mode::Gradient, ColorOrMap::Color(current_color), ColorOrMapVec::Color(colors)) => {
            Ok(Config::GradientConfig(GradientConfig {
                current_color,
                target_files,
                colors,
                weights,
                overwrite,
            }))
        }
        (Mode::Map, ColorOrMap::Map(current_color), ColorOrMapVec::Map(colors)) => {
            Ok(Config::MapConfig(MapConfig {
                current_color,
                target_files,
                colors,
                overwrite,
                weights,
            }))
        }
        (_, _, _) => Err(TranError::ConfigError("Inconsistent state".to_string())),
    }
}

pub fn write_config<T: AsRef<Path>>(config: Config, target: T) -> Result<(), TranError> {
    let f = std::fs::File::create(target)?;
    let mut writer = std::io::BufWriter::new(f);

    match config {
        Config::GradientConfig(config) => {
            writeln!(&mut writer, "[mode]")?;
            writeln!(&mut writer, "gradient")?;

            writeln!(&mut writer, "[overwrite]")?;
            writeln!(&mut writer, "{}", config.get_overwrite())?;

            writeln!(&mut writer, "[current_color]")?;
            writeln!(&mut writer, "{}", config.get_current_color())?;

            writeln!(&mut writer, "[colors]")?;
            for color in config.get_colors() {
                writeln!(&mut writer, "{}", color)?;
            }

            writeln!(&mut writer, "[target_files]")?;
            for target in config.get_target_files() {
                writeln!(&mut writer, "{}", target)?;
            }
        }
        Config::MapConfig(config) => {
            writeln!(&mut writer, "[mode]")?;
            writeln!(&mut writer, "gradient")?;

            writeln!(&mut writer, "[overwrite]")?;
            writeln!(&mut writer, "{}", config.get_overwrite())?;

            writeln!(&mut writer, "[current_color]")?;
            for color in config.get_current_colors() {
                write!(&mut writer, "{}", color)?;
            }
            writeln!(&mut writer)?;

            writeln!(&mut writer, "[colors]")?;
            for color_row in config.get_colors() {
                for color in color_row {
                    write!(&mut writer, "{}", color)?;
                }
                writeln!(&mut writer)?;
            }

            writeln!(&mut writer, "[target_files]")?;
            for target in config.get_target_files() {
                writeln!(&mut writer, "{}", target)?;
            }
        }
    }

    writer.flush()?;

    Ok(())
}
