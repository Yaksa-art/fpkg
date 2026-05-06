cmake_minimum_required(VERSION 3.20)
project(fpm-verifier VERSION 0.1.5 LANGUAGES CXX C)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

find_package(PkgConfig REQUIRED)
pkg_check_modules(SODIUM REQUIRED libsodium)

add_subdirectory(vendor/blake3)

add_library(fpm_verifier STATIC
    src/blake3_hasher.cpp
    src/merkle.cpp
    src/ed25519.cpp
    src/checksum_file.cpp
    src/pki.cpp
    src/verifier.cpp
    src/c_api.cpp
)

target_include_directories(fpm_verifier PUBLIC
    include
    vendor/blake3
    ${SODIUM_INCLUDE_DIRS}
)

target_link_libraries(fpm_verifier PUBLIC
    blake3
    ${SODIUM_LIBRARIES}
)

target_compile_options(fpm_verifier PRIVATE
    -Wall -Wextra -Wpedantic -Wno-unused-parameter
    $<$<CONFIG:Release>:-O3>
)

add_executable(fpm-verifier src/main.cpp)
target_link_libraries(fpm-verifier PRIVATE fpm_verifier)

enable_testing()
add_subdirectory(tests)
