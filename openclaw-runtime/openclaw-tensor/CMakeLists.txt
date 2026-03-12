cmake_minimum_required(VERSION 3.18)
project(openclaw_tensor LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_CXX_FLAGS_RELEASE "-O3 -march=native -ffast-math -DNDEBUG")

set(SOURCES
    src/tensor_engine.cpp
    src/mlx_apple_bridge.cpp
    src/xdna_amd_bridge.cpp
)

add_library(openclaw_tensor STATIC ${SOURCES})

if (APPLE)
    find_package(MLX REQUIRED)
    target_link_libraries(openclaw_tensor PRIVATE MLX::mlx)
    target_compile_definitions(openclaw_tensor PRIVATE USE_APPLE_MLX)
elseif (UNIX AND NOT APPLE)
    find_path(XDNA_INCLUDE_DIRS "xdna/xdna_engine.h")
    target_include_directories(openclaw_tensor PRIVATE ${XDNA_INCLUDE_DIRS})
    target_compile_definitions(openclaw_tensor PRIVATE USE_AMD_XDNA)
endif()

set_property(TARGET openclaw_tensor PROPERTY POSITION_INDEPENDENT_CODE ON)
