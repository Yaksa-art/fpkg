fn main() {
    // Link against M3 Verifier static library.
    // Build fpm-verifier first:
    //   cd ../fpm-verifier && cmake -B build -DCMAKE_BUILD_TYPE=Release && cmake --build build
    //
    // Then the linker will find libfpm_verifier.a + libsodium automatically.
    let verifier_build = std::path::PathBuf::from("../fpm-verifier/build");
    if verifier_build.exists() {
        println!(
            "cargo:rustc-link-search=native={}",
            verifier_build.display()
        );
        println!("cargo:rustc-link-lib=static=fpm_verifier");
        println!("cargo:rustc-link-lib=sodium");
        println!("cargo:rustc-cfg=feature=\"verifier_linked\"");
    } else {
        // Dev mode: verifier not yet built — verification step will be skipped
        println!("cargo:warning=fpm-verifier not built; skipping M3 link");
    }
    println!("cargo:rerun-if-changed=../fpm-verifier/build/libfpm_verifier.a");
}
