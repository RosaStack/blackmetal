use anyhow::Result;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::sync::Arc;

pub struct BMLInstance {
    layer: Option<BMLLayer>,
}

impl BMLInstance {
    pub fn new(layer: Option<BMLLayer>) -> Result<Arc<Self>> {
        Ok(Arc::new(Self { layer }))
    }

    pub fn layer(&self) -> &Option<BMLLayer> {
        &self.layer
    }
}

pub struct BMLLayer {
    pub window_display: RawDisplayHandle,
    pub window_handle: RawWindowHandle,
    pub width: u32,
    pub height: u32,
}
