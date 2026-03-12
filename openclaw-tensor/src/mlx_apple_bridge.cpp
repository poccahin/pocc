#include <cstddef>
#include <iostream>

#if defined(__APPLE__) && __has_include(<mlx/mlx.h>)
#include <mlx/mlx.h>
#define OPENCLAW_MLX_AVAILABLE 1
#else
#define OPENCLAW_MLX_AVAILABLE 0
#endif

extern "C" {

/// @brief 在 Apple Silicon 上计算意图张量与能力基底的语义摩擦力。
/// @param intent_ptr Rust 传来的意图向量原生指针。
/// @param intent_dim 向量维度。
/// @param capability_ptr Rust 传来的能力向量原生指针。
/// @param capability_dim 向量维度。
/// @param out_friction 输出的摩擦力标量 (指针写回)。
/// @return 状态码：0 成功，-1 输入错误，-2 执行异常或后端不可用。
int tensor_evaluate_friction_c(const float *intent_ptr, std::size_t intent_dim,
                               const float *capability_ptr,
                               std::size_t capability_dim,
                               float *out_friction) {
  if (intent_ptr == nullptr || capability_ptr == nullptr || out_friction == nullptr) {
    std::cerr << "🧩 [MLX ERROR] Null pointer input." << std::endl;
    return -1;
  }

  if (intent_dim != capability_dim || intent_dim == 0) {
    std::cerr << "📐 [MLX ERROR] Tensor dimensionality mismatch or zero." << std::endl;
    return -1;
  }

#if OPENCLAW_MLX_AVAILABLE
  try {
    using namespace mlx::core;

    const array intent = array(intent_ptr, {static_cast<int>(intent_dim)}, float32);
    const array capability =
        array(capability_ptr, {static_cast<int>(capability_dim)}, float32);

    const array dot_prod = sum(multiply(intent, capability));
    const array norm_i = sqrt(sum(multiply(intent, intent)));
    const array norm_c = sqrt(sum(multiply(capability, capability)));

    // 避免零范数导致除零，理论上应由调用侧保证非零向量，这里做硬防护。
    const array denom = maximum(multiply(norm_i, norm_c), array(1.0e-12f, float32));

    const array alignment = divide(dot_prod, denom);
    const array friction = subtract(array(1.0f, float32), alignment);

    eval(friction);
    *out_friction = friction.item<float>();
    return 0;
  } catch (const std::exception &e) {
    std::cerr << "💥 [MLX FATAL] Hardware execution failed: " << e.what() << std::endl;
    return -2;
  }
#else
  std::cerr << "⚠️ [MLX ERROR] Apple MLX backend unavailable on this build target." << std::endl;
  return -2;
#endif
}

} // extern "C"
