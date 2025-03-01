use wgpu::{Buffer, CommandEncoder};

use super::Simulation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotType {
    State,
    Energy,
}

#[derive(Default)]
pub struct SnapshotQueue {
    snapshots: Vec<(SnapshotType, Option<String>)>,
}

impl SnapshotType {
    pub fn name(&self) -> &'static str {
        match self {
            SnapshotType::State => "state",
            SnapshotType::Energy => "energy",
        }
    }

    pub fn stage<'a>(
        &self,
        simulation: &'a Simulation,
        encoder: &mut CommandEncoder,
    ) -> &'a Buffer {
        match self {
            SnapshotType::State => simulation.stage_state(encoder),
            SnapshotType::Energy => simulation.stage_energy(encoder),
        }
    }
}

impl SnapshotQueue {
    pub fn push(&mut self, snapshot: SnapshotType, name: Option<String>) {
        self.snapshots.push((snapshot, name));
    }

    pub fn extend(&mut self, snapshots: Vec<(SnapshotType, Option<String>)>) {
        self.snapshots.extend(snapshots);
    }

    pub fn pop(&mut self) -> Option<(SnapshotType, Option<String>)> {
        self.snapshots.pop()
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
