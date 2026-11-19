use crate::{cpu::CpuPolicy, error::Error, igpu::Igpu, smc_power_caps::SmcPowerCaps};
use nonempty::NonEmpty;
use std::{thread::sleep, time};

mod cpu;
mod error;
mod igpu;
mod smc_power_caps;

fn main() {
    let mut plimits = SmcPowerCaps::read().unwrap();
    let mut cpu_policies = CpuPolicy::open_all().unwrap();
    let mut igpu = Igpu::open().unwrap();
    apply_plimts(&plimits, &mut cpu_policies, &mut igpu).unwrap();
    loop {
        let new_plimts = SmcPowerCaps::read().unwrap();
        if new_plimts != plimits {
            plimits = new_plimts;
            println!("{plimits:?}");
            apply_plimts(&plimits, &mut cpu_policies, &mut igpu).unwrap();
        }
        sleep(time::Duration::from_millis(250));
    }
}

fn apply_plimts(
    plimits: &SmcPowerCaps,
    cpu_policies: &mut NonEmpty<CpuPolicy>,
    igpu: &mut Igpu,
) -> Result<(), Error> {
    for cpu in cpu_policies.iter_mut() {
        cpu.cap_power(plimits.cpu)?;
    }
    igpu.cap_power(plimits.igpu)?;
    Ok(())
}
