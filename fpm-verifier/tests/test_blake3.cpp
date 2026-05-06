#include "fpm_verifier.hpp"
#include <gtest/gtest.h>
#include <fstream>
#include <filesystem>

namespace fs = std::filesystem;

TEST(Blake3, KnownHash) {
    const uint8_t input[] = {'a', 'b', 'c'};
    auto h = fpm::blake3_bytes(std::span<const uint8_t>(input, 3));
    auto hex = fpm::blake3_hex(h);
    EXPECT_EQ(hex.size(), 64u);
    EXPECT_EQ(hex, "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85");
}

TEST(Blake3, EmptyInput) {
    auto h = fpm::blake3_bytes(std::span<const uint8_t>{});
    auto hex = fpm::blake3_hex(h);
    EXPECT_EQ(hex.size(), 64u);
}

TEST(Blake3, FileHash) {
    auto tmp = fs::temp_directory_path() / "fpm_test_blake3.txt";
    std::ofstream f(tmp);
    f << "hello world";
    f.close();
    auto h = fpm::blake3_file(tmp);
    EXPECT_EQ(fpm::blake3_hex(h).size(), 64u);
    fs::remove(tmp);
}

TEST(Blake3, HexRoundtrip) {
    const uint8_t input[] = {0xde, 0xad, 0xbe, 0xef};
    auto h = fpm::blake3_bytes(std::span<const uint8_t>(input, 4));
    auto hex = fpm::blake3_hex(h);
    auto h2  = fpm::blake3_from_hex(hex);
    EXPECT_EQ(h, h2);
}
