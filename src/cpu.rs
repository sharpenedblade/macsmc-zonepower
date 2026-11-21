use crate::{error::Error, plimits::power_cap::PowerCap};
use glob::glob;
use nonempty::NonEmpty;
use std::{fs::File, io::Write, path::PathBuf};

const CPU_POLICY_GLOB: &str = "/sys/devices/system/cpu/cpufreq/policy*";

pub struct CpuPolicy {
    scaling_max_freq: File,
    /// minimum cpu frequency in KHz
    min_freq: u32,
    /// maximum cpu frequency in KHz
    max_freq: u32,
}
impl CpuPolicy {
    fn new(path: PathBuf) -> Result<Self, Error> {
        let max_freq = std::fs::read_to_string(path.join("cpuinfo_max_freq"))
            .map_err(Error::CpuFreqLimitRead)?
            .trim()
            .parse()
            .map_err(|_| Error::CpuFreqLimitParse)?;
        let min_freq = std::fs::read_to_string(path.join("cpuinfo_min_freq"))
            .map_err(Error::CpuFreqLimitRead)?
            .trim()
            .parse()
            .map_err(|_| Error::CpuFreqLimitParse)?;
        let scaling_max_freq = std::fs::OpenOptions::new()
            .write(true)
            .open(path.join("scaling_max_freq"))
            .map_err(Error::CpuScalingLimitOpen)?;
        Ok(Self {
            scaling_max_freq,
            min_freq,
            max_freq,
        })
    }
    pub fn cap_power(&mut self, plimit: PowerCap) -> Result<(), Error> {
        let target_frequency =
            (self.max_freq - (plimit.as_u8() as u32) * 100 * 1000).max(self.min_freq);
        self.scaling_max_freq
            .write_all(target_frequency.to_string().as_bytes())
            .map_err(Error::CpuScalingLimitWrite)?;
        Ok(())
    }
    pub fn open_all() -> Result<NonEmpty<CpuPolicy>, Error> {
        let policies = glob(CPU_POLICY_GLOB)
            .unwrap()
            .filter_map(|p| p.ok())
            .map(CpuPolicy::new)
            .collect::<Result<Vec<_>, Error>>()?;
        NonEmpty::collect(policies).ok_or(Error::NoCpufreqPolicies)
    }
}
