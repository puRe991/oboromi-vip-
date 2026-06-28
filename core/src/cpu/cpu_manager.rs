use crate::cpu::UnicornCPU;
use memmap2::MmapMut;

pub const CORE_COUNT: usize = 8;

#[cfg(not(target_pointer_width = "64"))]
compile_error!("oboromi requires a 64-bit architecture to emulate 12GB of RAM.");
// 12GB Memory
pub const MEMORY_SIZE: u64 = 12 * 1024 * 1024 * 1024;
pub const MEMORY_BASE: u64 = 0x0;

pub struct CpuManager {
    pub cores: Vec<UnicornCPU>,
    // Anonymous mmap reserves the guest address space without forcing a 12GB heap allocation.
    pub shared_memory: MmapMut,
}

impl CpuManager {
    pub fn new() -> Self {
        Self::try_new().expect("failed to initialize CPU manager")
    }

    pub fn try_new() -> Result<Self, std::io::Error> {
        Self::try_new_with_memory_size(MEMORY_SIZE)
    }

    pub fn try_new_with_memory_size(memory_size: u64) -> Result<Self, std::io::Error> {
        let mut shared_memory = MmapMut::map_anon(memory_size as usize)?;
        let memory_ptr = shared_memory.as_mut_ptr();

        let mut cores = Vec::with_capacity(CORE_COUNT);
        for i in 0..CORE_COUNT {
            // SAFETY: `shared_memory` is owned by CpuManager and moved with a stable mapping
            // address; it outlives every UnicornCPU stored next to it in this struct.
            let cpu = unsafe { UnicornCPU::new_with_shared_mem(i as u32, memory_ptr, memory_size) };
            if let Some(cpu) = cpu {
                cores.push(cpu);
            } else {
                return Err(std::io::Error::other(format!("failed to create core {i}")));
            }
        }

        Ok(Self {
            cores,
            shared_memory,
        })
    }

    pub fn run_all(&self) {
        // for now, just step all cores sequentially (round-robin)
        // in the future, this would be threaded
        for core in &self.cores {
            core.step();
        }
    }

    pub fn get_core(&self, id: usize) -> Option<&UnicornCPU> {
        self.cores.get(id)
    }
}

impl Default for CpuManager {
    fn default() -> Self {
        Self::new()
    }
}
