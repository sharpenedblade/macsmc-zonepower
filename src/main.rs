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
    println!("Read initial SMC power limits");
    let mut cpu_policies = CpuPolicy::open_all().unwrap();
    println!("Found cpufreq policies");
    let mut idle_cpu = CpuIdle::open().unwrap();
    println!("Found idle cpu time device");
    let mut igpu = Igpu::open().unwrap();
    println!("Found igpu device");
    apply_plimts(&plimits, &mut cpu_policies, &mut igpu, &mut idle_cpu).unwrap();
    println!("Applied initial power limits");
    loop {
        let new_plimts = SmcPowerCaps::read().unwrap();
        if new_plimts != plimits {
            plimits = new_plimts;
            apply_plimts(&plimits, &mut cpu_policies, &mut igpu, &mut idle_cpu).unwrap();
        }
        sleep(time::Duration::from_millis(200));
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
