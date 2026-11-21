use crate::error::Error;

const PLIMIT_PATH: &str = "/sys/bus/platform/devices/APP0001:00/plimit";

pub mod power_cap {
    #[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
    pub struct PowerCap(u8);
    impl PowerCap {
        pub fn new(n: u8) -> Self {
            // the most significant bit represents the hard limit flag
            PowerCap(n & !(1 << 7))
        }
        pub fn disabled() -> Self {
            PowerCap(0)
        }
        #[inline(always)]
        pub fn as_u8(&self) -> u8 {
            self.0
        }
    }
}
use power_cap::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmcPowerCaps {
    pub idle: PowerCap,
    pub ram: PowerCap,
    pub igpu: PowerCap,
    pub cpu: PowerCap,
    pub dgpu: Option<PowerCap>,
}

impl SmcPowerCaps {
    pub fn read() -> Result<Self, Error> {
        let plimits_file = std::fs::read_to_string(PLIMIT_PATH).map_err(Error::PowerLimitRead)?;
        Self::parse(plimits_file.trim())
    }

    fn parse(plimits_file: &str) -> Result<Self, Error> {
        let (limits, dgpu_limit) = plimits_file.split_once(' ').ok_or(Error::PowerLimitParse)?;
        let dgpu = u8::from_str_radix(dgpu_limit, 16).map_err(|_| Error::PowerLimitParse)?;
        let [cpu, igpu, ram, idle] = u32::from_str_radix(limits, 16)
            .map(|i| i.to_le_bytes())
            .map_err(|_| Error::PowerLimitParse)?;

        Ok(Self {
            cpu: PowerCap::new(cpu),
            igpu: PowerCap::new(igpu),
            dgpu: (dgpu != 0).then(|| PowerCap::new(dgpu)),
            idle: PowerCap::new(idle),
            ram: PowerCap::new(ram),
        })
    }
    pub fn disabled() -> Self {
        Self {
            idle: PowerCap::disabled(),
            ram: PowerCap::disabled(),
            igpu: PowerCap::disabled(),
            cpu: PowerCap::disabled(),
            dgpu: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_plimits() {
        assert_eq!(
            SmcPowerCaps::parse("A1dead04 00").unwrap(),
            SmcPowerCaps {
                idle: PowerCap::new(0xA1),
                ram: PowerCap::new(0xde),
                igpu: PowerCap::new(0xad),
                cpu: PowerCap::new(4),
                dgpu: None,
            }
        );
    }
    #[test]
    fn ignore_plimit_hard_limit() {
        assert_eq!(SmcPowerCaps::parse("000000ad 00").unwrap().cpu.as_u8(), 45);
    }
    #[test]
    fn zero_dgpu_plimit_is_none() {
        assert_eq!(SmcPowerCaps::parse("01020304 00").unwrap().dgpu, None);
    }
    #[test]
    fn parses_dgpu_plimit() {
        assert_eq!(
            SmcPowerCaps::parse("01020304 0f")
                .unwrap()
                .dgpu
                .map(|p| p.as_u8()),
            Some(15)
        );
    }
}
