#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod moltenvk {
    pub fn setup() {
        let home = std::env::var_os("HOME")
            .unwrap()
            .to_string_lossy()
            .to_string();

        // TODO: Detect VulkanSDK versions.
        let vulkan_sdk = format!("{}/VulkanSDK/1.4.313.1/macOS", home);
        let dyld_fallback_library_path = format!("{}/lib", vulkan_sdk);
        let vk_icd_filenames = format!("{}/share/vulkan/icd.d/MoltenVK_icd.json", vulkan_sdk);
        let vk_layer_path = format!("{}/share/vulkan/explicit_layer.d", vulkan_sdk);

        println!(
            "cargo:rustc-env=DYLD_FALLBACK_LIBRARY_PATH={}",
            dyld_fallback_library_path
        );
        println!("cargo::rustc-env=VK_ICD_FILENAMES={}", vk_icd_filenames);
        println!("cargo::rustc-env=VK_LAYER_PATH={}", vk_layer_path);
    }
}
