# Building fpm-verifier

## Dependencies

- CMake 3.20+
- C++20 compiler (GCC 12+ / Clang 15+)
- libsodium (`libsodium-dev` on Debian/Ubuntu, `libsodium-devel` on Fedora)
- BLAKE3 source is vendored in `vendor/blake3/` — download from https://github.com/BLAKE3-team/BLAKE3

## Build

```sh
cd fpm-verifier
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j$(nproc)
```

The static library is at `build/libfpm_verifier.a`.
The CLI binary is at `build/fpm-verifier`.

## Tests

```sh
cmake -B build -DCMAKE_BUILD_TYPE=Debug
cmake --build build -j$(nproc)
ctest --test-dir build --output-on-failure
```

## Vendoring BLAKE3

```sh
git clone https://github.com/BLAKE3-team/BLAKE3 tmp_blake3
cp tmp_blake3/c/blake3.h         vendor/blake3/
cp tmp_blake3/c/blake3_impl.h    vendor/blake3/
cp tmp_blake3/c/blake3.c         vendor/blake3/
cp tmp_blake3/c/blake3_dispatch.c vendor/blake3/
cp tmp_blake3/c/blake3_portable.c vendor/blake3/
cp tmp_blake3/c/blake3_sse2_x86-64_unix.S  vendor/blake3/
cp tmp_blake3/c/blake3_sse41_x86-64_unix.S vendor/blake3/
cp tmp_blake3/c/blake3_avx2_x86-64_unix.S  vendor/blake3/
cp tmp_blake3/c/blake3_avx512_x86-64_unix.S vendor/blake3/
rm -rf tmp_blake3
```

## FFI from Rust

Link against `libfpm_verifier.a` and `libsodium` from Rust via `build.rs`:

```rust
println!("cargo:rustc-link-lib=static=fpm_verifier");
println!("cargo:rustc-link-search=native=../fpm-verifier/build");
println!("cargo:rustc-link-lib=sodium");
```

Then call functions from `fpm_verifier.h` via `bindgen` or manual `extern "C"` declarations.
