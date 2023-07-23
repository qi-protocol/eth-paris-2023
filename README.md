# eth-paris-2023

## Features
* [Good-After-Time](https://docs.cow.fi/tutorials/how-to-place-erc-1271-smart-contract-orders/good-after-time-gat-orders) order support for 4337 smart wallets
* MEV-rebate for smart wallets through a [feature-enriched bundler](https://github.com/qi-protocol/eth-paris-2023/blob/main/baby_bundler/src/bundler/bundler.rs)

## Purpose:

To create plugin / extension for account abstraction wallets with such a feature enabled. For our purposes we will create an example plugin for Good After Time execution.

As a bonus this extension can be selectively activated!

## Background - How do they work?

### Bundler
Run `cargo run` to start up the bundler at `127.0.0.1:3000`

Run `cargo test` to populate and send the `UserOperation` that swap ETH for USDC on UniswapV2(see how to populate a `UserOperation` using [Alloy](https://github.com/alloy-rs/core) [here](https://github.com/qi-protocol/eth-paris-2023/blob/e5ec66687b4ca6fea87f7cfa662d5cfa2eec76f7/baby_bundler/src/main.rs#L99))

TODO: Explanation


### GAT Order Plug-in

Plugins in this context function as an execution call before or after the main calldata execution. There likely is a method to add plugins to an existing wallet via modules, but for now this can be ignored and adding plugins only refers to adding during deployment.

```solidity
function initialize(
        address anOwner,
        address defalutCallbackHandler,
        bytes[] calldata modules,
        bytes[] calldata plugins
    ) external initializer {
        _addOwner(anOwner);
        _setFallbackHandler(defalutCallbackHandler);

        for (uint256 i = 0; i < modules.length;) {
            _addModule(modules[i]);
            unchecked {
                i++;
            }
        }
        for (uint256 i = 0; i < plugins.length;) {
            _addPlugin(plugins[i]);
            unchecked {
                i++;
            }
        }
    }
```

The modules when set take the address where the module has been deployed and any initialization data for that plugin. The function `aPlugin.supportsHook();` will return is the plugin is to be executed before of after the executed calldata. Plugins can be thought of as a modifier wrapper for all transactions.

When calldata is executed, the call will execute the test all plugins that wallet has enabled: 

```solidity
function _call(address target, uint256 value, bytes memory data) private executeHook(target, value, data) {
      assembly ("memory-safe") {
          let result := call(gas(), target, value, add(data, 0x20), mload(data), 0, 0)
          if iszero(result) {
              let ptr := mload(0x40)
              returndatacopy(ptr, 0, returndatasize())
              revert(ptr, returndatasize())
          }
      }
  }
```

### What’s the issue with intenful txs?

Intentful txs in a nutshell is invalid calldata with a valid premise. The premise being the conditions it which the calldata is valid. Execution will be continuously tested until valid or a deadline has passed.

Although, when the intent is sufficiently simple, the execution will already revert without the need of an intent. This means such test are good for a unit testing but bad in practice. 

For example, say I wanted to perform a limit order on Uniswap. Uniswap by default has a field minAmountOut, thus satisfying the limit order. This means the plugin test will happen along side the minAmountOut test, making the intent redundant.

Does this nullify the use of intents? No, we can think of various simple practical usecases where the intent is only ever so slightly more complicated. For example a stop loss limit order, the intent must be smart enough to pick which execution must be done given the current market parameters.

---

Prompt:

- “I want to swap X for Y, but if Y’s price is over A, the transaction will revert, hence fail during the verification stage”

From an AA perspective the tx flow is
userop > entrypoint > wallet > intent factory? > intent contract > swap router

For the purposed of our project, single execution only!

```solidity
// EntryPoint executes the userop in the form
Exec.call(mUserOp.sender, 0, callData, callGasLimit);

// which results in the call
function call(
    address to,
    uint256 value,
    bytes memory data,
    uint256 txGas
) internal returns (bool success) {
    assembly {
        success := call(txGas, to, value, add(data, 0x20), mload(data), 0, 0)
    }
}
```

```solidity
// calldata is formed on the wallet in the form:
function execute(
	address dest, 
	uint256 value, 
	bytes calldata func
) external override onlyEntryPoint {
        _call(dest, value, func);
}

// which calls
function _call(
	address target, 
	uint256 value, 
	bytes memory data
) private executeHook(target, value, data) {
    assembly ("memory-safe") {
      let result := call(gas(), target, value, add(data, 0x20), mload(data), 0, 0)
      if iszero(result) {
          let ptr := mload(0x40)
          returndatacopy(ptr, 0, returndatasize())
          revert(ptr, returndatasize())
      }
    }
}
```

Given the that our op calldata executes directly from the perspective of our wallet, the data is of the form:

[20:] is target
[20:52] is the value of the call
[52:] is the data to make the external call

To simplify the data the call will be specifically be the `swapExactEthForTokens` function on `uniswapV2Rounter02.sol` . And the call will trade 0.001 ETH for TEST tokens.

```
0x7ff36ab500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000080000000000000000000000000[user aa wallet]00000000000000000000000000000000000000000000000000000000669e545500000000000000000000000000000000000000000000000000000000000000020000000000000000000000007b79995e5f793a07bc00c21412e50ecae098e7f9000000000000000000000000ae0086b0f700d6d7d4814c4ba1e55d3bc0dfee02
```

0.001 ETH:

00000000000000000000000000000000000000000000000000038D7EA4C68000

This means that our full `calldata` field of our userop will have the full form of:

```
0xC532a74256D3Db42D0Bf7a0400fEFDbad769400800000000000000000000000000000000000000000000000000000000000000000000000000038D7EA4C680007ff36ab500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000080000000000000000000000000[user aa wallet]00000000000000000000000000000000000000000000000000000000669e545500000000000000000000000000000000000000000000000000000000000000020000000000000000000000007b79995e5f793a07bc00c21412e50ecae098e7f9000000000000000000000000ae0086b0f700d6d7d4814c4ba1e55d3bc0dfee02
```

To ensure the userop has sufficient gas the op will assume a value 500000 gas for execution and validation (overkill).

---

Now that the calldata of the userop is in the correct format the plugin data needs to be appended to the end of our execution. The form is an additonal key-value to enable the extension execution and a “good after” timestamp.

```solidity
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
```

### Conclusion:

The structure of the plugin contract can be easily modified to fit any execution style whether that be a 4337 plugin or retrofitted to be executed to be used with Gnosis SAFE. And better yet, this is chain agnostic.

---

### UserOp Data (will be changed to fit new structure):

Sender (created via SimpleAccountFactory):
0x9406Cc6185a346906296840746125a0E44976454

Calldata (uniswapv2 Router swapExectEthForTokens):
0x7ff36ab5000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000800000000000000000000000009406Cc6185a346906296840746125a0E4497645400000000000000000000000000000000000000000000000000000000669e545500000000000000000000000000000000000000000000000000000000000000020000000000000000000000007b79995e5f793a07bc00c21412e50ecae098e7f9000000000000000000000000ae0086b0f700d6d7d4814c4ba1e55d3bc0dfee02

---

### Important Addresses (sepolia):

WETH:
0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9

TEST Token:
0xAe0086B0f700d6d7d4814c4Ba1e55d3BC0dFEe02

EntryPoint.sol:
0x0576a174D229E3cFA37253523E645A78A0C91B57

SimpleAccountFactory:
0x9406Cc6185a346906296840746125a0E44976454

UniswapV2Router:
0xC532a74256D3Db42D0Bf7a0400fEFDbad7694008

UniswapV2Factory:
0x7E0987E5b3a30e3f2828572Bb659A548460a3003

Celo Deployment:
0x5E08D920B38a62a779eb1748564f8776Bb55bb0A

Sepolia via Quicknode RPC (verified):
0x453F47921f489117EFF6E10BF0BB68176506ebfe

Polygon:
0x5E08D920B38a62a779eb1748564f8776Bb55bb0A
