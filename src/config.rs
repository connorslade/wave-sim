use std::{fs, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    /// The path that shader and map files are relative to.
    pub base_path: Option<PathBuf>,

    /// The size of the simulation.
    pub size: (u32, u32),
    /// Basic simulation parameters.
    pub parameters: Parameters,
    /// Oscillator parameters.
    pub oscillator: Oscillator,

    /// The path to the shader file.
    pub shader: Option<PathBuf>,
    /// The path to an optional rhai script
    pub script: Option<PathBuf>,
    /// The path to an image file to use as a map.
    /// The red channel represents walls, green represents emitters, and blue represents change in c (128 is no change).
    /// Should be a lossless format like PNG.
    pub map: Option<PathBuf>,

    /// Audio configuration.
    pub audio: Option<AudioConfig>,
}

#[derive(Deserialize, Debug)]
pub struct Parameters {
    /// Time step (ms).
    pub dt: f32,
    /// Space step (mm).
    pub dx: f32,
    /// Wave speed m/s.
    pub v: f32,

    /// Reflective Boundaries
    pub reflective_boundary: bool,
}

#[derive(Deserialize, Debug)]
pub struct Oscillator {
    /// Initial oscillator amplitude.
    pub amplitude: f32,
    /// Initial oscillator frequency in Hz.
    pub frequency: f32,
}

#[derive(Deserialize, Debug)]
pub struct AudioConfig {
    pub input: PathBuf,
    pub output: PathBuf,
    pub pickup: (u32, u32),
}

#[derive(Parser)]
#[clap(name = "wave-sim", version = "0.1.0", author = "Connor Slade")]
pub struct Args {
    /// Path to a configuration file. (params.toml)
    config: PathBuf,
}

impl Config {
    pub fn base_path(&self) -> PathBuf {
        self.base_path.clone().unwrap_or_default()
    }
}

pub fn parse() -> Result<Config> {
    let args = Args::parse();

    let raw_config = fs::read_to_string(&args.config)?;
    let mut config = toml::from_str::<Config>(&raw_config)?;

    if config.base_path.is_none() {
        config.base_path = Some(args.config.parent().unwrap().to_path_buf());
    }

    Ok(config)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_path: None,
            size: (1920, 1080),
            parameters: Default::default(),
            oscillator: Default::default(),
            shader: None,
            map: None,
            script: None,
            audio: None,
        }
    }
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            dt: 1.16e-14,
            dx: 5e-6,
            v: 299_792_458.0,
            reflective_boundary: false,
        }
    }
}

impl Default for Oscillator {
    fn default() -> Self {
        Self {
            amplitude: 5e-2,
            frequency: 4.3e14,
        }
    }
}
