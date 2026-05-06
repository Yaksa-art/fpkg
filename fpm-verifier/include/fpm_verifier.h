#pragma once
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define FPM_OK                   0
#define FPM_ERR_SIGNATURE        1
#define FPM_ERR_MERKLE           2
#define FPM_ERR_CHECKSUM         3
#define FPM_ERR_PKI              4
#define FPM_ERR_IO               5
#define FPM_ERR_INVALID_INPUT    6

typedef struct FpmVerifyResult {
    int  code;
    char message[256];
} FpmVerifyResult;

FpmVerifyResult fpm_verify_signature(
    const uint8_t *pubkey,   size_t pubkey_len,
    const uint8_t *message,  size_t message_len,
    const uint8_t *sig,      size_t sig_len
);

FpmVerifyResult fpm_verify_merkle(
    const char *data_dir,
    const char *expected_merkle_root_hex
);

FpmVerifyResult fpm_verify_checksums(
    const char *data_dir,
    const char *checksums_blake3_path
);

FpmVerifyResult fpm_verify_pki_chain(
    const char *root_pubkey_path,
    const char *package_pubkey_path,
    const char *chain_sig_path
);

FpmVerifyResult fpm_verify_package(
    const char *extracted_dir,
    const char *pubkey_path
);

#ifdef __cplusplus
}
#endif
