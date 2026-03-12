#include <cstddef>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <mutex>
#include <stdexcept>
#include <string>
#include <vector>

/**
 * Strix Halo XDNA 2 NPU 加速插件（PoCC 验证路径）
 *
 * 设计目标：
 * 1) 在具备 AMD Ryzen AI SW Stack 的机器上，直连 XDNA Runtime；
 * 2) 在缺少 SDK 的开发/CI 环境中，保持可编译（mock 路径）；
 * 3) 对外提供稳定 C ABI，供 Rust/Go/Python FFI 调用。
 */

// ---------------------------------------------------------------------
// 可选：接入 AMD XDNA Runtime
// 编译时定义 USE_AMD_XDNA_RT=1 即可启用真实硬件路径。
// ---------------------------------------------------------------------
#if defined(USE_AMD_XDNA_RT)
#include <xdna_rt_api.h>
#endif

namespace {

constexpr const char* kDefaultXclbin = "firmware/pocc_validator_v1_halo.xclbin";

class StrixHaloXdnaEngine {
public:
    StrixHaloXdnaEngine() {
        std::cout << "🚀 [AMD] Initializing Strix Halo XDNA 2 NPU Accelerator..." << std::endl;
#if defined(USE_AMD_XDNA_RT)
        if (xdna_enumerate_devices(&device_) != XDNA_SUCCESS) {
            throw std::runtime_error("❌ [AMD] No XDNA 2 NPU detected on this Ryzen AI system.");
        }

        if (xdna_load_binary(device_, kDefaultXclbin, &pocc_binary_) != XDNA_SUCCESS) {
            throw std::runtime_error(std::string("❌ [AMD] Failed to load xclbin: ") + kDefaultXclbin);
        }
        std::cout << "✅ [AMD] XDNA 2 Primed. 500GB/s Memory Interface Locked." << std::endl;
#else
        std::cout
            << "⚠️ [AMD] Built without USE_AMD_XDNA_RT. Running deterministic mock path for CI/dev."
            << std::endl;
#endif
    }

    float validate_agent_intent(const float* intent_tensor, std::size_t size) {
        if (intent_tensor == nullptr || size == 0) {
            return 0.0f;
        }

#if defined(USE_AMD_XDNA_RT)
        xdna_buffer_handle_t input_buf{};
        xdna_buffer_handle_t output_buf{};

        if (xdna_alloc_shared_buffer(device_, size * sizeof(float), &input_buf) != XDNA_SUCCESS ||
            xdna_alloc_shared_buffer(device_, sizeof(float), &output_buf) != XDNA_SUCCESS) {
            throw std::runtime_error("❌ [AMD] Failed to allocate shared XDNA buffers.");
        }

        auto* mapped_input = static_cast<float*>(xdna_map_buffer(input_buf));
        std::memcpy(mapped_input, intent_tensor, size * sizeof(float));

        xdna_exec_request_t req{};
        if (xdna_submit_pocc_task(pocc_binary_, input_buf, output_buf, &req) != XDNA_SUCCESS) {
            xdna_free_buffer(input_buf);
            xdna_free_buffer(output_buf);
            throw std::runtime_error("❌ [AMD] Failed to submit PoCC task to XDNA runtime.");
        }

        xdna_wait_request(req, -1);

        auto* mapped_result = static_cast<float*>(xdna_map_buffer(output_buf));
        const float result = *mapped_result;

        xdna_free_buffer(input_buf);
        xdna_free_buffer(output_buf);
        return result;
#else
        // Mock 路径：用一个轻量的“方差近似”作为可重复验证结果，
        // 保证上层链路（FFI、网关、计费）可在无硬件时联调。
        double mean = 0.0;
        for (std::size_t i = 0; i < size; ++i) {
            mean += static_cast<double>(intent_tensor[i]);
        }
        mean /= static_cast<double>(size);

        double variance = 0.0;
        for (std::size_t i = 0; i < size; ++i) {
            const double d = static_cast<double>(intent_tensor[i]) - mean;
            variance += d * d;
        }
        variance /= static_cast<double>(size);

        return static_cast<float>(variance);
#endif
    }

private:
#if defined(USE_AMD_XDNA_RT)
    xdna_device_handle_t device_{};
    xdna_binary_handle_t pocc_binary_{};
#endif
};

StrixHaloXdnaEngine& engine_instance() {
    static StrixHaloXdnaEngine engine;
    return engine;
}

std::mutex g_engine_mutex;

}  // namespace

extern "C" float amd_halo_verify_pocc(const float* tensor_ptr, std::size_t len) {
    try {
        std::lock_guard<std::mutex> lock(g_engine_mutex);
        return engine_instance().validate_agent_intent(tensor_ptr, len);
    } catch (const std::exception& ex) {
        std::cerr << "❌ [AMD] amd_halo_verify_pocc error: " << ex.what() << std::endl;
        return -1.0f;
    }
}
