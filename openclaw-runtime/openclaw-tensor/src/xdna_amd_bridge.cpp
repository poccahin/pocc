extern "C" int openclaw_xdna_bridge_enabled() {
#ifdef USE_AMD_XDNA
    return 1;
#else
    return 0;
#endif
}
