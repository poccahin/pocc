extern "C" int openclaw_mlx_bridge_enabled() {
#ifdef USE_APPLE_MLX
    return 1;
#else
    return 0;
#endif
}
