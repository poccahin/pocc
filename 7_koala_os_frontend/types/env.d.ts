interface ImportMetaEnv {
  readonly VITE_WS_URL?: string;
  readonly VITE_RPC_ENDPOINT?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
