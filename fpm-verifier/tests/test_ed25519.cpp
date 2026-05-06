#include "fpm_verifier.hpp"
#include <gtest/gtest.h>
#include <sodium.h>

TEST(Ed25519, SignAndVerify) {
    if (sodium_init() < 0) GTEST_SKIP();

    uint8_t pk[crypto_sign_ed25519_PUBLICKEYBYTES];
    uint8_t sk[crypto_sign_ed25519_SECRETKEYBYTES];
    crypto_sign_ed25519_keypair(pk, sk);

    const uint8_t msg[] = {'f', 'p', 'm'};
    uint8_t sig[crypto_sign_ed25519_BYTES];
    crypto_sign_ed25519_sign_detached(sig, nullptr, msg, 3, sk);

    fpm::Ed25519Pubkey pubkey;
    fpm::Ed25519Sig   signature;
    std::copy(pk, pk + fpm::ED25519_PUBKEY_LEN, pubkey.begin());
    std::copy(sig, sig + fpm::ED25519_SIG_LEN,  signature.begin());

    bool ok = fpm::ed25519_verify(pubkey,
        std::span<const uint8_t>(msg, 3), signature);
    EXPECT_TRUE(ok);
}

TEST(Ed25519, WrongSignatureRejected) {
    if (sodium_init() < 0) GTEST_SKIP();

    uint8_t pk[crypto_sign_ed25519_PUBLICKEYBYTES];
    uint8_t sk[crypto_sign_ed25519_SECRETKEYBYTES];
    crypto_sign_ed25519_keypair(pk, sk);

    const uint8_t msg[] = {'f', 'p', 'm'};
    uint8_t sig[crypto_sign_ed25519_BYTES] = {};

    fpm::Ed25519Pubkey pubkey;
    fpm::Ed25519Sig   signature;
    std::copy(pk, pk + fpm::ED25519_PUBKEY_LEN, pubkey.begin());
    std::copy(sig, sig + fpm::ED25519_SIG_LEN,  signature.begin());

    bool ok = fpm::ed25519_verify(pubkey,
        std::span<const uint8_t>(msg, 3), signature);
    EXPECT_FALSE(ok);
}

TEST(Ed25519, WrongKeyRejected) {
    if (sodium_init() < 0) GTEST_SKIP();

    uint8_t pk1[crypto_sign_ed25519_PUBLICKEYBYTES];
    uint8_t sk1[crypto_sign_ed25519_SECRETKEYBYTES];
    uint8_t pk2[crypto_sign_ed25519_PUBLICKEYBYTES];
    uint8_t sk2[crypto_sign_ed25519_SECRETKEYBYTES];
    crypto_sign_ed25519_keypair(pk1, sk1);
    crypto_sign_ed25519_keypair(pk2, sk2);

    const uint8_t msg[] = {'t', 'e', 's', 't'};
    uint8_t sig[crypto_sign_ed25519_BYTES];
    crypto_sign_ed25519_sign_detached(sig, nullptr, msg, 4, sk1);

    fpm::Ed25519Pubkey wrong_key;
    fpm::Ed25519Sig   signature;
    std::copy(pk2, pk2 + fpm::ED25519_PUBKEY_LEN, wrong_key.begin());
    std::copy(sig,  sig  + fpm::ED25519_SIG_LEN,  signature.begin());

    bool ok = fpm::ed25519_verify(wrong_key,
        std::span<const uint8_t>(msg, 4), signature);
    EXPECT_FALSE(ok);
}
