use crate::cpu::UnicornCPU;
use std::fs;
use std::io;
use std::path::Path;
use thiserror::Error;

pub const DEFAULT_LOAD_ADDRESS: u64 = 0x0000_1000;
pub const MAX_HOMEBREW_SIZE: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomebrewBinary {
    pub entry_point: u64,
    pub load_address: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("failed to read binary: {0}")]
    Io(#[from] io::Error),
    #[error("binary is empty")]
    Empty,
    #[error("binary is too large: {actual} bytes > {max} bytes")]
    TooLarge { actual: usize, max: usize },
    #[error("load address overflow for {load_address:#x} + {len:#x}")]
    AddressOverflow { load_address: u64, len: usize },
    #[error("binary length must be a multiple of 4 bytes for raw AArch64 code")]
    UnalignedLength,
    #[error("failed to write binary into emulated memory at {address:#x}")]
    MemoryWrite { address: u64 },
}

impl HomebrewBinary {
    pub fn from_bytes(bytes: Vec<u8>, load_address: u64) -> Result<Self, LoaderError> {
        if bytes.is_empty() {
            return Err(LoaderError::Empty);
        }
        if bytes.len() > MAX_HOMEBREW_SIZE {
            return Err(LoaderError::TooLarge {
                actual: bytes.len(),
                max: MAX_HOMEBREW_SIZE,
            });
        }
        if bytes.len() % 4 != 0 {
            return Err(LoaderError::UnalignedLength);
        }
        load_address
            .checked_add(bytes.len() as u64)
            .ok_or(LoaderError::AddressOverflow {
                load_address,
                len: bytes.len(),
            })?;

        Ok(Self {
            entry_point: load_address,
            load_address,
            bytes,
        })
    }

    pub fn load_path<P: AsRef<Path>>(path: P, load_address: u64) -> Result<Self, LoaderError> {
        Self::from_bytes(fs::read(path)?, load_address)
    }

    pub fn map_into(&self, cpu: &UnicornCPU) -> Result<(), LoaderError> {
        cpu.write_bytes(self.load_address, &self.bytes)
            .map_err(|_| LoaderError::MemoryWrite {
                address: self.load_address,
            })?;
        cpu.set_pc(self.entry_point);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::UnicornCPU;

    #[test]
    fn rejects_invalid_raw_binaries() {
        assert!(matches!(
            HomebrewBinary::from_bytes(Vec::new(), DEFAULT_LOAD_ADDRESS),
            Err(LoaderError::Empty)
        ));
        assert!(matches!(
            HomebrewBinary::from_bytes(vec![0; 3], DEFAULT_LOAD_ADDRESS),
            Err(LoaderError::UnalignedLength)
        ));
        assert!(matches!(
            HomebrewBinary::from_bytes(vec![0; 4], u64::MAX - 1),
            Err(LoaderError::AddressOverflow { .. })
        ));
    }

    #[test]
    fn maps_raw_aarch64_binary_and_sets_pc() {
        let cpu = UnicornCPU::new().expect("cpu should initialize");
        let binary =
            HomebrewBinary::from_bytes(0xD503201Fu32.to_le_bytes().to_vec(), DEFAULT_LOAD_ADDRESS)
                .expect("valid raw binary");
        binary.map_into(&cpu).expect("map binary");

        assert_eq!(cpu.get_pc(), DEFAULT_LOAD_ADDRESS);
        assert_eq!(cpu.read_u32(DEFAULT_LOAD_ADDRESS), 0xD503201F);
    }
}
