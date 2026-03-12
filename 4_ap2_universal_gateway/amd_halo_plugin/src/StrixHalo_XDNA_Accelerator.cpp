#include <cmath>
#include <cstddef>

float compute_variance(const float* tensor, std::size_t len) {
    if (tensor == nullptr || len == 0) {
        return 0.0f;
    }

    float mean = 0.0f;
    for (std::size_t i = 0; i < len; ++i) {
        mean += tensor[i];
    }
    mean /= static_cast<float>(len);

    float variance = 0.0f;
    for (std::size_t i = 0; i < len; ++i) {
        const float delta = tensor[i] - mean;
        variance += delta * delta;
    }

    return variance / static_cast<float>(len);
}
