use std::{fs::File, io::Read};

use anyhow::{Ok, Result};
use hound::{WavReader, WavWriter};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferDescriptor, BufferUsages, Device,
};
use wgpu::{CommandEncoder, Maintain, MapMode};

use crate::GraphicsContext;

const OUTPUT_BUFFER_SIZE: usize = 512;
const SAMPLE_RATE: u32 = 16_000;

pub struct Audio {
    pub audio_in_buffer: Buffer,
    pub audio_in_len: usize,

    pub audio_out_buffer: Buffer,
    audio_writer: WavWriter<File>,

    staging_buffer: Buffer,
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

        let buf_size = OUTPUT_BUFFER_SIZE as u64 * 4;
        let audio_out_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: buf_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: buf_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Ok(Self {
            audio_in_buffer,
            audio_in_len: audio_in.len(),

            audio_out_buffer,
            audio_writer,

            staging_buffer,
        })
    }

    pub fn tick(&mut self, tick: u64, gc: &GraphicsContext, encoder: &mut CommandEncoder) {
        if tick > 0 && tick as usize % OUTPUT_BUFFER_SIZE == OUTPUT_BUFFER_SIZE - 1 {
            encoder.copy_buffer_to_buffer(
                &self.audio_out_buffer,
                0,
                &self.staging_buffer,
                0,
                512 * 4,
            );

            let slice = self.staging_buffer.slice(..);
            let (tx, rx) = crossbeam_channel::bounded(1);
            slice.map_async(MapMode::Read, move |_| tx.send(()).unwrap());

            gc.device.poll(Maintain::Wait);

            rx.recv().unwrap();
            let mapped = slice.get_mapped_range();
            let data = bytemuck::cast_slice::<_, f32>(&mapped);
            for sample in data {
                self.audio_writer
                    .write_sample((1.0 - (-sample.abs()).exp()).copysign(*sample))
                    .unwrap();
            }
            drop(mapped);
            self.staging_buffer.unmap();
        }
    }
}
