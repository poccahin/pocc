import { ethers } from 'ethers';
import IdentityRegistryABI from '../contracts/abi/IdentityRegistry.json';

const REGISTRY_ADDRESS = '0x8004A169FB4a3325136EB29fA0ceB6D2e539a432';

export class IdentityClient {
  private contract: ethers.Contract;
  private wallet: ethers.Wallet;

  constructor(
    privateKey: string,
    rpcUrl: string = 'https://rpc.testnet3.goat.network',
  ) {
    const provider = new ethers.JsonRpcProvider(rpcUrl);
    this.wallet = new ethers.Wallet(privateKey, provider);
    this.contract = new ethers.Contract(REGISTRY_ADDRESS, IdentityRegistryABI, this.wallet);
  }

  async bootstrapIdentity(domainPrefix: string, capabilitiesURI: string): Promise<string> {
    console.log(`🌌 [ERC-8004] Initiating Genesis Registration for ${domainPrefix}...`);

    try {
      const tx = await this.contract.register(domainPrefix, capabilitiesURI, this.wallet.address);
      console.log(`⏳ [ERC-8004] Transaction broadcasted: ${tx.hash}. Waiting for crystallization...`);

      const receipt = await tx.wait();
      const parsedEvent = receipt.logs
        .map((log: ethers.Log) => {
          try {
            return this.contract.interface.parseLog(log);
          } catch {
            return null;
          }
        })
        .find((parsed: ethers.LogDescription | null) => parsed?.name === 'Registered');

      const agentId = parsedEvent?.args?.[0]?.toString();
      if (!agentId) {
        throw new Error('Registered event was not found in transaction receipt logs.');
      }

      console.log(`✅ [ERC-8004] Entity awakened. Agent ID: ${agentId}, DID: ${domainPrefix}.ahin.io`);
      return agentId;
    } catch (error) {
      console.error('💥 [ERC-8004 FATAL] Identity registration failed:', error);
      throw error;
    }
  }
}
