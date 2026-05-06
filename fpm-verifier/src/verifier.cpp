#include "fpm_verifier.hpp"
#include <fstream>
#include <sstream>

namespace fpm {

void verify_package(
    const std::filesystem::path &extracted_dir,
    const std::filesystem::path &pubkey_path
) {
    auto meta_dir    = extracted_dir / "META";
    auto data_dir    = extracted_dir / "DATA";
    auto sig_path    = meta_dir / "signature.ed25519";
    auto manifest_path = meta_dir / "manifest.toml";
    auto checksums_path = meta_dir / "checksums.blake3";

    for (auto &p : {meta_dir, data_dir, sig_path, manifest_path, checksums_path}) {
        if (!std::filesystem::exists(p))
            throw VerifyError("missing required path: " + p.string());
    }

    auto pubkey = ed25519_pubkey_from_file(pubkey_path);

    std::ifstream mf(manifest_path, std::ios::binary);
    if (!mf) throw VerifyError("cannot open manifest");
    std::vector<uint8_t> manifest_bytes(
        std::istreambuf_iterator<char>(mf),
        std::istreambuf_iterator<char>());

    auto sig = ed25519_sig_from_file(sig_path);

    if (!ed25519_verify(pubkey, std::span<const uint8_t>(manifest_bytes.data(), manifest_bytes.size()), sig))
        throw VerifyError("Ed25519 signature verification failed for manifest.toml");

    auto checksums = parse_checksums_file(checksums_path);
    verify_checksums(data_dir, checksums);

    auto actual_root   = merkle_root_of_dir(data_dir);
    auto actual_root_hex = blake3_hex(actual_root);

    std::ifstream content_tree_f(meta_dir / "content_tree.txt");
    if (content_tree_f) {
        std::string expected_root_hex;
        std::getline(content_tree_f, expected_root_hex);
        if (!expected_root_hex.empty() && expected_root_hex != actual_root_hex)
            throw VerifyError(
                "Merkle root mismatch: expected=" + expected_root_hex +
                " actual=" + actual_root_hex
            );
    }
}

}
