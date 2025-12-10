use crate::terrain::field_compute::*;

/// Constructs the mesh given the data from the work done in noise_field

pub fn recieve(trigger: On<ReadbackComplete>) {
    info!(
        "Received data from noise_field from entity {}",
        trigger.entity
    );
}
