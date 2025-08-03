pub mod command;
pub mod device;
pub mod drawable;
pub mod instance;
pub mod metalshaper;
pub mod render;
pub mod sync;

pub use command::*;
pub use device::*;
pub use drawable::*;
pub use instance::*;
pub use render::*;
pub use sync::*;

#[cfg(test)]
mod tests {
    use anyhow::{Ok, Result};

    use super::*;

    #[test]
    fn headless_environment() -> Result<()> {
        let instance = BMLInstance::new(None)?;

        let device = MTLDevice::create(instance)?;

        Ok(())
    }
}
