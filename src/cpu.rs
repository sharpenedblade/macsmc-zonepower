use crate::{error::Error, plimits::power_cap::PowerCap};
use glob::glob;
use nonempty::NonEmpty;
use std::{fs::File, io::Write, path::PathBuf};

const CPU_POLICY_GLOB: &str = "/sys/devices/system/cpu/cpufreq/policy*";
/// 100 MHz in KHz
const STEP_SIZE: u32 = 1000 * 100;

pub struct CpuPolicy {
    scaling_max_freq: File,
    /// minimum cpu frequency in KHz
    min_freq: u32,
    /// boost  frequency in KHz
    max_freq: u32,
    /// base frequency in KHz
    base_freq: u32,
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
        let base_freq = std::fs::read_to_string(path.join("base_frequency"))
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
            base_freq,
        })
    }
    pub fn cap_power(&mut self, plimit: PowerCap) -> Result<(), Error> {
        let target_frequency =
            (self.max_freq - (plimit.as_u8() as u32) * STEP_SIZE).max(self.min_freq);
        self.scaling_max_freq
            .write_all(target_frequency.to_string().as_bytes())
            .map_err(Error::CpuScalingLimitWrite)?;
        Ok(())
    }
    /// Number of scaling steps at or below the base frequency
    pub fn base_step_count(&self) -> u32 {
        (self.base_freq - self.min_freq) / STEP_SIZE + 1
    }
    /// Number of scaling steps above the base frequency
    pub fn boost_step_count(&self) -> u32 {
        (self.max_freq - self.base_freq) / STEP_SIZE
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
