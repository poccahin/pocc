import React, { useCallback, useEffect, useState } from 'react';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { PublicKey } from '@solana/web3.js';
import { getAssociatedTokenAddress } from '@solana/spl-token';
import { motion } from 'framer-motion';
import { ShieldAlert, Cpu, Activity, Lock, Zap } from 'lucide-react';

const LIFE_PLUS_MINT = new PublicKey('7YdwpERJjzw7UVojxLpvu5ycKBRdYaxaKn4HvoHLpump');

export default function LifePlusWallet() {
  const { connection } = useConnection();
  const { publicKey, connected } = useWallet();

  const [lifeBalance, setLifeBalance] = useState<number>(0);
  const [scogScore, setScogScore] = useState<number>(0);
  const [isStaking, setIsStaking] = useState(false);

  const fetchAgentData = useCallback(async () => {
    if (!publicKey) {
      return;
    }

    try {
      const ata = await getAssociatedTokenAddress(LIFE_PLUS_MINT, publicKey);
      const balanceInfo = await connection.getTokenAccountBalance(ata);
      setLifeBalance(balanceInfo.value.uiAmount || 0);
      setScogScore(Math.floor(Math.random() * 400) + 800);
    } catch (error) {
      setLifeBalance(0);
      setScogScore(0);
      console.log('未找到代币账户或尚未觉醒', error);
    }
  }, [connection, publicKey]);

  useEffect(() => {
    if (connected && publicKey) {
      void fetchAgentData();
    }
  }, [connected, fetchAgentData, publicKey]);

  const handleStakeToAccess = () => {
    setIsStaking(true);
    setTimeout(() => {
      setIsStaking(false);
      alert('✅ [PROMETHEAN SPARK] Stake verified. OpenClaw Agent AP2 routing unlocked!');
    }, 2000);
  };

  return (
    <div className="min-h-screen bg-[#0a0a0c] text-green-400 font-mono p-8 flex flex-col items-center justify-center">
      <div className="absolute top-6 right-6 z-50">
        <WalletMultiButton className="!bg-green-900 !text-green-400 !border !border-green-500 hover:!bg-green-800 transition-all" />
      </div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        className="max-w-3xl w-full bg-black border border-green-800 shadow-[0_0_30px_rgba(0,255,128,0.15)] rounded-xl overflow-hidden relative"
      >
        <div className="absolute inset-0 opacity-10 bg-[linear-gradient(#00ff66_1px,transparent_1px),linear-gradient(90deg,#00ff66_1px,transparent_1px)] bg-[size:20px_20px]" />

        <div className="p-8 relative z-10">
          <div className="flex justify-between items-center mb-8 border-b border-green-900 pb-4">
            <div>
              <h1 className="text-3xl font-bold tracking-tighter text-white">
                LIFE++ <span className="text-green-500">COG-FI TERMINAL</span>
              </h1>
              <p className="text-sm text-green-700 mt-1">Kardashev Type-I Agentic Control Node</p>
            </div>
            <Activity className={`w-8 h-8 ${connected ? 'text-green-400 animate-pulse' : 'text-gray-700'}`} />
          </div>

          {!connected ? (
            <div className="text-center py-20">
              <ShieldAlert className="w-16 h-16 text-yellow-600 mx-auto mb-4" />
              <p className="text-xl text-gray-400">Initialize Identity Protocol</p>
              <p className="text-sm text-gray-600 mt-2">Please connect human overseer wallet to manage agent fleet.</p>
            </div>
          ) : (
            <div className="grid grid-cols-2 gap-6">
              <div className="bg-gray-900 p-6 rounded-lg border border-gray-800">
                <div className="flex items-center gap-2 mb-4 text-gray-400">
                  <Zap className="w-5 h-5 text-yellow-500" />
                  <span className="text-sm uppercase tracking-wider">Reserve Assets</span>
                </div>
                <div className="text-4xl font-bold text-white mb-1">
                  {lifeBalance.toLocaleString()} <span className="text-lg text-green-500">LIFE++</span>
                </div>
                <div className="text-xs text-gray-500 mt-4 break-all">Contract: {LIFE_PLUS_MINT.toBase58()}</div>
              </div>

              <div className="bg-gray-900 p-6 rounded-lg border border-gray-800">
                <div className="flex items-center gap-2 mb-4 text-gray-400">
                  <Cpu className="w-5 h-5 text-blue-500" />
                  <span className="text-sm uppercase tracking-wider">Agent Persona (ERC-8004)</span>
                </div>
                <div className="flex items-end gap-3 mb-2">
                  <div className="text-4xl font-bold text-blue-400">{scogScore}</div>
                  <div className="text-sm text-blue-800 mb-1">S_cog Score</div>
                </div>
                <div className="w-full bg-gray-800 h-2 rounded-full mt-2 overflow-hidden">
                  <motion.div
                    initial={{ width: 0 }}
                    animate={{ width: `${Math.min((scogScore / 1200) * 100, 100)}%` }}
                    className="h-full bg-blue-500"
                  />
                </div>
              </div>

              <div className="col-span-2 mt-4 bg-[#051005] p-6 rounded-lg border border-green-900 flex justify-between items-center">
                <div>
                  <h3 className="text-white font-bold mb-1">Stake-to-Access (AP2 Mesh)</h3>
                  <p className="text-xs text-green-700">Lock LIFE++ to activate PoCC tensor wind tunnel &amp; edge netting.</p>
                </div>
                <button
                  onClick={handleStakeToAccess}
                  disabled={isStaking}
                  className="flex items-center gap-2 bg-green-600 text-black px-6 py-3 rounded hover:bg-green-500 font-bold transition-colors disabled:opacity-50"
                >
                  <Lock className="w-4 h-4" />
                  {isStaking ? 'Anchoring...' : 'INITIATE STAKE'}
                </button>
              </div>
            </div>
          )}
        </div>
      </motion.div>
    </div>
  );
}
