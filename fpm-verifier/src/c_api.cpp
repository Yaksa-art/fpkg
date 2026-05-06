#include "fpm_verifier.h"
#include "fpm_verifier.hpp"
#include <cstring>

static FpmVerifyResult ok_result() {
    FpmVerifyResult r{};
    r.code = FPM_OK;
    r.message[0] = '\0';
    return r;
}

static FpmVerifyResult err_result(int code, const char *msg) {
    FpmVerifyResult r{};
    r.code = code;
    std::strncpy(r.message, msg, sizeof(r.message) - 1);
    r.message[sizeof(r.message) - 1] = '\0';
    return r;
}

extern "C" {

FpmVerifyResult fpm_verify_signature(
    const uint8_t *pubkey,  size_t pubkey_len,
    const uint8_t *message, size_t message_len,
    const uint8_t *sig,     size_t sig_len
) {
    if (pubkey_len != fpm::ED25519_PUBKEY_LEN || sig_len != fpm::ED25519_SIG_LEN)
        return err_result(FPM_ERR_INVALID_INPUT, "wrong key or sig length");
    try {
        fpm::Ed25519Pubkey key;
        fpm::Ed25519Sig signature;
        std::copy(pubkey, pubkey + fpm::ED25519_PUBKEY_LEN, key.begin());
        std::copy(sig,    sig    + fpm::ED25519_SIG_LEN,    signature.begin());
        bool ok = fpm::ed25519_verify(key,
            std::span<const uint8_t>(message, message_len), signature);
        if (!ok) return err_result(FPM_ERR_SIGNATURE, "ed25519 verification failed");
        return ok_result();
    } catch (const std::exception &e) {
        return err_result(FPM_ERR_SIGNATURE, e.what());
    }
}

FpmVerifyResult fpm_verify_merkle(
    const char *data_dir,
    const char *expected_hex
) {
    try {
        auto actual = fpm::merkle_root_of_dir(data_dir);
        auto actual_hex = fpm::blake3_hex(actual);
        if (actual_hex != expected_hex)
            return err_result(FPM_ERR_MERKLE,
                ("merkle mismatch: " + actual_hex).c_str());
        return ok_result();
    } catch (const std::exception &e) {
        return err_result(FPM_ERR_MERKLE, e.what());
    }
}

FpmVerifyResult fpm_verify_checksums(
    const char *data_dir,
    const char *checksums_path
) {
    try {
        auto map = fpm::parse_checksums_file(checksums_path);
        fpm::verify_checksums(data_dir, map);
        return ok_result();
    } catch (const std::exception &e) {
        return err_result(FPM_ERR_CHECKSUM, e.what());
    }
}

FpmVerifyResult fpm_verify_pki_chain(
    const char *root_pubkey_path,
    const char *package_pubkey_path,
    const char *chain_sig_path
) {
    try {
        fpm::verify_pki_chain(root_pubkey_path, package_pubkey_path, chain_sig_path);
        return ok_result();
    } catch (const std::exception &e) {
        return err_result(FPM_ERR_PKI, e.what());
    }
}

FpmVerifyResult fpm_verify_package(
    const char *extracted_dir,
    const char *pubkey_path
) {
    try {
        fpm::verify_package(extracted_dir, pubkey_path);
        return ok_result();
    } catch (const std::exception &e) {
        return err_result(FPM_ERR_SIGNATURE, e.what());
    }
}

}
