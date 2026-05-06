#include "fpm_verifier.hpp"
#include <filesystem>
#include <iostream>
#include <string>
#include <vector>

static void usage(const char *argv0) {
    std::cerr
        << "Usage:\n"
        << "  " << argv0 << " package  <extracted-dir> <pubkey>\n"
        << "  " << argv0 << " merkle   <data-dir> <expected-root-hex>\n"
        << "  " << argv0 << " checksum <data-dir> <checksums.blake3>\n"
        << "  " << argv0 << " pki      <root-pubkey> <pkg-pubkey> <chain-sig>\n"
        << "  " << argv0 << " hash     <file>\n";
}

int main(int argc, char **argv) {
    if (argc < 2) { usage(argv[0]); return 1; }

    std::string cmd = argv[1];

    try {
        if (cmd == "package" && argc == 4) {
            fpm::verify_package(argv[2], argv[3]);
            std::cout << "OK\n";

        } else if (cmd == "merkle" && argc == 4) {
            auto actual = fpm::merkle_root_of_dir(argv[2]);
            auto hex    = fpm::blake3_hex(actual);
            std::cout << "merkle: " << hex << '\n';
            if (std::string(argv[3]) != hex) {
                std::cerr << "MISMATCH: expected " << argv[3] << '\n';
                return 2;
            }
            std::cout << "OK\n";

        } else if (cmd == "checksum" && argc == 4) {
            auto map = fpm::parse_checksums_file(argv[3]);
            fpm::verify_checksums(argv[2], map);
            std::cout << "OK\n";

        } else if (cmd == "pki" && argc == 5) {
            fpm::verify_pki_chain(argv[2], argv[3], argv[4]);
            std::cout << "OK\n";

        } else if (cmd == "hash" && argc == 3) {
            auto h = fpm::blake3_file(argv[2]);
            std::cout << fpm::blake3_hex(h) << '\n';

        } else {
            usage(argv[0]);
            return 1;
        }
    } catch (const fpm::VerifyError &e) {
        std::cerr << "FAIL: " << e.what() << '\n';
        return 2;
    } catch (const std::exception &e) {
        std::cerr << "error: " << e.what() << '\n';
        return 1;
    }

    return 0;
}
