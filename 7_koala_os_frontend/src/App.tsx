import React, { useMemo } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react';
import { WalletModalProvider } from '@solana/wallet-adapter-react-ui';
import {
  PhantomWalletAdapter,
  SolflareWalletAdapter,
} from '@solana/wallet-adapter-wallets';
import { clusterApiUrl } from '@solana/web3.js';

import '@solana/wallet-adapter-react-ui/styles.css';

import KoalaOSDashboard from '../components/KoalaOS_Dashboard';
import CogFi_MacroRadar from '../components/CogFi_MacroRadar';
import LifePlusWallet from '../components/LifePlusWallet';
import HolographicMap from '../components/HolographicMap';
import HoloGlobe from '../components/HoloGlobe';
import { EarthTwin } from '../components/earth_twin/EarthTwin';

const SOLANA_RPC_ENDPOINT = import.meta.env.VITE_RPC_ENDPOINT ?? clusterApiUrl('mainnet-beta');

export default function App() {
  const wallets = useMemo(
    () => [new PhantomWalletAdapter(), new SolflareWalletAdapter()],
    [],
  );

  return (
    <ConnectionProvider endpoint={SOLANA_RPC_ENDPOINT}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>
          <BrowserRouter>
            <Routes>
              <Route path="/" element={<KoalaOSDashboard />} />
              <Route path="/cogfi/macro-radar" element={<CogFi_MacroRadar />} />
              <Route path="/wallet/life-plus" element={<LifePlusWallet />} />
              <Route path="/koala/holo-globe" element={<HolographicMap />} />
              <Route path="/koala/holo-globe/3d" element={<HoloGlobe />} />
              <Route path="/koala/earth-twin" element={<EarthTwin />} />
              <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
          </BrowserRouter>
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
