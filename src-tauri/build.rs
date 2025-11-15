fn main() {
    // Force environment variables for mp3lame-sys before it runs
    std::env::set_var("LAME_STATIC", "0");

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        // Set environment variables that mp3lame-sys build script will read
        std::env::set_var("LAME_HOST", "x86_64-apple-darwin");
        std::env::set_var("TARGET", "x86_64-apple-darwin");

        // Force use of system library paths
        if let Ok(lame_lib_dir) = std::env::var("LAME_LIB_DIR") {
            println!("cargo:rustc-link-search=native={}", lame_lib_dir);
            std::env::set_var("LIBRARY_PATH", &lame_lib_dir);
        }
        if let Ok(lame_include_dir) = std::env::var("LAME_INCLUDE_DIR") {
            std::env::set_var("CPATH", &lame_include_dir);
        }

        // Force dynamic linking to system LAME library
        println!("cargo:rustc-link-lib=dylib=mp3lame");
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        // Set environment variables that mp3lame-sys build script will read
        std::env::set_var("LAME_HOST", "aarch64-apple-darwin");
        std::env::set_var("TARGET", "aarch64-apple-darwin");

        // Force use of system library paths
        if let Ok(lame_lib_dir) = std::env::var("LAME_LIB_DIR") {
            println!("cargo:rustc-link-search=native={}", lame_lib_dir);
            std::env::set_var("LIBRARY_PATH", &lame_lib_dir);
        }
        if let Ok(lame_include_dir) = std::env::var("LAME_INCLUDE_DIR") {
            std::env::set_var("CPATH", &lame_include_dir);
        }

        // Force dynamic linking to system LAME library
        println!("cargo:rustc-link-lib=dylib=mp3lame");
    }

    tauri_build::build()
}
