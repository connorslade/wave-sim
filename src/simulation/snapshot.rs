use wgpu::{Buffer, CommandEncoder};

use super::Simulation;

pub enum SnapshotType {
    None,
    State,
    Energy,
}

impl SnapshotType {
    pub fn name(&self) -> &'static str {
        match self {
            SnapshotType::None => "none",
            SnapshotType::State => "state",
            SnapshotType::Energy => "energy",
        }
    }

    pub fn stage<'a>(
        &self,
        simulation: &'a Simulation,
        encoder: &mut CommandEncoder,
    ) -> Option<&'a Buffer> {
        Some(match self {
            SnapshotType::State => simulation.stage_state(encoder),
            SnapshotType::Energy => simulation.stage_energy(encoder),
            SnapshotType::None => return None,
        })
    }
}

impl Simulation {
    fn stage_state(&self, encoder: &mut CommandEncoder) -> &Buffer {
        let offset = (self.parameters.tick % 3) * (self.size.x * self.size.y * 4) as u64;
        encoder.copy_buffer_to_buffer(
            &self.states,
            offset,
            &self.staging_buffer,
            0,
            self.staging_buffer.size(),
        );
        &self.staging_buffer
    }
}
