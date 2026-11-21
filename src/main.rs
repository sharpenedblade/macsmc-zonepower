use crate::{
    cpu::CpuPolicy,
    error::Error,
    idle_cpu::CpuIdle,
    igpu::Igpu,
    plimits::SmcPowerCaps,
    smc::{Smc, SmcKey},
};
use nonempty::NonEmpty;
use signal_hook::consts::{SIGINT, SIGTERM};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::sleep,
    time::{self},
};

mod cpu;
mod error;
mod idle_cpu;
mod igpu;
mod plimits;
mod smc;

fn main() {
    let cancellation_token = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(SIGINT, cancellation_token.clone()).unwrap();
    signal_hook::flag::register(SIGTERM, cancellation_token.clone()).unwrap();
    let mut cpu_policies = CpuPolicy::open_all().unwrap();
    println!("Found cpufreq policies");
    let mut idle_cpu = CpuIdle::open().unwrap();
    println!("Found idle cpu time device");
    let mut igpu = Igpu::open().unwrap();
    println!("Found igpu device");
    let mut smc = Smc::open().unwrap();
    println!("Found smc device");
    send_steps_to_smc(&mut smc, &cpu_policies, &igpu).unwrap();
    println!("Sent scaling step counts to smc");
    let mut plimits = SmcPowerCaps::read().unwrap();
    println!("Read initial SMC power limits");
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

fn send_steps_to_smc(
    smc: &mut Smc,
    cpu_policies: &NonEmpty<CpuPolicy>,
    igpu: &Igpu,
) -> Result<(), Error> {
    let igpu_key = if smc.exists(SmcKey::Dgpu)? {
        SmcKey::IgpuOrDgpu
    } else {
        SmcKey::Igpu
    };
    smc.write(
        SmcKey::CpuBase,
        cpu_policies.first().base_step_count().clamp(1, 127) as u8,
    )?;
    smc.write(
        SmcKey::CpuBoost,
        cpu_policies.first().boost_step_count().clamp(1, 127) as u8,
    )?;
    smc.write(igpu_key, igpu.step_count().clamp(1, 127) as u8)?;
    Ok(())
}
