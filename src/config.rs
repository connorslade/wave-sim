use std::{fs, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    /// The path that shader and map files are relative to.
    pub base_path: Option<PathBuf>,

    /// The size of the simulation.
    pub size: (u32, u32),
    /// Reflective Boundaries
    pub reflective_boundary: bool,

    /// The path to the shader file.
    pub shader: Option<PathBuf>,

    /// The path to an image file to use as a map.
    /// The red channel represents walls, green represents emitters, and blue represents change in c (128 is no change).
    /// Should be a lossless format like PNG.
    pub map: Option<PathBuf>,

    /// Time step (ms).
    pub dt: f32,
    /// Space step (mm).
    pub dx: f32,

    /// Wave velocity m/s.
    pub v: f32,
    /// Initial oscillator amplitude.
    pub amplitude: f32,
    /// Initial oscillator frequency in Hz.
    pub frequency: f32,

    /// Audio configuration.
    pub audio: Option<AudioConfig>,
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
