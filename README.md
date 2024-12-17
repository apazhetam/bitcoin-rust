# Bitcoin Client Implementation

Welcome!

This is an implementation of a Bitcoin client with full node functionality, using the Rust programming language.

This project is based on the coursework for Principles of Blockchains (Fall 2023) at Princeton University.

## Project Description

The entire implementation for this project is contained in the `src` folder. The steps taken to complete a working Bitcoin client include:

- Implementing the **Address** and **Transaction** structs.
- Implementing the **MerkleTree** struct.
- Implementing the **Block** and **Blockchain** structs.
- Implement a mining module of the bitcoin client. The **miner** module will produce blocks by solving the proof-of-work puzzle.
- Implement the **network** module of Bitcoin client. The network module is in charge of communicating with other nodes/clients. It forms the peer-to-peer (p2p) network and uses gossip protocol to exchange data, including blocks and transactions.
- Combine the previously implemented modules (miner, network, and blockchain) to create a functioning data blockchain. You will need to add PoW validation and a block buffer to handle orphan blocks.
- Implement the **Transaction** struct. Integrate the transaction structure inside the block content, add network functionality to transaction propagation and adding a transaction **Mempool** to be used by the miner to include transaction content in the block being mined.
- Complete the Bitcoin client by maintaining a **State** for the ledger that the blockchain creates, which stores all the required information to check transactions.
- Build an API endpoint to output a representation of the state at a certain block in the longest chain.