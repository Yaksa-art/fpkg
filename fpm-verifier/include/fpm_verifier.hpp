#pragma once
#include <array>
#include <cstdint>
#include <filesystem>
#include <span>
#include <string>
#include <unordered_map>
#include <vector>

namespace fpm {

constexpr size_t BLAKE3_OUT_LEN = 32;
constexpr size_t ED25519_PUBKEY_LEN = 32;
constexpr size_t ED25519_SIG_LEN = 64;

using Blake3Hash = std::array<uint8_t, BLAKE3_OUT_LEN>;
using Ed25519Pubkey = std::array<uint8_t, ED25519_PUBKEY_LEN>;
using Ed25519Sig = std::array<uint8_t, ED25519_SIG_LEN>;

Blake3Hash blake3_file(const std::filesystem::path &path);
Blake3Hash blake3_bytes(std::span<const uint8_t> data);
std::string blake3_hex(const Blake3Hash &h);
Blake3Hash blake3_from_hex(const std::string &hex);

Blake3Hash merkle_root(const std::vector<Blake3Hash> &leaves);
Blake3Hash merkle_root_of_dir(const std::filesystem::path &data_dir);

bool ed25519_verify(
    const Ed25519Pubkey &pubkey,
    std::span<const uint8_t> message,
    const Ed25519Sig &sig
);

Ed25519Pubkey ed25519_pubkey_from_file(const std::filesystem::path &path);
Ed25519Sig   ed25519_sig_from_file(const std::filesystem::path &path);

using ChecksumMap = std::unordered_map<std::string, Blake3Hash>;
ChecksumMap parse_checksums_file(const std::filesystem::path &path);
void verify_checksums(const std::filesystem::path &data_dir,
                      const ChecksumMap &expected);

void verify_pki_chain(
    const std::filesystem::path &root_pubkey_path,
    const std::filesystem::path &package_pubkey_path,
    const std::filesystem::path &chain_sig_path
);

void verify_package(const std::filesystem::path &extracted_dir,
                    const std::filesystem::path &pubkey_path);

class VerifyError : public std::runtime_error {
public:
    explicit VerifyError(const std::string &msg) : std::runtime_error(msg) {}
};

}
