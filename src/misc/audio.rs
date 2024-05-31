use std::{fs::File, io::Read};

use anyhow::{Ok, Result};
use hound::{SampleFormat, WavReader, WavWriter};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferDescriptor, BufferUsages, CommandEncoder, Device, Maintain, MapMode,
};

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
        let mut audio_in_reader = WavReader::new(wav_in)?;
        let audio_in_spec = audio_in_reader.spec();
        let audio_in = match audio_in_spec.sample_format {
            SampleFormat::Float => audio_in_reader
                .samples::<f32>()
                .collect::<Result<Vec<_>, hound::Error>>(),
            SampleFormat::Int => {
                //todo: test
                let denominator = (1u32 << audio_in_spec.bits_per_sample - 1) as f32;
                audio_in_reader
                    .samples::<i32>()
                    .map(|x| x.map(|x| x as f32 / denominator))
                    .collect::<Result<Vec<_>, hound::Error>>()
            }
        }?;

        // let params = SincInterpolationParameters {
        //     sinc_len: 256,
        //     f_cutoff: 0.95,
        //     interpolation: SincInterpolationType::Linear,
        //     oversampling_factor: 256,
        //     window: WindowFunction::BlackmanHarris2,
        // };
        // let mut resampler = SincFixedIn::<f32>::new(
        //     SAMPLE_RATE as f64 / audio_in_spec.sample_rate as f64,
        //     2.0,
        //     params,
        //     1024,
        //     1,
        // )?;
        // let audio_in = &resampler.process(&[&audio_in], None)?[0];

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
                OUTPUT_BUFFER_SIZE as u64 * 4,
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
