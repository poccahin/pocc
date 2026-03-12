// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";

/// @title Life++ 硅基未来劳动力产能债券金库 (HK RWA Compliant Vault)
/// @notice 将底层边缘节点与人形机器人集群的未来做功现金流证券化
contract RoboticLaborBondVault is ERC4626, AccessControl, ReentrancyGuard {
    using SafeERC20 for IERC20;

    bytes32 public constant COMPLIANCE_ORACLE_ROLE = keccak256("COMPLIANCE_ORACLE");
    bytes32 public constant NETTING_PROCESSOR_ROLE = keccak256("L3_NETTING_PROCESSOR");

    /// @dev 香港 SFC 框架下的合规投资者白名单 (KYC/AML)
    mapping(address => bool) public isCompliantInvestor;

    /// @dev 最大杠杆率：总资产不得超过历史物理做功价值的 3 倍
    uint256 public constant MAX_LEVERAGE_RATIO = 3;

    /// @dev 历史累积真实做功价值 (USD 记账单位)
    uint256 public historicalPoKWValue;

    event ComplianceStatusUpdated(address indexed investor, bool status);
    event ThermodynamicYieldInjected(uint256 amount, uint256 newPoKWValue, uint256 timestamp);

    error InvestorNotCompliant();
    error ReceiverNotCompliant();
    error LeverageExceedsPoKWBounds();

    constructor(IERC20 baseAsset, string memory name_, string memory symbol_)
        ERC4626(baseAsset)
        ERC20(name_, symbol_)
    {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
    }

    /// @notice 合规预言机更新投资者状态 (接入香港持牌信托或券商 KYC 数据)
    function updateComplianceStatus(address investor, bool status) external onlyRole(COMPLIANCE_ORACLE_ROLE) {
        isCompliantInvestor[investor] = status;
        emit ComplianceStatusUpdated(investor, status);
    }

    /// @notice 重写存款逻辑：执行 KYC 与物理杠杆边界校验
    function deposit(uint256 assets, address receiver) public override nonReentrant returns (uint256 shares) {
        if (!isCompliantInvestor[receiver]) revert ReceiverNotCompliant();

        uint256 projectedTotalAssets = totalAssets() + assets;
        if (projectedTotalAssets > historicalPoKWValue * MAX_LEVERAGE_RATIO) {
            revert LeverageExceedsPoKWBounds();
        }

        shares = super.deposit(assets, receiver);
    }

    /// @notice 重写铸币逻辑：和 deposit 一样执行 KYC 与物理杠杆边界校验
    function mint(uint256 shares, address receiver) public override nonReentrant returns (uint256 assets) {
        if (!isCompliantInvestor[receiver]) revert ReceiverNotCompliant();

        assets = previewMint(shares);
        uint256 projectedTotalAssets = totalAssets() + assets;
        if (projectedTotalAssets > historicalPoKWValue * MAX_LEVERAGE_RATIO) {
            revert LeverageExceedsPoKWBounds();
        }

        assets = super.mint(shares, receiver);
    }

    /// @notice 物理做功收益注入：每日轧差系统将机器人收益打入金库
    /// @dev 只有真实 CTx 与 PoKW 形成的收益可用于份额净值增长
    function injectThermodynamicYield(uint256 yieldAmount, uint256 newPoKWValue)
        external
        onlyRole(NETTING_PROCESSOR_ROLE)
        nonReentrant
    {
        historicalPoKWValue += newPoKWValue;

        IERC20(asset()).safeTransferFrom(msg.sender, address(this), yieldAmount);

        emit ThermodynamicYieldInjected(yieldAmount, newPoKWValue, block.timestamp);
    }

    /// @notice RWA 持有者提取本息
    function withdraw(uint256 assets, address receiver, address owner)
        public
        override
        nonReentrant
        returns (uint256 shares)
    {
        if (!isCompliantInvestor[receiver]) revert ReceiverNotCompliant();
        if (!isCompliantInvestor[owner]) revert InvestorNotCompliant();
        shares = super.withdraw(assets, receiver, owner);
    }

    /// @notice RWA 持有者赎回份额
    function redeem(uint256 shares, address receiver, address owner)
        public
        override
        nonReentrant
        returns (uint256 assets)
    {
        if (!isCompliantInvestor[receiver]) revert ReceiverNotCompliant();
        if (!isCompliantInvestor[owner]) revert InvestorNotCompliant();
        assets = super.redeem(shares, receiver, owner);
    }
}
