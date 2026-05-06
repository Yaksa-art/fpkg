#include "fpm_verifier.hpp"
#include "blake3.h"
#include <fstream>
#include <stdexcept>

namespace fpm {

Blake3Hash blake3_file(const std::filesystem::path &path) {
    std::ifstream f(path, std::ios::binary);
    if (!f) throw VerifyError("cannot open: " + path.string());

    blake3_hasher hasher;
    blake3_hasher_init(&hasher);

    char buf[65536];
    while (f.read(buf, sizeof(buf)) || f.gcount() > 0) {
        blake3_hasher_update(&hasher,
            reinterpret_cast<const uint8_t *>(buf),
            static_cast<size_t>(f.gcount()));
    }

    Blake3Hash out;
    blake3_hasher_finalize(&hasher, out.data(), BLAKE3_OUT_LEN);
    return out;
}

Blake3Hash blake3_bytes(std::span<const uint8_t> data) {
    blake3_hasher hasher;
    blake3_hasher_init(&hasher);
    blake3_hasher_update(&hasher, data.data(), data.size());
    Blake3Hash out;
    blake3_hasher_finalize(&hasher, out.data(), BLAKE3_OUT_LEN);
    return out;
}

std::string blake3_hex(const Blake3Hash &h) {
    constexpr char digits[] = "0123456789abcdef";
    std::string s;
    s.reserve(BLAKE3_OUT_LEN * 2);
    for (uint8_t b : h) {
        s += digits[b >> 4];
        s += digits[b & 0xf];
    }
    return s;
}

Blake3Hash blake3_from_hex(const std::string &hex) {
    if (hex.size() != BLAKE3_OUT_LEN * 2)
        throw VerifyError("invalid blake3 hex length: " + hex);
    Blake3Hash h;
    for (size_t i = 0; i < BLAKE3_OUT_LEN; ++i) {
        auto nibble = [&](char c) -> uint8_t {
            if (c >= '0' && c <= '9') return c - '0';
            if (c >= 'a' && c <= 'f') return c - 'a' + 10;
            if (c >= 'A' && c <= 'F') return c - 'A' + 10;
            throw VerifyError(std::string("invalid hex char: ") + c);
        };
        h[i] = (nibble(hex[2*i]) << 4) | nibble(hex[2*i+1]);
    }
    return h;
}

}
