use crate::{
    cpu::CpuPolicy, error::Error, idle_cpu::CpuIdle, igpu::Igpu, smc_power_caps::SmcPowerCaps,
};
use nonempty::NonEmpty;
use std::{thread::sleep, time};

mod cpu;
mod error;
mod idle_cpu;
mod igpu;
mod smc_power_caps;

fn main() {
    let mut plimits = SmcPowerCaps::read().unwrap();
    let mut cpu_policies = CpuPolicy::open_all().unwrap();
    let mut igpu = Igpu::open().unwrap();
    let mut idle_cpu = CpuIdle::open().unwrap();
    apply_plimts(&plimits, &mut cpu_policies, &mut igpu, &mut idle_cpu).unwrap();
    loop {
        let new_plimts = SmcPowerCaps::read().unwrap();
        if new_plimts != plimits {
            plimits = new_plimts;
            println!("{plimits:?}");
            apply_plimts(&plimits, &mut cpu_policies, &mut igpu, &mut idle_cpu).unwrap();
        }
        sleep(time::Duration::from_millis(250));
    }
}

fn apply_plimts(
    plimits: &SmcPowerCaps,
    cpu_policies: &mut NonEmpty<CpuPolicy>,
    igpu: &mut Igpu,
    idle: &mut CpuIdle,
) -> Result<(), Error> {
    for cpu in cpu_policies.iter_mut() {
        cpu.cap_power(plimits.cpu)?;
    }
    igpu.cap_power(plimits.igpu)?;
    idle.cap_power(plimits.idle)?;
    Ok(())
}
