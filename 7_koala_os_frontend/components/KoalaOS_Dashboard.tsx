import React, { useEffect, useRef, useState } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { MeshDistortMaterial, Sphere, Stars } from '@react-three/drei';
import { useWallet } from '@solana/wallet-adapter-react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { AnimatePresence, motion } from 'framer-motion';
import { Cpu, Database, Lock, ShieldAlert, Zap } from 'lucide-react';
import * as THREE from 'three';

const TensorSphere = () => {
  const sphereRef = useRef<THREE.Mesh>(null);

  useFrame(({ clock }) => {
    if (sphereRef.current) {
      sphereRef.current.rotation.y = clock.getElapsedTime() * 0.05;
      sphereRef.current.rotation.z = clock.getElapsedTime() * 0.02;
    }
  });

  return (
    <Sphere ref={sphereRef} args={[1.8, 64, 64]} position={[0, 0, -1]}>
      <MeshDistortMaterial
        color="#00ffcc"
        emissive="#004433"
        wireframe
        distort={0.25}
        speed={1.5}
        roughness={0.2}
      />
    </Sphere>
  );
};

interface X402Log {
  id: string;
  timestamp: string;
  type: 'ROUTING' | 'NETTING_ANCHOR';
  description: string;
  feeEarned: number;
}

export default function KoalaOSDashboard() {
  const { connected } = useWallet();

  const [gdp, setGdp] = useState(1.24);
  const [stakeRate, setStakeRate] = useState(58.4);
  const [tps, setTps] = useState(14200);
  const [slashAlert, setSlashAlert] = useState<string | null>(null);

  const [lifeBalance, setLifeBalance] = useState<number>(10);
  const [scogScore] = useState<number>(942);
  const [txLogs, setTxLogs] = useState<X402Log[]>([]);
  const [totalYield, setTotalYield] = useState<number>(0);
  const [isStaking, setIsStaking] = useState(false);
  const logEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const macroInterval = setInterval(() => {
      setGdp((prev) => prev + Math.random() * 0.001);
      setStakeRate((prev) => Math.min(60, Math.max(50, prev + (Math.random() * 0.2 - 0.1))));
      setTps(Math.floor(10000 + Math.random() * 8000));
    }, 1000);

    const slashInterval = setInterval(() => {
      if (Math.random() > 0.85) {
        const fakeAddress = `0x${Math.random().toString(16).substring(2, 10).toUpperCase()}...`;
        setSlashAlert(`[EXECUTION] Persona Slash triggered for Agent ${fakeAddress}. Stake Burned.`);
        setTimeout(() => setSlashAlert(null), 4000);
      }
    }, 7000);

    return () => {
      clearInterval(macroInterval);
      clearInterval(slashInterval);
    };
  }, []);

  useEffect(() => {
    if (!connected) {
      return;
    }

    let logCounter = 0;
    const microInterval = setInterval(() => {
      logCounter += 1;
      const d = new Date();
      const now = `${d.toLocaleTimeString('en-US', { hour12: false })}.${String(Math.floor(d.getMilliseconds() / 10)).padStart(2, '0')}`;

      if (logCounter % 3 !== 0) {
        const fee = Math.random() * 0.0005;
        setTxLogs((prev) => [
          ...prev.slice(-29),
          {
            id: `tx-${Date.now()}`,
            timestamp: now,
            type: 'ROUTING',
            description: 'AP2 Intent Routed (x402 protocol)',
            feeEarned: fee,
          },
        ]);
        setTotalYield((prev) => prev + fee);
      } else if (logCounter % 15 === 0) {
        setTxLogs((prev) => [
          ...prev.slice(-29),
          {
            id: `anchor-${Date.now()}`,
            timestamp: now,
            type: 'NETTING_ANCHOR',
            description: 'Merkle Root Anchored to Solana',
            feeEarned: 0,
          },
        ]);

        setLifeBalance((prev) => prev + totalYield);
      }
    }, 1500);

    return () => clearInterval(microInterval);
  }, [connected, totalYield]);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [txLogs]);

  const handleStake = () => {
    setIsStaking(true);

    setTimeout(() => {
      setIsStaking(false);
      alert('✅ [PROMETHEAN SPARK] AP2 routing unlocked!');
    }, 2000);
  };

  return (
    <div className="w-screen h-screen bg-[#050505] text-green-400 font-mono relative overflow-hidden flex">
      <div className="absolute inset-0 z-0 pointer-events-none">
        <Canvas camera={{ position: [0, 0, 5] }}>
          <ambientLight intensity={0.2} />
          <pointLight position={[10, 10, 10]} intensity={1.5} color="#00ffcc" />
          <Stars radius={100} depth={50} count={3000} factor={4} saturation={0} fade speed={1} />
          <TensorSphere />
        </Canvas>
      </div>

      <div className="absolute inset-0 z-10 flex justify-between p-8 pointer-events-none">
        <div className="w-1/3 flex flex-col gap-8 pointer-events-auto">
          <div>
            <h1 className="text-3xl font-bold tracking-tighter text-white">
              KOALA <span className="text-green-500">OS</span>
            </h1>
            <p className="text-sm text-green-700 mt-1">Kardashev Type-I Command Center</p>
          </div>

          <div className="bg-black/60 backdrop-blur-md border border-green-900/50 p-6 rounded-xl">
            <h2 className="text-xs tracking-[0.2em] text-green-600 mb-2">GLOBAL AGENT GDP</h2>
            <div className="text-4xl font-bold text-white text-shadow-neon-green">
              ${gdp.toFixed(4)} <span className="text-lg text-green-500">Trillion</span>
            </div>
          </div>

          <div className="bg-black/60 backdrop-blur-md border border-green-900/50 p-6 rounded-xl flex justify-between">
            <div>
              <h2 className="text-xs tracking-[0.2em] text-yellow-600 mb-2">DEFLATION LOCK-UP</h2>
              <div className="text-3xl font-bold text-yellow-500">{stakeRate.toFixed(2)}%</div>
            </div>
            <div className="text-right">
              <h2 className="text-xs tracking-[0.2em] text-blue-600 mb-2">NETWORK TPS</h2>
              <div className="text-3xl font-bold text-blue-400">{tps.toLocaleString()}</div>
            </div>
          </div>

          <AnimatePresence>
            {slashAlert && (
              <motion.div
                initial={{ opacity: 0, x: -50 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, filter: 'blur(5px)' }}
                className="bg-red-950/80 backdrop-blur-md border border-red-600 p-4 rounded-xl text-red-500 font-bold shadow-[0_0_20px_rgba(255,0,0,0.3)]"
              >
                ⚠️ {slashAlert}
              </motion.div>
            )}
          </AnimatePresence>
        </div>

        <div className="w-1/3 flex flex-col gap-6 pointer-events-auto">
          <div className="flex justify-end">
            <WalletMultiButton className="!bg-black/80 backdrop-blur-md !text-green-400 !border !border-green-800 hover:!bg-green-900/50 transition-all" />
          </div>

          {!connected ? (
            <div className="flex-1 flex flex-col items-center justify-center bg-black/40 backdrop-blur-md border border-gray-800 rounded-xl">
              <ShieldAlert className="w-12 h-12 text-gray-600 mb-4" />
              <p className="text-gray-500">Wallet Offline. Connect to sync Agent Node.</p>
            </div>
          ) : (
            <>
              <div className="grid grid-cols-2 gap-4">
                <div className="bg-black/60 backdrop-blur-md p-5 rounded-xl border border-green-900/50">
                  <div className="flex items-center gap-2 mb-2 text-gray-400">
                    <Zap className="w-4 h-4 text-yellow-500" />
                    <span className="text-xs">ASSETS</span>
                  </div>
                  <div className="text-2xl font-bold text-white">{lifeBalance.toFixed(4)}</div>
                  <div className="text-[10px] text-green-500 mt-1">Yield: +{totalYield.toFixed(6)}</div>
                </div>

                <div className="bg-black/60 backdrop-blur-md p-5 rounded-xl border border-blue-900/50">
                  <div className="flex items-center gap-2 mb-2 text-gray-400">
                    <Cpu className="w-4 h-4 text-blue-500" />
                    <span className="text-xs">S_COG SCORE</span>
                  </div>
                  <div className="text-2xl font-bold text-blue-400">{scogScore}</div>
                  <div className="w-full bg-gray-800 h-1 rounded-full mt-2">
                    <div className="h-full bg-blue-500 w-[78%]" />
                  </div>
                </div>
              </div>

              <div className="flex-1 flex flex-col bg-black/60 backdrop-blur-md rounded-xl border border-green-900/50 overflow-hidden">
                <div className="p-3 bg-green-900/20 border-b border-green-900/50 flex justify-between items-center text-xs font-bold text-green-600">
                  <div className="flex items-center gap-2">
                    <Database className="w-3 h-3" /> EDGE NETTING LEDGER
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="animate-ping absolute inline-flex h-2 w-2 rounded-full bg-green-400 opacity-75" />
                    <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
                  </div>
                </div>
                <div className="flex-1 overflow-y-auto p-3 space-y-2 text-[10px] scrollbar-thin scrollbar-thumb-green-900 scrollbar-track-transparent">
                  <AnimatePresence>
                    {txLogs.map((log) => (
                      <motion.div
                        key={log.id}
                        initial={{ opacity: 0, x: 10 }}
                        animate={{ opacity: 1, x: 0 }}
                        className={`flex justify-between p-2 rounded ${
                          log.type === 'NETTING_ANCHOR'
                            ? 'bg-blue-900/20 border-l border-blue-500'
                            : 'hover:bg-green-900/10'
                        }`}
                      >
                        <div className="flex gap-2">
                          <span className="text-gray-600">[{log.timestamp}]</span>
                          <span className={log.type === 'NETTING_ANCHOR' ? 'text-blue-400' : 'text-gray-400'}>
                            {log.description}
                          </span>
                        </div>
                        {log.feeEarned > 0 && <span className="text-green-400">+{log.feeEarned.toFixed(6)}</span>}
                      </motion.div>
                    ))}
                  </AnimatePresence>
                  <div ref={logEndRef} />
                </div>
              </div>

              <button
                onClick={handleStake}
                disabled={isStaking}
                className="w-full bg-green-900/40 hover:bg-green-600 text-green-400 hover:text-black border border-green-600 p-4 rounded-xl font-bold flex justify-center items-center gap-2 transition-all disabled:opacity-50 backdrop-blur-md"
              >
                <Lock className="w-4 h-4" /> {isStaking ? 'ANCHORING...' : 'INITIATE STAKE-TO-ACCESS'}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
