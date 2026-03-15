import { ethers } from 'ethers';

export interface X402Order {
  intentHash: string;
  amount: string;
  paymentAddress: string;
}

export class X402Channel {
  private wallet: ethers.Wallet;

  constructor(privateKey: string) {
    this.wallet = new ethers.Wallet(privateKey);
  }

  getAddress(): string {
    return this.wallet.address;
  }

  async createPaymentProof(order: X402Order): Promise<string> {
    console.log(`⚡ [x402] Generating thermodynamic payment proof for Intent ${order.intentHash}`);

    const payload = ethers.solidityPackedKeccak256(
      ['string', 'uint256', 'address'],
      [order.intentHash, ethers.parseUnits(order.amount, 6), order.paymentAddress],
    );

    return this.wallet.signMessage(ethers.getBytes(payload));
  }

  verifyPayment(order: X402Order, signature: string): boolean {
    const payload = ethers.solidityPackedKeccak256(
      ['string', 'uint256', 'address'],
      [order.intentHash, ethers.parseUnits(order.amount, 6), order.paymentAddress],
    );

    const recoveredAddress = ethers.verifyMessage(ethers.getBytes(payload), signature);
    const isValid = recoveredAddress.toLowerCase() === order.paymentAddress.toLowerCase();

    if (isValid) {
      console.log('✅ [x402] Micro-payment cryptographically secured. Clearing path for tensor execution.');
    } else {
      console.error('💀 [SLASHER WARNING] Invalid x402 signature. Payment rejected.');
    }

    return isValid;
  }
}
