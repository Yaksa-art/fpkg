#include "fpm_verifier.hpp"
#include "blake3.h"
#include <algorithm>
#include <vector>

namespace fpm {

Blake3Hash merkle_root(const std::vector<Blake3Hash> &leaves) {
    if (leaves.empty()) {
        return blake3_bytes({});
    }
    if (leaves.size() == 1) {
        return leaves[0];
    }

    std::vector<Blake3Hash> current = leaves;

    while (current.size() > 1) {
        std::vector<Blake3Hash> next;
        for (size_t i = 0; i < current.size(); i += 2) {
            if (i + 1 < current.size()) {
                std::array<uint8_t, BLAKE3_OUT_LEN * 2> pair;
                std::copy(current[i].begin(),   current[i].end(),   pair.begin());
                std::copy(current[i+1].begin(), current[i+1].end(), pair.begin() + BLAKE3_OUT_LEN);
                next.push_back(blake3_bytes(std::span<const uint8_t>(pair.data(), pair.size())));
            } else {
                next.push_back(current[i]);
            }
        }
        current = std::move(next);
    }

    return current[0];
}

Blake3Hash merkle_root_of_dir(const std::filesystem::path &data_dir) {
    std::vector<std::filesystem::path> files;
    for (auto &entry : std::filesystem::recursive_directory_iterator(data_dir)) {
        if (entry.is_regular_file()) {
            files.push_back(entry.path());
        }
    }
    std::sort(files.begin(), files.end());

    std::vector<Blake3Hash> leaves;
    leaves.reserve(files.size());
    for (auto &f : files) {
        leaves.push_back(blake3_file(f));
    }

    return merkle_root(leaves);
}

}
