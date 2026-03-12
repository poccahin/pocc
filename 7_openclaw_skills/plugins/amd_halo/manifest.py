import ctypes
import os
import sys

from openclaw.sdk import BasePlugin, register_plugin


class AmdHaloPlugin(BasePlugin):
    def __init__(self):
        super().__init__()
        self.name = "amd-halo-accelerator"
        self.version = "1.0.0"
        self._lib = None
        self._load_binary()

    def _load_binary(self):
        # 根据系统加载对应的内核驱动
        ext = ".dll" if sys.platform == "win32" else ".so"
        lib_path = os.path.join(os.path.dirname(__file__), f"libopenclaw_amd_halo{ext}")

        try:
            self._lib = ctypes.CDLL(lib_path)
            # 定义 FFI 接口：float amd_halo_verify_pocc(const float* tensor, size_t len)
            self._lib.amd_halo_verify_pocc.argtypes = [ctypes.POINTER(ctypes.c_float), ctypes.c_size_t]
            self._lib.amd_halo_verify_pocc.restype = ctypes.c_float
            print("🚀 [OpenClaw] Strix Halo XDNA 2 Plugin Activated. Ready for 50 TOPS load.")
        except Exception as e:
            print(f"⚠️ [OpenClaw] Failed to ignite AMD Halo: {e}")

    def verify_pocc(self, tensor_data):
        """核心调用：将任务压入 XDNA 2 硬件队列"""
        if not self._lib:
            return None

        # 转换为 C 指针
        c_array = (ctypes.c_float * len(tensor_data))(*tensor_data)
        variance = self._lib.amd_halo_verify_pocc(c_array, len(tensor_data))
        return variance


# 注册插件
register_plugin(AmdHaloPlugin())
