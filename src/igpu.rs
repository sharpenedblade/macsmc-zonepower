use crate::{error::Error, plimits::power_cap::PowerCap};
use std::{fs::File, io::Write, path::Path};

const DRM_CARD_GLOB: &str = "/sys/class/drm/card*";
const INTEL_VENDOR_STRING: &str = "0x8086";
/// in MHz
const STEP_SIZE: u32 = 50;

pub struct Igpu {
    /// Maximum frequency in MHz
    rp0_freq: u32,
    /// Minimum frequency in MHz
    rpn_freq: u32,
    /// Suggested maximum frequency in MHz
    max_freq: File,
    /// Maximum frequency in MHz
    boost_freq: File,
}

impl Igpu {
    pub fn open() -> Result<Self, Error> {
        for dir in glob::glob(DRM_CARD_GLOB).unwrap() {
            let dir = dir.map_err(|_| Error::IgpuDeviceDirOpen)?;
            if let Ok(vendor) = std::fs::read_to_string(dir.join("device/vendor"))
                && vendor.trim() == INTEL_VENDOR_STRING
            {
                return Self::new(dir.as_path());
            }
        }
        Err(Error::NoIgpu)
    }
    fn new(p: &Path) -> Result<Self, Error> {
        let rp0_freq = std::fs::read_to_string(p.join("gt_RP0_freq_mhz"))
            .map_err(Error::IgpuLimitRead)?
            .trim()
            .parse()
            .map_err(|_| Error::IgpuLimitParse)?;
        let rpn_freq = std::fs::read_to_string(p.join("gt_RPn_freq_mhz"))
            .map_err(Error::IgpuLimitRead)?
            .trim()
            .parse()
            .map_err(|_| Error::IgpuLimitParse)?;
        let max_freq = std::fs::OpenOptions::new()
            .write(true)
            .open(p.join("gt_max_freq_mhz"))
            .map_err(|_| Error::IgpuMaxFreqOpen)?;
        let boost_freq = std::fs::OpenOptions::new()
            .write(true)
            .open(p.join("gt_boost_freq_mhz"))
            .map_err(|_| Error::IgpuMaxFreqOpen)?;
        Ok(Self {
            rp0_freq,
            rpn_freq,
            max_freq,
            boost_freq,
        })
    }
    pub fn cap_power(&mut self, plimit: PowerCap) -> Result<(), Error> {
        let freq = (self.rp0_freq - plimit.as_u8() as u32 * STEP_SIZE)
            .max(self.rpn_freq)
            .to_string();
        self.max_freq
            .write_all(freq.as_bytes())
            .map_err(|_| Error::IgpuMaxFreqWrite)?;
        self.boost_freq
            .write_all(freq.as_bytes())
            .map_err(|_| Error::IgpuMaxFreqWrite)?;

        Ok(())
    }
    pub fn step_count(&self) -> u32 {
        (self.rp0_freq - self.rpn_freq) / STEP_SIZE + 1
    }
}
