#include "fpm_verifier.hpp"
#include <gtest/gtest.h>
#include <sodium.h>
#include <filesystem>
#include <fstream>

namespace fs = std::filesystem;

static fs::path make_pkg_dir(const std::string &name) {
    auto root = fs::temp_directory_path() / name;
    fs::create_directories(root / "META" / "scripts");
    fs::create_directories(root / "DATA" / "usr" / "bin");
    return root;
}

TEST(Verifier, ValidPackagePasses) {
    if (sodium_init() < 0) GTEST_SKIP();

    auto root = make_pkg_dir("fpm_pkg_valid");
    auto meta = root / "META";
    auto data = root / "DATA";

    std::ofstream(data / "usr" / "bin" / "hello") << "#!/bin/sh\necho hello";

    std::string manifest_content = "[package]\nname=\"hello\"\nversion=\"1.0.0\"\n";
    std::ofstream(meta / "manifest.toml") << manifest_content;

    auto bin_hash = fpm::blake3_file(data / "usr" / "bin" / "hello");
    std::ofstream cs(meta / "checksums.blake3");
    cs << fpm::blake3_hex(bin_hash) << "  usr/bin/hello\n";
    cs.close();

    auto merkle = fpm::merkle_root_of_dir(data);
    std::ofstream(meta / "content_tree.txt") << fpm::blake3_hex(merkle) << '\n';

    uint8_t pk[crypto_sign_ed25519_PUBLICKEYBYTES];
    uint8_t sk[crypto_sign_ed25519_SECRETKEYBYTES];
    crypto_sign_ed25519_keypair(pk, sk);

    uint8_t sig[crypto_sign_ed25519_BYTES];
    crypto_sign_ed25519_sign_detached(
        sig, nullptr,
        reinterpret_cast<const uint8_t *>(manifest_content.data()),
        manifest_content.size(), sk);

    std::ofstream(meta / "signature.ed25519", std::ios::binary)
        .write(reinterpret_cast<const char *>(sig), sizeof(sig));

    auto pk_path = meta / "pubkey.raw";
    std::ofstream(pk_path, std::ios::binary)
        .write(reinterpret_cast<const char *>(pk), sizeof(pk));

    EXPECT_NO_THROW(fpm::verify_package(root, pk_path));
    fs::remove_all(root);
}

TEST(Verifier, BadSignatureFails) {
    if (sodium_init() < 0) GTEST_SKIP();

    auto root = make_pkg_dir("fpm_pkg_badsig");
    auto meta = root / "META";
    auto data = root / "DATA";

    std::ofstream(data / "usr" / "bin" / "hello") << "hello";
    std::ofstream(meta / "manifest.toml") << "[package]\nname=\"x\"\n";

    auto h = fpm::blake3_file(data / "usr" / "bin" / "hello");
    std::ofstream cs(meta / "checksums.blake3");
    cs << fpm::blake3_hex(h) << "  usr/bin/hello\n";

    auto merkle = fpm::merkle_root_of_dir(data);
    std::ofstream(meta / "content_tree.txt") << fpm::blake3_hex(merkle);

    uint8_t pk[crypto_sign_ed25519_PUBLICKEYBYTES];
    uint8_t sk[crypto_sign_ed25519_SECRETKEYBYTES];
    crypto_sign_ed25519_keypair(pk, sk);

    uint8_t bad_sig[crypto_sign_ed25519_BYTES] = {};
    std::ofstream(meta / "signature.ed25519", std::ios::binary)
        .write(reinterpret_cast<const char *>(bad_sig), sizeof(bad_sig));

    auto pk_path = meta / "pubkey.raw";
    std::ofstream(pk_path, std::ios::binary)
        .write(reinterpret_cast<const char *>(pk), sizeof(pk));

    EXPECT_THROW(fpm::verify_package(root, pk_path), fpm::VerifyError);
    fs::remove_all(root);
}
