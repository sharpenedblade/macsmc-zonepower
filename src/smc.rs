use crate::error::Error;

mod raw {
    use std::{fs::File, os::fd::AsRawFd};

    #[repr(C)]
    pub struct ioctl_smc_rw_payload {
        /// 4 characters packed into a big endian integer
        key: u32,
        /// Size of data in buf in bytes
        size: u32,
        /// SAFETY: Do not acess data after size
        buf: [u8; 32],
    }
    impl ioctl_smc_rw_payload {
        pub fn new(key: u32, size: u32, value: &[u8]) -> Self {
            assert!(size <= 32 && value.len() <= size as usize);
            let mut buf = [0; 32];
            buf[..value.len()].copy_from_slice(value);
            Self { key, size, buf }
        }
        pub fn value(&self) -> &[u8] {
            &self.buf[..self.size as usize]
        }
        nix::ioctl_readwrite!(smc_read, b'S', 1, ioctl_smc_rw_payload);
        nix::ioctl_write_ptr!(smc_write, b'S', 2, ioctl_smc_rw_payload);
    }

    #[repr(C, align(4))]
    pub struct ioctl_smc_info_payload {
        /// 4 characters packed into a big endian integer
        key: u32,
        type_code: u32,
        /// size of data in key
        pub size: u8,
        flags: u8,
    }
    impl ioctl_smc_info_payload {
        fn new(key: u32) -> Self {
            Self {
                key,
                type_code: 0,
                size: 0,
                flags: 0,
            }
        }
        nix::ioctl_readwrite!(smc_get_key_info, b'S', 4, ioctl_smc_info_payload);
    }

    pub struct RawSmc(File);

    impl RawSmc {
        pub fn open() -> Result<Self, std::io::Error> {
            Ok(Self(
                std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open("/dev/smc")?,
            ))
        }
        pub fn read(&mut self, key: u32, size: u32) -> Result<ioctl_smc_rw_payload, nix::Error> {
            let mut buf = ioctl_smc_rw_payload::new(key, size, &[]);
            // SAFETY: self.0 is /dev/smc and buf is an exclusive reference
            let _res = unsafe { ioctl_smc_rw_payload::smc_read(self.0.as_raw_fd(), &mut buf) }?;
            Ok(buf)
        }
        pub fn write(&mut self, payload: ioctl_smc_rw_payload) -> Result<(), nix::Error> {
            // SAFETY: self.0 is /dev/smc
            let _res = unsafe { ioctl_smc_rw_payload::smc_write(self.0.as_raw_fd(), &payload) }?;
            Ok(())
        }
        pub fn info(&self, key: u32) -> Result<ioctl_smc_info_payload, nix::Error> {
            let mut buf = ioctl_smc_info_payload::new(key);
            // SAFETY: self.0 is /dev/smc
            let _res =
                unsafe { ioctl_smc_info_payload::smc_get_key_info(self.0.as_raw_fd(), &mut buf) }?;
            Ok(buf)
        }
    }
}

#[derive(Clone, Copy)]
pub enum SmcKey {
    /// iGPU scaling steps, when there is no HE2N
    Igpu,
    /// iGPU scaling steps where HE2N exists, otherwise dGPU scaling steps
    IgpuOrDgpu,
    /// dGPU scaling steps if this exists
    Dgpu,
    /// CPU scaling steps below base
    CpuBase,
    /// CPU scaling steps above base
    CpuBoost,
}

impl SmcKey {
    fn key(&self) -> u32 {
        match self {
            Self::Igpu => u32::from_be_bytes(*b"HI0N"),
            Self::IgpuOrDgpu => u32::from_be_bytes(*b"HE0N"),
            Self::Dgpu => u32::from_be_bytes(*b"HE2N"),
            Self::CpuBase => u32::from_be_bytes(*b"MSPC"),
            Self::CpuBoost => u32::from_be_bytes(*b"MSTS"),
        }
    }
}

pub struct Smc {
    raw: raw::RawSmc,
}

impl Smc {
    pub fn open() -> Result<Self, Error> {
        let raw = raw::RawSmc::open().map_err(Error::SmcDeviceOpen)?;
        Ok(Self { raw })
    }
    pub fn exists(&mut self, key: SmcKey) -> Result<bool, Error> {
        match self.raw.info(key.key()).map(|i| i.size != 0) {
            Ok(b) => Ok(b),
            Err(nix::Error::EIO) => Ok(false),
            Err(err) => Err(Error::SmcRead(err)),
        }
    }
    fn size_of_key(&mut self, key: SmcKey) -> Result<u32, Error> {
        self.raw
            .info(key.key())
            .map_err(Error::SmcRead)
            .map(|info| info.size as u32)
    }
    #[allow(dead_code)]
    pub fn read(&mut self, key: SmcKey) -> Result<u8, Error> {
        let size = self.size_of_key(key)?;
        self.raw
            .read(key.key(), size)
            .map(|payload| payload.value()[0])
            .map_err(Error::SmcRead)
    }
    /// val must be in 0..=128
    pub fn write(&mut self, key: SmcKey, val: u8) -> Result<(), Error> {
        assert!((1..=127).contains(&val));
        let size = self.size_of_key(key)?;
        let payload = raw::ioctl_smc_rw_payload::new(key.key(), size, &[val]);
        self.raw.write(payload).map_err(Error::SmcWrite)
    }
}
