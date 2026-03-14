import React, { FC, useMemo } from 'react';
import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react';
import { WalletModalProvider } from '@solana/wallet-adapter-react-ui';
import { clusterApiUrl } from '@solana/web3.js';

import {
  koalaDashboardRoute,
  cogFiRadarRoute,
  lifePlusWalletRoute,
  holoGlobeRoute,
  holographicMapRoute,
  earthTwinRoute,
} from './routes';

const router = createBrowserRouter([
  koalaDashboardRoute,
  cogFiRadarRoute,
  lifePlusWalletRoute,
  holoGlobeRoute,
  holographicMapRoute,
  earthTwinRoute,
]);

const App: FC = () => {
  const endpoint = useMemo(() => clusterApiUrl('mainnet-beta'), []);

  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={[]} autoConnect>
        <WalletModalProvider>
          <RouterProvider router={router} />
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
};

export default App;
