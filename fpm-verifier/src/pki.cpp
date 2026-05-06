#include "fpm_verifier.hpp"
#include <fstream>

namespace fpm {

void verify_pki_chain(
    const std::filesystem::path &root_pubkey_path,
    const std::filesystem::path &package_pubkey_path,
    const std::filesystem::path &chain_sig_path
) {
    auto root_key = ed25519_pubkey_from_file(root_pubkey_path);

    std::ifstream pkf(package_pubkey_path, std::ios::binary);
    if (!pkf) throw VerifyError("cannot open package pubkey: " + package_pubkey_path.string());
    std::vector<uint8_t> pkg_key_bytes(
        std::istreambuf_iterator<char>(pkf),
        std::istreambuf_iterator<char>());

    auto chain_sig = ed25519_sig_from_file(chain_sig_path);

    bool ok = ed25519_verify(
        root_key,
        std::span<const uint8_t>(pkg_key_bytes.data(), pkg_key_bytes.size()),
        chain_sig
    );

    if (!ok)
        throw VerifyError(
            "PKI chain verification failed: package pubkey not signed by repo root key"
        );
}

}
