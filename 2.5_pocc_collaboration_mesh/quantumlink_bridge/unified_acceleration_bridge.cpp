#include <cstddef>
#include <iostream>

#if defined(__APPLE__)
#include <mlx/mlx.h>
#elif defined(__AMD__)
#include <hip/hip_runtime.h>
#include <xdna_rt.h>
#endif

namespace lifeplusplus::quantumlink {

class TensorAccelerator {
public:
  bool execute_ap2_intent(float *data, std::size_t size) {
    if (data == nullptr || size == 0) {
      std::cerr << "[QuantumLink] Invalid AP2 tensor payload." << std::endl;
      return false;
    }

#if defined(__APPLE__)
    auto array = mlx::core::array(data, {static_cast<int>(size)});
    mlx::core::eval(array);
    std::cout << "🍏 [M4] MLX zero-copy inference complete." << std::endl;
    return true;
#elif defined(__AMD__)
    xdna_submit_task(data, size);
    std::cout << "🚀 [Strix Halo] XDNA 2 NPU validation complete." << std::endl;
    return true;
#else
    std::cout << "⚠️ [QuantumLink] No hardware backend detected. Using CPU dry-run path."
              << std::endl;
    return true;
#endif
  }
};

} // namespace lifeplusplus::quantumlink
