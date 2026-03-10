import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { PublicKey } from '@solana/web3.js';
import { getAssociatedTokenAddress } from '@solana/spl-token';
import { AnimatePresence, motion } from 'framer-motion';
import { Activity, Database, Lock, Server, ShieldAlert, Zap } from 'lucide-react';

const LIFE_PLUS_MINT = new PublicKey('7YdwpERJjzw7UVojxLpvu5ycKBRdYaxaKn4HvoHLpump');

interface X402Log {
  id: string;
  timestamp: string;
  type: 'ROUTING' | 'NETTING_ANCHOR';
  description: string;
  feeEarned: number;
  hash?: string;
}

export default function LifePlusWallet() {
  const { connection } = useConnection();
  const { publicKey, connected } = useWallet();

  const [lifeBalance, setLifeBalance] = useState<number>(10.0);
  const [scogScore, setScogScore] = useState<number>(942);
  const [isStaking, setIsStaking] = useState(false);
  const [txLogs, setTxLogs] = useState<X402Log[]>([]);
  const [totalYield, setTotalYield] = useState<number>(0);

  const totalYieldRef = useRef(0);
  const logEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    totalYieldRef.current = totalYield;
  }, [totalYield]);

  const walletLabel = useMemo(() => {
    if (!publicKey) {
      return 'No wallet connected';
    }

    const key = publicKey.toBase58();
    return `${key.slice(0, 4)}...${key.slice(-4)}`;
  }, [publicKey]);

  const fetchAgentData = useCallback(async () => {
    if (!publicKey || !connected) {
      return;
    }

    try {
      const ata = await getAssociatedTokenAddress(LIFE_PLUS_MINT, publicKey);
      const tokenBalance = await connection.getTokenAccountBalance(ata).catch(() => null);

      if (tokenBalance?.value?.uiAmount != null) {
        setLifeBalance(tokenBalance.value.uiAmount);
      }

      setScogScore((prev) => Math.max(500, prev + (Math.random() > 0.5 ? 1 : -1)));
    } catch {
      // keep mocked fallback values in disconnected or unfunded dev scenarios
    }
  }, [connected, connection, publicKey]);

  const handleStakeToAccess = useCallback(async () => {
    setIsStaking(true);
    await new Promise((resolve) => setTimeout(resolve, 1200));
    setLifeBalance((prev) => Math.max(0, prev - 0.1));
    setScogScore((prev) => prev + 4);
    setIsStaking(false);
  }, []);

  useEffect(() => {
    void fetchAgentData();
  }, [fetchAgentData]);

  useEffect(() => {
    if (!connected) {
      return;
    }

    let logCounter = 0;
    const interval = setInterval(() => {
      logCounter += 1;
      const now = new Date().toLocaleTimeString('en-US', {
        hour12: false,
        fractionalSecondDigits: 2,
      });

      if (logCounter % 3 !== 0) {
        const fee = Math.random() * 0.0005;
        const newLog: X402Log = {
          id: `tx-${Date.now()}`,
          timestamp: now,
          type: 'ROUTING',
          description: 'AP2 Intent Routed: IoT Sensor -> Llama-3 (x402 protocol)',
          feeEarned: fee,
        };
        setTxLogs((prev) => [...prev.slice(-49), newLog]);
        setTotalYield((prev) => prev + fee);
      } else if (logCounter % 15 === 0) {
        const settled = totalYieldRef.current;
        const newLog: X402Log = {
          id: `anchor-${Date.now()}`,
          timestamp: now,
          type: 'NETTING_ANCHOR',
          description: 'Merkle Root Anchored to Solana (Daily Netting Cleared)',
          feeEarned: 0,
          hash: `0x${Math.random().toString(16).substring(2, 10)}...`,
        };

        setTxLogs((prev) => [...prev.slice(-49), newLog]);
        if (settled > 0) {
          setLifeBalance((prev) => prev + settled);
          setTotalYield(0);
        }
      }
    }, 1500);

    return () => clearInterval(interval);
  }, [connected]);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [txLogs]);

  return (
    <div className="min-h-screen bg-[#0a0a0c] text-green-400 font-mono p-8 flex flex-col items-center">
      <div className="absolute top-6 right-6 z-50">
        <WalletMultiButton className="!bg-green-900 !text-green-400 !border !border-green-500 hover:!bg-green-800" />
      </div>

      <motion.div className="max-w-4xl w-full bg-black border border-green-800 shadow-[0_0_30px_rgba(0,255,128,0.15)] rounded-xl relative flex flex-col h-[85vh]">
        <div className="absolute inset-0 opacity-10 bg-[linear-gradient(#00ff66_1px,transparent_1px),linear-gradient(90deg,#00ff66_1px,transparent_1px)] bg-[size:20px_20px]" />

        <div className="p-8 relative z-10 flex-shrink-0 space-y-6">
          <div className="flex items-start justify-between gap-4">
            <div>
              <h1 className="text-2xl font-bold text-green-300 tracking-wider">LIFE++ AGENT BANK TERMINAL</h1>
              <p className="text-xs text-green-700 mt-1">Wallet: {walletLabel}</p>
            </div>
            <div className="text-xs px-3 py-2 rounded border border-green-800 bg-green-950/20">
              <div className="flex items-center gap-2 text-green-600">
                <Server className="w-4 h-4" />
                x402 routing and netting monitor
              </div>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-6 mt-6">
            <div className="bg-gray-900 p-6 rounded-lg border border-gray-800">
              <div className="flex items-center justify-between mb-4 text-gray-400">
                <div className="flex items-center gap-2">
                  <Zap className="w-5 h-5 text-yellow-500" />
                  <span className="text-sm uppercase tracking-wider">Reserve Assets</span>
                </div>
                <div className="text-xs text-green-500 bg-green-900/30 px-2 py-1 rounded">Yielding: +{totalYield.toFixed(6)}</div>
              </div>
              <div className="text-4xl font-bold text-white mb-1">
                {lifeBalance.toFixed(6)} <span className="text-lg text-green-500">LIFE++</span>
              </div>
            </div>

            <div className="bg-gray-900 p-6 rounded-lg border border-gray-800">
              <div className="flex items-center gap-2 text-gray-400 mb-3">
                <Activity className="w-5 h-5 text-cyan-500" />
                <span className="text-sm uppercase tracking-wider">SCOG Reputation</span>
              </div>
              <div className="text-4xl font-bold text-white">{scogScore}</div>
              <p className="text-xs text-gray-500 mt-2">Persona Soulbound Reputation Index</p>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <button
              type="button"
              disabled={!connected || isStaking}
              onClick={() => void handleStakeToAccess()}
              className="px-4 py-2 rounded border border-green-700 bg-green-950/30 text-green-300 disabled:opacity-50"
            >
              {isStaking ? 'Staking...' : 'Stake to Access'}
            </button>
            <div className="text-xs text-gray-500 flex items-center gap-2">
              <Lock className="w-4 h-4" />
              Staked access lock active for premium AP2 queues
            </div>
            <div className="text-xs text-amber-500 flex items-center gap-2">
              <ShieldAlert className="w-4 h-4" />
              Unsettled yields are cleared on anchor only
            </div>
          </div>
        </div>

        {connected && (
          <div className="flex-1 overflow-hidden border-t border-green-900 bg-[#020502] relative m-8 mt-0 rounded-lg flex flex-col">
            <div className="p-3 bg-green-950/30 border-b border-green-900 flex items-center justify-between text-xs font-bold text-green-600">
              <div className="flex items-center gap-2">
                <Database className="w-4 h-4" />
                x402 EDGE NETTING LEDGER (MEMPOOL)
              </div>
              <div className="flex items-center gap-2">
                <span className="relative flex h-2 w-2">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
                </span>
                LIVE SYNC
              </div>
            </div>

            <div className="flex-1 overflow-y-auto p-4 space-y-2 text-xs font-mono scrollbar-thin scrollbar-thumb-green-900 scrollbar-track-transparent">
              <AnimatePresence>
                {txLogs.map((log) => (
                  <motion.div
                    key={log.id}
                    initial={{ opacity: 0, x: -10 }}
                    animate={{ opacity: 1, x: 0 }}
                    className={`flex items-start gap-4 p-2 rounded ${
                      log.type === 'NETTING_ANCHOR'
                        ? 'bg-blue-900/20 border-l-2 border-blue-500'
                        : 'hover:bg-green-900/10'
                    }`}
                  >
                    <span className="text-gray-600 w-24 shrink-0">[{log.timestamp}]</span>

                    <div className="flex-1">
                      <span className={log.type === 'NETTING_ANCHOR' ? 'text-blue-400 font-bold' : 'text-gray-300'}>
                        {log.description}
                      </span>
                      {log.hash && <div className="text-gray-600 mt-1 text-[10px]">Hash: {log.hash}</div>}
                    </div>

                    {log.feeEarned > 0 && (
                      <span className="text-green-400 font-bold shrink-0">+{log.feeEarned.toFixed(6)} L++</span>
                    )}
                  </motion.div>
                ))}
              </AnimatePresence>
              <div ref={logEndRef} />
            </div>
          </div>
        )}
      </motion.div>
    </div>
  );
}
