fn main() {
    // Configure LAME linking for macOS
    #[cfg(target_os = "macos")]
    {
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
