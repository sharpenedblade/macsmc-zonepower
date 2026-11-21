use crate::{
    cpu::CpuPolicy,
    error::Error,
    idle_cpu::CpuIdle,
    igpu::Igpu,
    plimits::SmcPowerCaps,
};
use nonempty::NonEmpty;
use signal_hook::consts::{SIGINT, SIGTERM};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::sleep,
    time,
};

mod cpu;
mod error;
mod idle_cpu;
mod igpu;
mod plimits;

fn main() {
    let cancellation_token = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(SIGINT, cancellation_token.clone()).unwrap();
    signal_hook::flag::register(SIGTERM, cancellation_token.clone()).unwrap();
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
    while !cancellation_token.load(Ordering::Relaxed) {
        let new_plimts = SmcPowerCaps::read().unwrap();
        if new_plimts != plimits {
            plimits = new_plimts;
            apply_plimts(&plimits, &mut cpu_policies, &mut igpu, &mut idle_cpu).unwrap();
        }
        sleep(time::Duration::from_millis(200));
    }
    println!("Disabling power limits");
    apply_plimts(
        &SmcPowerCaps::disabled(),
        &mut cpu_policies,
        &mut igpu,
        &mut idle_cpu,
    )
    .unwrap();
    println!("Exited");
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
