#include "fpm_verifier.hpp"
#include <gtest/gtest.h>
#include <fstream>
#include <filesystem>

namespace fs = std::filesystem;

static fs::path make_temp_dir(const std::string &name) {
    auto p = fs::temp_directory_path() / name;
    fs::create_directories(p);
    return p;
}

TEST(Checksums, ParseAndVerifyValid) {
    auto dir = make_temp_dir("fpm_cs_valid");
    auto file = dir / "bin" / "tool";
    fs::create_directories(file.parent_path());
    std::ofstream(file) << "binary content";

    auto h = fpm::blake3_file(file);
    auto hex = fpm::blake3_hex(h);

    auto cs_path = dir / "checksums.blake3";
    std::ofstream cs(cs_path);
    cs << hex << "  bin/tool\n";
    cs.close();

    auto map = fpm::parse_checksums_file(cs_path);
    EXPECT_NO_THROW(fpm::verify_checksums(dir, map));
    fs::remove_all(dir);
}

TEST(Checksums, MismatchDetected) {
    auto dir = make_temp_dir("fpm_cs_mismatch");
    auto file = dir / "bin" / "tool";
    fs::create_directories(file.parent_path());
    std::ofstream(file) << "original";

    auto cs_path = dir / "checksums.blake3";
    std::ofstream cs(cs_path);
    cs << std::string(64, '0') << "  bin/tool\n";
    cs.close();

    auto map = fpm::parse_checksums_file(cs_path);
    EXPECT_THROW(fpm::verify_checksums(dir, map), fpm::VerifyError);
    fs::remove_all(dir);
}

TEST(Checksums, MissingFileDetected) {
    auto dir = make_temp_dir("fpm_cs_missing");
    auto cs_path = dir / "checksums.blake3";
    std::ofstream cs(cs_path);
    cs << std::string(64, '0') << "  bin/ghost\n";
    cs.close();

    auto map = fpm::parse_checksums_file(cs_path);
    EXPECT_THROW(fpm::verify_checksums(dir, map), fpm::VerifyError);
    fs::remove_all(dir);
}
