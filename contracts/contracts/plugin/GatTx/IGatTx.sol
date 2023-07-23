// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.17;

interface IGatTx {
    event GATSuccess(uint256 nonce, bytes data);
    event PreSetDailyLimit(address[] token, uint256[] limit);
    event CancelSetDailyLimit(address[] token, uint256[] limit);

    function checkData(bytes calldata data) external returns(uint256 timestamp_, bool enabled_);
}
