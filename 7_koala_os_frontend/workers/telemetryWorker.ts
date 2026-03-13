const TELEMETRY_WS_URL = 'wss://telemetry.ahin.io/stream';

const ws = new WebSocket(TELEMETRY_WS_URL);
ws.binaryType = 'arraybuffer';

ws.onmessage = (event: MessageEvent<ArrayBuffer>) => {
  const data = new Float32Array(event.data);

  for (let i = 0; i < data.length; i += 2) {
    const index = data[i];
    const friction = data[i + 1];

    if (!Number.isFinite(index) || !Number.isFinite(friction)) {
      continue;
    }

    postMessage({ index, friction });
  }
};

ws.onerror = () => {
  // Keep worker alive; host app can decide if telemetry degradation requires UI notification.
};
