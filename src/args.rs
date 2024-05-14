use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{builder::ValueParser, Parser};
use serde::Deserialize;

#[derive(Parser, Deserialize, Debug)]
pub struct Args {
    /// The path that shader and map files are relative to.
    #[arg(short, long)]
    pub base_path: Option<PathBuf>,

    /// The size of the simulation.
    #[arg(short, long, value_parser = ValueParser::new(parse_size))]
    pub size: (u32, u32),
    /// Reflective Boundaries
    #[arg(short, long, default_value_t = false)]
    pub reflective_boundary: bool,

    /// The path to the shader file.
    #[arg(long, alias = "sh")]
    pub shader: Option<PathBuf>,

    /// The path to an image file to use as a map.
    /// The red channel represents walls, green represents emitters, and blue represents change in c (128 is no change).
    /// Should be a lossless format like PNG.
    #[arg(long, short)]
    pub map: Option<PathBuf>,

    /// Time step (ms).
    #[arg(short = 't', long, default_value_t = 0.011)]
    pub dt: f32,
    /// Space step (mm).
    #[arg(short = 'x', long, default_value_t = 0.05)]
    pub dx: f32,

    /// Wave velocity m/s.
    #[arg(short, long, default_value_t = 340.29)]
    pub v: f32,
    /// Initial oscillator amplitude.
    #[arg(short, long, default_value_t = 0.005)]
    pub amplitude: f32,
    /// Initial oscillator frequency in Hz.
    #[arg(short, long, default_value_t = 0.04)]
    pub frequency: f32,
}

#[derive(Parser, Debug)]
pub struct MapArgs {
    pub path: PathBuf,
}

impl Args {
    pub fn base_path(&self) -> PathBuf {
        self.base_path.clone().unwrap_or_default()
    }
}

pub fn parse() -> Result<Args> {
    let raw_args = env::args().skip(1).collect::<Vec<_>>();
    if raw_args.len() == 1 {
        let path = Path::new(&raw_args[0]);

        let raw_config = fs::read_to_string(path)?;
        let mut config = toml::from_str::<Args>(&raw_config)?;

        if config.base_path.is_none() {
            config.base_path = Some(path.parent().unwrap().to_path_buf());
        }

        return Ok(config);
    }

    Ok(Args::parse())
}

fn parse_size(raw: &str) -> Result<(u32, u32)> {
    let (width, height) = raw
        .split_once('x')
        .context("Size must be in the format WIDTHxHEIGHT")?;
    Ok((width.parse()?, height.parse()?))
}
