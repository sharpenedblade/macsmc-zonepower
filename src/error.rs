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
}
