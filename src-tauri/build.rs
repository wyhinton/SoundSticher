fn main() {
    // Configure LAME build for macOS x86_64 architecture
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        // Set correct host architecture for LAME build (from Stack Exchange solution)
        println!("cargo:rustc-env=LAME_HOST=x86_64-apple-darwin");

        // Help mp3lame-sys find the LAME library
        if let Ok(lame_lib_dir) = std::env::var("LAME_LIB_DIR") {
            println!("cargo:rustc-link-search=native={}", lame_lib_dir);
        }
        if let Ok(lame_include_dir) = std::env::var("LAME_INCLUDE_DIR") {
            println!("cargo:include={}", lame_include_dir);
        }

        // Try to link with system LAME library
        println!("cargo:rustc-link-lib=mp3lame");
    }

    // Configure LAME build for macOS ARM64 architecture
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        // Set correct host architecture for LAME build
        println!("cargo:rustc-env=LAME_HOST=aarch64-apple-darwin");

        // Help mp3lame-sys find the LAME library
        if let Ok(lame_lib_dir) = std::env::var("LAME_LIB_DIR") {
            println!("cargo:rustc-link-search=native={}", lame_lib_dir);
        }
        if let Ok(lame_include_dir) = std::env::var("LAME_INCLUDE_DIR") {
            println!("cargo:include={}", lame_include_dir);
        }

        // Try to link with system LAME library
        println!("cargo:rustc-link-lib=mp3lame");
    }

    tauri_build::build()
}
