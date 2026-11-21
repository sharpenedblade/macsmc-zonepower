use crate::{error::Error, plimits::power_cap::PowerCap};
use std::{fs::File, io::Write};

pub struct CpuIdle {
    /// maximum percent of cpu time forced idle
    max_idle_time: u32,
    /// percent of idle cpu time
    idle_time: File,
}

impl CpuIdle {
    pub fn open() -> Result<Self, Error> {
        for device in glob::glob("/sys/class/thermal/cooling_device*")
            .unwrap()
            .filter_map(Result::ok)
        {
            if std::fs::read_to_string(device.join("type"))
                .map_err(|_| Error::FindCoolingDevice)?
                .trim()
                == "intel_powerclamp"
            {
                let max_idle_time =
                    std::fs::read_to_string("/sys/module/intel_powerclamp/parameters/max_idle")
                        .map_err(|_| Error::IntelPowerclampSysfsRead)?
                        .trim()
                        .parse()
                        .map_err(|_| Error::IntelPowerclampSysfsRead)?;
                let idle_time = std::fs::OpenOptions::new()
                    .write(true)
                    .open(device.join("cur_state"))
                    .map_err(Error::IdleCpuOpen)?;
                return Ok(Self {
                    max_idle_time,
                    idle_time,
                });
            }
        }
        Err(Error::NoIntelPowerclamp)
    }
    pub fn cap_power(&mut self, plimit: PowerCap) -> Result<(), Error> {
        let idle_time = if plimit.as_u8() != 0 {
            (100 - plimit.as_u8() as u32).min(self.max_idle_time)
        } else {
            0
        };
        self.idle_time
            .write_all(idle_time.to_string().as_bytes())
            .map_err(Error::IdleCpuWrite)?;
        Ok(())
    }
}
