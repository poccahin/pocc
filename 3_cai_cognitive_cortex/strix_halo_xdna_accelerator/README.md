# Strix Halo XDNA Accelerator (PoCC)

该目录提供 `amd_halo_verify_pocc` 的 C ABI 插件实现，用于把 PoCC 验证下沉到 AMD Ryzen AI 的 XDNA 2 NPU。

## 构建模式

### 1) 开发/CI（无 AMD SDK）
默认走 mock 路径，可直接编译验证 FFI 连通性：

```bash
g++ -std=c++17 -O2 -fPIC -shared \
  3_cai_cognitive_cortex/strix_halo_xdna_accelerator/StrixHalo_XDNA_Accelerator.cpp \
  -o libstrix_halo_xdna_accelerator.so
```

### 2) 硬件加速（有 AMD Ryzen AI SW Stack）
在编译时启用 `USE_AMD_XDNA_RT=1` 并链接 XDNA 运行时库：

```bash
g++ -std=c++17 -O2 -fPIC -shared -DUSE_AMD_XDNA_RT=1 \
  3_cai_cognitive_cortex/strix_halo_xdna_accelerator/StrixHalo_XDNA_Accelerator.cpp \
  -I<amd_ryzen_ai_include_path> -L<amd_ryzen_ai_lib_path> -lxdna_rt \
  -o libstrix_halo_xdna_accelerator.so
```

> 固件默认加载路径：`firmware/pocc_validator_v1_halo.xclbin`。
