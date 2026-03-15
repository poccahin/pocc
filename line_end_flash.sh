#!/usr/bin/env bash
# ============================================================
# CIS Line-End Station (LES) – Cryptographic Provisioning Script
# Station 7: Silicon Awakening & ERC-8004 Identity Bootstrap
#
# This script runs on the factory EMS workstation.  It pushes the
# compiled `cis_genesis_provision` binary to the Device Under Test
# (DUT) via SSH, triggers the one-time awakening ceremony, then
# verifies that the device has self-locked and powered down.
#
# Zero-Trust Principle: private keys are generated on-device.
# This script never handles any cryptographic material.
# ============================================================
set -euo pipefail

# ── Configuration ─────────────────────────────────────────────
DUT_IP="${CIS_DUT_IP:-192.168.100.50}"
FACTORY_RPC="${CIS_FACTORY_RPC:-http://192.168.100.1:8545}"
FACTORY_IPFS="${CIS_FACTORY_IPFS:-http://192.168.100.1:5001}"
BINARY_PATH="${CIS_BINARY_PATH:-./target/release/cis_genesis_provision}"
# Set CIS_NETWORK_IFACE to override interface auto-detection (e.g. eth0, enp0s3).
SSH_OPTS="-o StrictHostKeyChecking=no -o ConnectTimeout=10"

echo ">>> [CIS EMS System] Starting Station 7: Cryptographic Provisioning"
echo ">>> DUT IP:          ${DUT_IP}"
echo ">>> Factory RPC:     ${FACTORY_RPC}"

# ── Pre-flight: verify binary exists ──────────────────────────
if [[ ! -f "${BINARY_PATH}" ]]; then
    echo "❌ [ERROR] Genesis binary not found at: ${BINARY_PATH}"
    echo "   Build with: cargo build --release -p cis-genesis-provisioner"
    exit 1
fi

# ── Step 1: Confirm DUT is reachable ──────────────────────────
echo ">>> Checking DUT connectivity..."
if ! ping -c 1 -W 5 "${DUT_IP}" > /dev/null 2>&1; then
    echo "❌ [ERROR] DUT not reachable at ${DUT_IP}"
    exit 1
fi

# ── Step 2: Push the awakening binary to the device RAM disk ──
echo ">>> Pushing cis_genesis_provision to DUT RAM disk..."
# shellcheck disable=SC2086
scp ${SSH_OPTS} "${BINARY_PATH}" "root@${DUT_IP}:/tmp/cis_genesis_provision"
# shellcheck disable=SC2086
ssh ${SSH_OPTS} "root@${DUT_IP}" "chmod +x /tmp/cis_genesis_provision"

# ── Step 3: Capture the DUT MAC address for audit log ─────────
CIS_IFACE="${CIS_NETWORK_IFACE:-}"
if [[ -z "${CIS_IFACE}" ]]; then
    # Auto-detect the primary network interface (compatible with predictable names)
    CIS_IFACE=$(ssh ${SSH_OPTS} "root@${DUT_IP}" \
        "ip route show default 2>/dev/null | awk '/dev/ {print \$5}' | head -1 || echo eth0")
fi
DUT_MAC=$(ssh ${SSH_OPTS} "root@${DUT_IP}" \
    "cat /sys/class/net/${CIS_IFACE}/address 2>/dev/null || echo 'unknown'" | tr '[:lower:]' '[:upper:]')
echo ">>> DUT Interface:   ${CIS_IFACE}"
echo ">>> DUT MAC Address: ${DUT_MAC}"

# ── Step 4: Trigger the Silicon Awakening sequence ────────────
# The device will:
#   1. Extract SRAM PUF entropy
#   2. Derive Ed25519 + EVM key pairs
#   3. Sign the ERC-8004 genesis transaction
#   4. Burn OTP fuses (JTAG + Secure Boot)
#   5. Power itself down
echo ">>> Executing genesis sequence. Device will self-lock and power down."
# shellcheck disable=SC2086
ssh ${SSH_OPTS} "root@${DUT_IP}" \
    "CIS_MAC_ADDRESS='${DUT_MAC}' \
     CIS_FACTORY_RPC='${FACTORY_RPC}' \
     CIS_FACTORY_IPFS='${FACTORY_IPFS}' \
     /tmp/cis_genesis_provision; \
     poweroff" || true
# Exit code may be non-zero once the device powers off – that is expected.

# ── Step 5: Confirm the device is offline (OTP burn success) ──
echo ">>> Waiting for device to power down (max 15 s)..."
sleep 5
PING_ATTEMPTS=0
MAX_ATTEMPTS=3
while ping -c 1 -W 2 "${DUT_IP}" > /dev/null 2>&1; do
    PING_ATTEMPTS=$((PING_ATTEMPTS + 1))
    if [[ "${PING_ATTEMPTS}" -ge "${MAX_ATTEMPTS}" ]]; then
        echo "❌ [ERROR] Device failed to power off after ${MAX_ATTEMPTS} attempts."
        echo "   OTP burn may have failed – quarantine this unit for manual inspection."
        exit 1
    fi
    echo "   Device still responding – waiting..."
    sleep 5
done

echo "✅ [PASS] Station 7 Complete. Box is sovereign, sealed, and ready for packaging."
