// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.17;

import "../BasePlugin.sol";
import "./IGatTx.sol";
import "../../safeLock/SafeLock.sol";
import "../../interfaces/IExecutionManager.sol";

abstract contract GatTx is BasePlugin, IGatTx, SafeLock {
    //using AddressLinkedList for mapping(address => address);

    uint256 nonce;

    bytes32 constant ENABLE_GAT = keccak256(abi.encodePacked("ENABLE_GAT"));

    constructor(bytes32 initData) SafeLock("PLUGIN_GATTX_SAFELOCK_SLOT", 2 days)  {}

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

    // this plugin is a preHook, meaning executes before the main calldata
    // concept:
    // x bytes used for calldata to be executed, remaining bytes are ignored and can be used for storage data
    // we can there store GAT timestamp uint256 and nonce
    function preHook(address target, uint256 value, bytes calldata data) external view override {
        (target, value, data);

        // extract nonce and timestamp data from the 
        uint256 size;
        assembly { size := sub(calldatasize(), 56) }
        bytes memory data_ = data[size:];

        if (data_.length >= 64) {

            // Parse the last 96 bytes into three variables
            bytes32 key_;
            //uint256 nonce_;
            uint256 timestamp_;
            assembly {
                let dataEnd := add(data_, mload(data_))
                timestamp_ := mload(sub(dataEnd, 32))
                //nonce_ := mload(sub(dataEnd, 64))
                key_ := mload(sub(dataEnd, 64))
            }

            // Check if the first one matches the constant keccak256("ENABLE_GAT")
            // by adding the key check users can enable/disable this hook feature in their userop tx
            if(key_ == ENABLE_GAT) {
                require(block.timestamp < timestamp_, "GatTx: Execution too early");
            }
        }
    }

    // check to extra and analyze data to check if failed userop failed caused by the this Hook or normal operations
    function checkData(bytes calldata data) external pure override returns(uint256 timestamp_, bool enabled_) {
        bytes memory data_ = data;
        if (data_.length >= 64) {
            bytes32 key_;
            assembly {
                let dataEnd := add(data_, mload(data_))
                timestamp_ := mload(sub(data_, 32))
                key_ := mload(sub(dataEnd, 64))
            }

            // Check key matches enabled state
            enabled_ = key_ == ENABLE_GAT;
        }
    }

    // postHook kees a history of our success
    function postHook(address target, uint256 value, bytes calldata data) external override {
        (target, value, data);
        nonce++;
        emit GATSuccess(nonce, data);
        //revert("GatTx: preHook not support");
    }


}