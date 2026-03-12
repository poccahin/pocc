#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <stdexcept>

#if defined(USE_AMD_XDNA)
#include <xdna/xdna_engine.h>
#endif

namespace {

float cosine_friction_cpu_fallback(
    const float* intent_ptr,
    std::size_t intent_dim,
    const float* capability_ptr,
    std::size_t capability_dim
) {
    if (intent_ptr == nullptr || capability_ptr == nullptr || intent_dim == 0 ||
        capability_dim == 0 || intent_dim != capability_dim) {
        throw std::runtime_error("Invalid tensor inputs for cosine friction.");
    }

    double dot = 0.0;
    double norm_intent = 0.0;
    double norm_capability = 0.0;

    for (std::size_t i = 0; i < intent_dim; ++i) {
        const double intent = static_cast<double>(intent_ptr[i]);
        const double capability = static_cast<double>(capability_ptr[i]);
        dot += intent * capability;
        norm_intent += intent * intent;
        norm_capability += capability * capability;
    }

    if (norm_intent == 0.0 || norm_capability == 0.0) {
        return 1.0f;
    }

    const double cosine = dot / (std::sqrt(norm_intent) * std::sqrt(norm_capability));
    return static_cast<float>(1.0 - cosine);
}

}  // namespace

extern "C" {

int tensor_evaluate_friction_c(
    const float* intent_ptr,
    std::size_t intent_dim,
    const float* capability_ptr,
    std::size_t capability_dim,
    float* out_friction
) {
    if (out_friction == nullptr) {
        std::cerr << "🧭 [XDNA ERROR] Output pointer is null." << std::endl;
        return -1;
    }

    if (intent_dim != capability_dim || intent_dim == 0 || intent_ptr == nullptr ||
        capability_ptr == nullptr) {
        std::cerr << "📐 [XDNA ERROR] Tensor dimensionality mismatch/zero or null input." << std::endl;
        return -1;
    }

#if defined(USE_AMD_XDNA)
    float* dma_intent = nullptr;
    float* dma_capability = nullptr;
    float* dma_result = nullptr;

    try {
        const std::size_t bytes_size = intent_dim * sizeof(float);

        if (posix_memalign(reinterpret_cast<void**>(&dma_intent), 4096, bytes_size) != 0 ||
            posix_memalign(reinterpret_cast<void**>(&dma_capability), 4096, bytes_size) != 0 ||
            posix_memalign(reinterpret_cast<void**>(&dma_result), 4096, sizeof(float)) != 0) {
            std::cerr << "💾 [XDNA FATAL] Failed to allocate 4K-aligned DMA memory." << std::endl;
            free(dma_intent);
            free(dma_capability);
            free(dma_result);
            return -2;
        }

        std::memcpy(dma_intent, intent_ptr, bytes_size);
        std::memcpy(dma_capability, capability_ptr, bytes_size);

        xdna::Device device = xdna::get_edge_npu_device();
        xdna::CommandQueue queue = device.create_queue();
        xdna::Kernel friction_kernel = device.load_kernel("cosine_friction_f32");

        friction_kernel.set_arg(0, dma_intent);
        friction_kernel.set_arg(1, dma_capability);
        friction_kernel.set_arg(2, static_cast<std::uint32_t>(intent_dim));
        friction_kernel.set_arg(3, dma_result);

        queue.submit(friction_kernel);
        queue.wait_idle();

        *out_friction = dma_result[0];

        free(dma_intent);
        free(dma_capability);
        free(dma_result);
        return 0;
    } catch (const std::exception& e) {
        std::cerr << "💥 [XDNA FATAL] NPU hardware execution panicked: " << e.what() << std::endl;
        free(dma_intent);
        free(dma_capability);
        free(dma_result);
        return -3;
    }
#else
    try {
        *out_friction = cosine_friction_cpu_fallback(
            intent_ptr,
            intent_dim,
            capability_ptr,
            capability_dim
        );
        return 0;
    } catch (const std::exception& e) {
        std::cerr << "💥 [XDNA MOCK ERROR] " << e.what() << std::endl;
        return -3;
    }
#endif
}

}  // extern "C"
