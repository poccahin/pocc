import { useEffect, useMemo, useState } from 'react';

export type PlanetaryEventType =
  | 'TENSOR_INTERCEPTION'
  | 'LIFE_BURN'
  | 'ROUTING_SUCCESS';

export type PlanetaryPulse = {
  id: string;
  type: PlanetaryEventType;
  agent_id: string;
  gps: [number, number];
  diagnosis?: string;
  amount_burned?: number;
  route?: string;
  receivedAt: number;
};

const MAX_PULSES = 120;
const PULSE_TTL_MS = 2_500;

export function useOmnisphere(url: string) {
  const [pulses, setPulses] = useState<PlanetaryPulse[]>([]);
  const [isConnected, setIsConnected] = useState(false);

  useEffect(() => {
    const ws = new WebSocket(url);

    ws.onopen = () => setIsConnected(true);
    ws.onclose = () => setIsConnected(false);
    ws.onerror = () => setIsConnected(false);

    ws.onmessage = (message) => {
      try {
        const payload = JSON.parse(message.data as string) as Omit<PlanetaryPulse, 'id' | 'receivedAt'>;
        if (!payload?.type || !payload?.gps || payload.gps.length !== 2) {
          return;
        }

        const pulse: PlanetaryPulse = {
          ...payload,
          id: `${payload.agent_id}-${Date.now()}-${Math.random().toString(16).slice(2)}`,
          receivedAt: Date.now(),
        };

        setPulses((prev) => [...prev.slice(-MAX_PULSES + 1), pulse]);
      } catch {
        // swallow malformed payloads
      }
    };

    return () => ws.close();
  }, [url]);

  useEffect(() => {
    const interval = window.setInterval(() => {
      const now = Date.now();
      setPulses((prev) => prev.filter((pulse) => now - pulse.receivedAt <= PULSE_TTL_MS));
    }, 250);

    return () => window.clearInterval(interval);
  }, []);

  const groupedCounts = useMemo(() => {
    return pulses.reduce(
      (acc, pulse) => {
        acc[pulse.type] = (acc[pulse.type] ?? 0) + 1;
        return acc;
      },
      {
        TENSOR_INTERCEPTION: 0,
        LIFE_BURN: 0,
        ROUTING_SUCCESS: 0,
      } as Record<PlanetaryEventType, number>,
    );
  }, [pulses]);

  return {
    pulses,
    groupedCounts,
    isConnected,
  };
}
