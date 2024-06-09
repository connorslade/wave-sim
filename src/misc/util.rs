use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::Result;
use wgpu::{Buffer, MaintainBase, MapMode};

use crate::GraphicsContext;

pub fn save_dated_file(base: impl AsRef<Path>, name: &str, ext: &str) -> Result<PathBuf> {
    let base = base.as_ref();

    if !base.exists() {
        fs::create_dir_all(base)?;
    }

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for i in 0.. {
        let name = if i == 0 {
            format!("{name}-{timestamp}.{ext}")
        } else {
            format!("{name}-{timestamp}-{i}.{ext}")
        };

        let path = base.join(name);
        if !path.exists() {
            return Ok(path);
        }
    }

    unreachable!()
}

pub fn download_buffer(buffer: &Buffer, gc: &GraphicsContext) -> Vec<u8> {
    let slice = buffer.slice(..);

    let (tx, rx) = crossbeam_channel::bounded(1);
    slice.map_async(MapMode::Read, move |_| tx.send(()).unwrap());

    gc.device.poll(MaintainBase::Wait);
    rx.recv().unwrap();

    let data = slice.get_mapped_range().to_vec();
    buffer.unmap();

    data
}
