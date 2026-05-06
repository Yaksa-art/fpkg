#include "fpm_verifier.hpp"
#include <fstream>
#include <sstream>

namespace fpm {

ChecksumMap parse_checksums_file(const std::filesystem::path &path) {
    std::ifstream f(path);
    if (!f) throw VerifyError("cannot open checksums: " + path.string());

    ChecksumMap map;
    std::string line;
    while (std::getline(f, line)) {
        if (line.empty() || line[0] == '#') continue;
        std::istringstream ss(line);
        std::string hex, filepath;
        ss >> hex >> filepath;
        if (hex.empty() || filepath.empty()) continue;
        map[filepath] = blake3_from_hex(hex);
    }
    return map;
}

void verify_checksums(
    const std::filesystem::path &data_dir,
    const ChecksumMap &expected
) {
    for (auto &[rel_path, expected_hash] : expected) {
        auto abs = data_dir / rel_path;
        if (!std::filesystem::exists(abs))
            throw VerifyError("missing file: " + rel_path);

        auto actual = blake3_file(abs);
        if (actual != expected_hash) {
            throw VerifyError(
                "checksum mismatch: " + rel_path +
                " expected=" + blake3_hex(expected_hash) +
                " actual=" + blake3_hex(actual)
            );
        }
    }
}

}
