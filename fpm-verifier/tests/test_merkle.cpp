#include "fpm_verifier.hpp"
#include <gtest/gtest.h>

TEST(Merkle, SingleLeaf) {
    const uint8_t data[] = {1, 2, 3};
    auto leaf = fpm::blake3_bytes(std::span<const uint8_t>(data, 3));
    auto root = fpm::merkle_root({leaf});
    EXPECT_EQ(root, leaf);
}

TEST(Merkle, EmptyLeaves) {
    auto root = fpm::merkle_root({});
    auto empty = fpm::blake3_bytes(std::span<const uint8_t>{});
    EXPECT_EQ(root, empty);
}

TEST(Merkle, TwoLeavesDeterministic) {
    const uint8_t a[] = {0xaa};
    const uint8_t b[] = {0xbb};
    auto ha = fpm::blake3_bytes(std::span<const uint8_t>(a, 1));
    auto hb = fpm::blake3_bytes(std::span<const uint8_t>(b, 1));
    auto r1 = fpm::merkle_root({ha, hb});
    auto r2 = fpm::merkle_root({ha, hb});
    EXPECT_EQ(r1, r2);
}

TEST(Merkle, OrderMatters) {
    const uint8_t a[] = {0xaa};
    const uint8_t b[] = {0xbb};
    auto ha = fpm::blake3_bytes(std::span<const uint8_t>(a, 1));
    auto hb = fpm::blake3_bytes(std::span<const uint8_t>(b, 1));
    auto r_ab = fpm::merkle_root({ha, hb});
    auto r_ba = fpm::merkle_root({hb, ha});
    EXPECT_NE(r_ab, r_ba);
}

TEST(Merkle, OddNumberOfLeaves) {
    std::vector<fpm::Blake3Hash> leaves;
    for (uint8_t i = 0; i < 5; ++i) {
        leaves.push_back(fpm::blake3_bytes(std::span<const uint8_t>(&i, 1)));
    }
    EXPECT_NO_THROW(fpm::merkle_root(leaves));
}
