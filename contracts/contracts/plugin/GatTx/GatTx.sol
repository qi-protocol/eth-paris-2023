// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.17;

import "../BasePlugin.sol";
import "./IGatTx.sol";
import "../../safeLock/SafeLock.sol";
import "../../libraries/AddressLinkedList.sol";
import "../../libraries/SignatureDecoder.sol";
import "@account-abstraction/contracts/core/Helpers.sol";
import "../../interfaces/IExecutionManager.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "../../interfaces/IPluginStorage.sol";

contract GatTx is BasePlugin, IGatTx, SafeLock {
    //using AddressLinkedList for mapping(address => address);

    uint256 nonce;

    mapping(uint256 => bytes) private _calldata;

    constructor() SafeLock("PLUGIN_DAILYLIMIT_SAFELOCK_SLOT", 2 days) {}

    struct Layout {
        uint256 nonce;
    }

    function _supportsHook() internal pure override returns (uint8 hookType) {
        hookType = PRE_HOOK;
    }

    function inited(address wallet) internal view virtual override returns (bool);

    // we don't need to initalize any data
    function _init(bytes calldata data) internal virtual override;

    function _deInit() internal virtual override;

    function _calcRequiredPrefund(UserOperation calldata userOp) private pure returns (uint256 requiredPrefund) {
        uint256 requiredGas = userOp.callGasLimit + userOp.verificationGasLimit + userOp.preVerificationGas;
        requiredPrefund = requiredGas * userOp.maxFeePerGas;
    }

    function preHook(address target, uint256 value, bytes calldata data) external pure override {
        (target, value, data);
        revert("GatTx: preHook not support");
    }

    function postHook(address target, uint256 value, bytes calldata data) external override {
        (target, value, data);
        revert("GatTx: preHook not support");
    }


}


// each tx has a nonce 
// each tx increments nonce
// each tx has a Good After Timestamp value