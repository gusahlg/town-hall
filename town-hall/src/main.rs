use anyhow::Result;
use neuro_ash::Gpu;
use town_hall::Village;

fn main() -> Result<()> {
    let gpu = Gpu::new().unwrap();
    println!("Device: {}", gpu.device_name());

    let _village = Village::new();
    Ok(())
}
