use std::io::{Seek, Write};
use std::{fs::File, io::Read};

use anyhow::{Ok, Result};
use hound::{WavReader, WavWriter};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferDescriptor, BufferUsages, Device,
};

use crate::GraphicsContext;

const OUTPUT_BUFFER_SIZE: usize = 512;
const SAMPLE_RATE: u32 = 16_000;

trait WriteSeek: Write + Seek {}

pub struct Audio {
    audio_in_buffer: Buffer,
    audio_out_buffer: Buffer,
    audio_in_length: usize,
    staging_buffer: Buffer,
    audio_writer: WavWriter<File>,
}

impl Audio {
    pub fn new(device: &Device, wav_in: impl Read, wav_out: File) -> Result<Self> {
        let audio_in = WavReader::new(wav_in)?
            .samples::<f32>()
            .collect::<Result<Vec<_>, hound::Error>>()?;

        let audio_writer = WavWriter::new(
            wav_out,
            hound::WavSpec {
                channels: 1,
                sample_rate: SAMPLE_RATE,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();

        let audio_in_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&audio_in),
            usage: BufferUsages::STORAGE,
        });

        let audio_out_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: 512 * 4, // 512 samples * 4 bytes per sample
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: 512 * 4,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Ok(Self {
            audio_in_buffer,
            audio_out_buffer,
            audio_in_length: audio_in.len(),
            staging_buffer,
            audio_writer,
        })
    }

    pub fn tick(&mut self, tick: usize, gc: &GraphicsContext) {
        if tick > 0 && tick % OUTPUT_BUFFER_SIZE == OUTPUT_BUFFER_SIZE - 1 {}
    }
}
