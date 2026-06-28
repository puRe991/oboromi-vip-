use ash::vk;
use thiserror::Error;

pub mod sm86;
pub mod spirv;

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("failed to load Vulkan loader: {0}")]
    Loader(#[from] ash::LoadingError),
    #[error("Vulkan call failed: {0:?}")]
    Vk(#[from] vk::Result),
}

#[derive(Default)]
pub struct VkState {
    pub entry: Option<ash::Entry>,
    pub instance: Option<ash::Instance>,
}

impl VkState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) -> Result<(), GpuError> {
        if self.instance.is_some() {
            return Ok(());
        }

        // SAFETY: `Entry::load` only loads function pointers from the process Vulkan loader.
        // The returned Entry owns no borrowed data and is stored before any Instance using it.
        let entry = unsafe { ash::Entry::load()? };
        let app_name = c"oboromi";
        let engine_name = c"oboromi";
        let app_info = vk::ApplicationInfo {
            p_application_name: app_name.as_ptr(),
            application_version: vk::make_api_version(0, 0, 0, 1),
            p_engine_name: engine_name.as_ptr(),
            engine_version: vk::make_api_version(0, 0, 0, 1),
            api_version: vk::API_VERSION_1_0,
            ..Default::default()
        };
        let create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            ..Default::default()
        };

        // SAFETY: `create_info` points to stack data valid for this call; no extensions/layers
        // are supplied, and allocation callbacks are not used.
        let instance = unsafe { entry.create_instance(&create_info, None)? };
        self.entry = Some(entry);
        self.instance = Some(instance);
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.entry.is_some() && self.instance.is_some()
    }
}

impl Drop for VkState {
    fn drop(&mut self) {
        if let Some(instance) = self.instance.take() {
            // SAFETY: The instance was created by this VkState and is destroyed at most once.
            unsafe { instance.destroy_instance(None) };
        }
    }
}

#[derive(Default)]
pub struct State {
    pub shared_memory: *mut u8,
    pub global_memory: *mut u8,
    pub pc: u64,
    pub vk: VkState,
}

impl State {
    pub fn new() -> Self {
        Self {
            shared_memory: core::ptr::null_mut(),
            global_memory: core::ptr::null_mut(),
            pc: 0,
            vk: VkState::new(),
        }
    }

    pub fn init(&mut self) -> Result<(), GpuError> {
        self.vk.init()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vk_state_starts_uninitialized_safely() {
        let vk = VkState::new();
        assert!(!vk.is_initialized());
    }
}
