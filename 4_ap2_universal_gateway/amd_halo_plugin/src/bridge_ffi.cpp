#include <cstddef>

float compute_variance(const float* tensor, std::size_t len);

extern "C" float amd_halo_verify_pocc(const float* tensor, std::size_t len) {
    return compute_variance(tensor, len);
}
