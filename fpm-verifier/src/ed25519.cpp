#include "fpm_verifier.hpp"
#include <sodium.h>
#include <fstream>
#include <sstream>

namespace fpm {

bool ed25519_verify(
    const Ed25519Pubkey &pubkey,
    std::span<const uint8_t> message,
    const Ed25519Sig &sig
) {
    if (sodium_init() < 0)
        throw VerifyError("libsodium init failed");
    return crypto_sign_ed25519_verify_detached(
        sig.data(),
        message.data(), message.size(),
        pubkey.data()
    ) == 0;
}

Ed25519Pubkey ed25519_pubkey_from_file(const std::filesystem::path &path) {
    std::ifstream f(path, std::ios::binary);
    if (!f) throw VerifyError("cannot open pubkey: " + path.string());

    std::string line;
    std::string hex;
    while (std::getline(f, line)) {
        if (line.rfind("-----", 0) == 0) continue;
        if (line.empty()) continue;
        hex += line;
    }

    if (hex.size() == ED25519_PUBKEY_LEN * 2) {
        Ed25519Pubkey key;
        for (size_t i = 0; i < ED25519_PUBKEY_LEN; ++i) {
            auto h = [](char c) -> uint8_t {
                if (c >= '0' && c <= '9') return c - '0';
                if (c >= 'a' && c <= 'f') return c - 'a' + 10;
                if (c >= 'A' && c <= 'F') return c - 'A' + 10;
                return 0;
            };
            key[i] = (h(hex[2*i]) << 4) | h(hex[2*i+1]);
        }
        return key;
    }

    if (hex.size() == ED25519_PUBKEY_LEN) {
        Ed25519Pubkey key;
        std::copy(
            reinterpret_cast<const uint8_t *>(hex.data()),
            reinterpret_cast<const uint8_t *>(hex.data()) + ED25519_PUBKEY_LEN,
            key.data()
        );
        return key;
    }

    throw VerifyError("unrecognised pubkey format in: " + path.string());
}

Ed25519Sig ed25519_sig_from_file(const std::filesystem::path &path) {
    std::ifstream f(path, std::ios::binary);
    if (!f) throw VerifyError("cannot open sig: " + path.string());

    std::vector<uint8_t> raw(
        std::istreambuf_iterator<char>(f),
        std::istreambuf_iterator<char>());

    if (raw.size() != ED25519_SIG_LEN)
        throw VerifyError("signature file has wrong size: " + path.string());

    Ed25519Sig sig;
    std::copy(raw.begin(), raw.end(), sig.begin());
    return sig;
}

}
