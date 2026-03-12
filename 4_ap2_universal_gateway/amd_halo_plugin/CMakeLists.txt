cmake_minimum_required(VERSION 3.22)
project(OpenClaw_AMD_Halo_Plugin LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 🚨 核心逻辑：自动探测 AMD 环境
if(WIN32)
    message(STATUS "🔍 [AMD] Detecting Windows Ryzen AI SDK...")
    set(XDNA_SDK_PATH "C:/Program Files/AMD/RyzenAI/1.2.0") # 默认安装路径
    include_directories("${XDNA_SDK_PATH}/include")
    link_directories("${XDNA_SDK_PATH}/lib")
    set(AMD_LIBS xdna_rt.lib onnxruntime.lib)
else()
    message(STATUS "🔍 [AMD] Detecting Linux ROCm & XDNA Stack...")
    find_package(HIP REQUIRED)
    include_directories("/opt/rocm/include" "/usr/include/x86_64-linux-gnu/xdna")
    link_directories("/opt/rocm/lib")
    set(AMD_LIBS hip_hcc xdna_rt)
endif()

# 编译核心插件库
add_library(openclaw_amd_halo SHARED
    src/StrixHalo_XDNA_Accelerator.cpp
    src/bridge_ffi.cpp
)

target_link_libraries(openclaw_amd_halo PRIVATE ${AMD_LIBS})

# 优化标志：针对 Strix Halo 的 Zen 5 + RDNA 3.5 架构
if(CMAKE_CXX_COMPILER_ID MATCHES "Clang|GNU")
    target_compile_options(openclaw_amd_halo PRIVATE -march=znver5 -O3 -fPIC)
endif()

message(STATUS "✅ [AMD] Build configuration complete for Strix Halo Accelerator.")
