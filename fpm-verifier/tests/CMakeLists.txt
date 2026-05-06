find_package(GTest QUIET)
if(NOT GTest_FOUND)
    include(FetchContent)
    FetchContent_Declare(googletest
        GIT_REPOSITORY https://github.com/google/googletest.git
        GIT_TAG        v1.14.0
    )
    FetchContent_MakeAvailable(googletest)
endif()

add_executable(fpm_verifier_tests
    test_blake3.cpp
    test_merkle.cpp
    test_ed25519.cpp
    test_checksums.cpp
    test_verifier.cpp
)

target_link_libraries(fpm_verifier_tests
    PRIVATE fpm_verifier GTest::gtest_main
)

include(GoogleTest)
gtest_discover_tests(fpm_verifier_tests)
