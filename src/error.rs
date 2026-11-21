use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot read smc power limits")]
    PowerLimitRead(#[source] io::Error),
    #[error("could not parse smc power limits")]
    PowerLimitParse,
    #[error("could not get cpu frequency limits")]
    CpuFreqLimitRead(#[source] io::Error),
    #[error("could not parse cpu frequency limits")]
    CpuFreqLimitParse,
    #[error("could not open cpu scaling limit")]
    CpuScalingLimitOpen(#[source] io::Error),
    #[error("could not write cpu scaling limit")]
    CpuScalingLimitWrite(#[source] io::Error),
    #[error("no cpu frequency policies found")]
    NoCpufreqPolicies,
    #[error("no intel igpu found")]
    NoIgpu,
    #[error("no intel igpu found")]
    IgpuDeviceDirOpen,
    #[error("could not get igpu frequency limits")]
    IgpuLimitRead(#[source] io::Error),
    #[error("could not parse igpu frequency limits")]
    IgpuLimitParse,
    #[error("could not open igpu max frequency")]
    IgpuMaxFreqOpen,
    #[error("could not write igpu max frequency")]
    IgpuMaxFreqWrite,
    #[error("no intel powerclamp device found")]
    NoIntelPowerclamp,
    #[error("could not read cooling devices")]
    FindCoolingDevice,
    #[error("could not read intel_powerclamp module parameter")]
    IntelPowerclampSysfsRead,
    #[error("could not open intel powerclamp device")]
    IdleCpuOpen(#[source] io::Error),
    #[error("could not write idle cpu time")]
    IdleCpuWrite(#[source] io::Error),
    #[error("could not open smc device")]
    SmcDeviceOpen(#[source] io::Error),
    #[error("could not read key from smc")]
    SmcRead(#[source] nix::Error),
    #[error("could not write key to smc")]
    SmcWrite(#[source] nix::Error),
}
