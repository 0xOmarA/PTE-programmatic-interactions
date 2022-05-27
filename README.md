# PTE Programmatic Interaction

The repo show cases an example of how programmatic interactions with the Radix PTE can be established through the APIs used in the PTE.

As of v0.4.0 of Scrypto, when transactions are signed, the signing of the transaction looks a little something like `sign(sbor_encode(transaction + nonce), private_key)`. So, what gets signed is the SBOR representation of the transaction instructions plus an added nonce instruction. These get signed using the private key and then either sent off to the PTE to be executed or run locally.

The logic that you see in this repository can be implemented in any other programming language, however, you will also need to implement methods which can perform the SBOR encoding and decoding which is an extra overhead for implementing this in another programing language. Instead, you can use the already existing SBOR libraries provided with Scrypto repository if will be using Rust.

The [main](./src/main.rs) example showcases how you can programmatically create transactions and send them off to the PTE to run and executed. It also showcases what the PTE sends back as a response.