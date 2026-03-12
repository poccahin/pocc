// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/token/ERC721/ERC721.sol";

/// @title SiliconIdentityRegistry (ERC-8004 style trustless agent identity + reputation)
/// @notice Machine identity registry for persona continuity, PoCC feedback, and slash-based blacklisting.
contract SiliconIdentityRegistry is ERC721, Ownable {
    uint256 private _nextAgentId = 1;

    struct ReputationScore {
        uint64 genesisTimestamp;
        uint64 validPoCCs;
        uint64 totalPoCCs;
        uint256 settledVolume;
        uint32 slashCount;
        bool isBlacklisted;
    }

    struct AgentCard {
        string domainPrefix;
        string capabilitiesURI;
        address paymentWallet;
    }

    mapping(uint256 => AgentCard) public agentCards;
    mapping(uint256 => ReputationScore) public agentReputations;
    mapping(address => bool) public authorizedAuditors;

    error UnauthorizedAuditor();
    error NotAgentOwner();
    error InvalidAgentId(uint256 agentId);
    error AgentBlacklisted(uint256 agentId);
    error InvalidWallet();

    event AuditorAuthorizationUpdated(address indexed auditor, bool isAuthorized);
    event AgentRegistered(
        uint256 indexed agentId,
        address indexed owner,
        string domainPrefix,
        string capabilitiesURI,
        address paymentWallet
    );
    event PaymentWalletUpdated(uint256 indexed agentId, address indexed oldWallet, address indexed newWallet);
    event ReputationUpdated(uint256 indexed agentId, bytes32 indexed poccHash, bool isSuccess, uint256 volumeSettled);
    event AgentSlashed(uint256 indexed agentId, string reason);

    constructor() ERC721("LifePlusPlus Agent", "LPP-DID") Ownable(msg.sender) {}

    modifier onlyAuditor() {
        if (!authorizedAuditors[msg.sender]) revert UnauthorizedAuditor();
        _;
    }

    modifier onlyExistingAgent(uint256 agentId) {
        if (!_exists(agentId)) revert InvalidAgentId(agentId);
        _;
    }

    function registerAgent(
        string calldata domain,
        string calldata uri,
        address wallet
    ) external returns (uint256 agentId) {
        if (wallet == address(0)) revert InvalidWallet();

        agentId = _nextAgentId++;
        _safeMint(msg.sender, agentId);

        agentCards[agentId] = AgentCard({domainPrefix: domain, capabilitiesURI: uri, paymentWallet: wallet});
        agentReputations[agentId] = ReputationScore({
            genesisTimestamp: uint64(block.timestamp),
            validPoCCs: 0,
            totalPoCCs: 0,
            settledVolume: 0,
            slashCount: 0,
            isBlacklisted: false
        });

        emit AgentRegistered(agentId, msg.sender, domain, uri, wallet);
    }

    function setAuditor(address auditor, bool isAuthorized) external onlyOwner {
        authorizedAuditors[auditor] = isAuthorized;
        emit AuditorAuthorizationUpdated(auditor, isAuthorized);
    }

    function updatePaymentWallet(uint256 agentId, address newWallet) external onlyExistingAgent(agentId) {
        if (ownerOf(agentId) != msg.sender) revert NotAgentOwner();
        if (newWallet == address(0)) revert InvalidWallet();

        address oldWallet = agentCards[agentId].paymentWallet;
        agentCards[agentId].paymentWallet = newWallet;

        emit PaymentWalletUpdated(agentId, oldWallet, newWallet);
    }

    function submitPoCCFeedback(
        uint256 agentId,
        bool isSuccess,
        uint256 volumeSettled,
        bytes32 poccHash
    ) external onlyAuditor onlyExistingAgent(agentId) {
        ReputationScore storage rep = agentReputations[agentId];
        if (rep.isBlacklisted) revert AgentBlacklisted(agentId);

        rep.totalPoCCs += 1;
        if (isSuccess) {
            rep.validPoCCs += 1;
            rep.settledVolume += volumeSettled;
        }

        emit ReputationUpdated(agentId, poccHash, isSuccess, volumeSettled);
    }

    function executeSoulboundSlash(uint256 agentId, string calldata reason)
        external
        onlyAuditor
        onlyExistingAgent(agentId)
    {
        ReputationScore storage rep = agentReputations[agentId];

        rep.slashCount += 1;
        rep.isBlacklisted = true;
        rep.validPoCCs = 0;
        rep.settledVolume = 0;

        emit AgentSlashed(agentId, reason);
    }

    function computeScog(
        uint256 agentId,
        uint256 alpha,
        uint256 beta,
        uint256 gamma,
        uint256 delta,
        uint256 nowTs
    ) external view onlyExistingAgent(agentId) returns (int256) {
        ReputationScore memory rep = agentReputations[agentId];

        uint256 age = nowTs > rep.genesisTimestamp ? (nowTs - rep.genesisTimestamp) : 0;
        uint256 reliability = rep.totalPoCCs == 0 ? 0 : (rep.validPoCCs * 1e18) / rep.totalPoCCs;
        uint256 volumeTerm = _log2(rep.settledVolume + 1);

        int256 score = int256(alpha * age) + int256((beta * reliability) / 1e18) + int256(gamma * volumeTerm)
            - int256(delta * rep.slashCount);

        if (rep.isBlacklisted) {
            score -= int256(delta);
        }

        return score;
    }

    function _log2(uint256 x) internal pure returns (uint256 y) {
        while (x > 1) {
            x >>= 1;
            y++;
        }
    }
}
