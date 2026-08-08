use anyhow::Result;
use neuro_ash::{DEFAULT_MAX_GPU_BATCH_SIZE, Gpu};
use town_hall::Village;

const DEFAULT_ITERATIONS: u128 = 100;
const GPU_BATCH_SIZE_ENV: &str = "TOWN_HALL_GPU_BATCH_SIZE";

fn main() -> Result<()> {
    let iterations = std::env::args()
        .nth(1)
        .map(|value| value.parse::<u128>())
        .transpose()?
        .unwrap_or(DEFAULT_ITERATIONS);
    let gpu_batch_size = std::env::var(GPU_BATCH_SIZE_ENV)
        .ok()
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(DEFAULT_MAX_GPU_BATCH_SIZE);
    let mut village = Village::new();
    let mut gpu = Gpu::with_max_batch_size(gpu_batch_size)?;
    println!(
        "Device: {} (max GPU batch: {})",
        gpu.device_name(),
        gpu.max_batch_size()
    );

    while village.iterations() < iterations {
        let report = village.step(&mut gpu)?;
        println!(
            "iteration {:>3}: entities={}, deaths={}, learned_slots={}, consumed={}, entity_resources={}, world_resources={}",
            report.iteration,
            village.entities().len(),
            report.deaths.len(),
            report.replaced_slots,
            report.consumed_resources,
            village.total_entity_resources(),
            village.resources()
        );
    }

    Ok(())
}
