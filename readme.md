This tool helps you convert snarkjs outputs (verification keys, proofs) to Aptos representations.

## How to use
Clone this repo and cd into the repo root.

Say your BN254-based Groth16 verification key file is at `/path/to/vk.json`, and the corresponding proof at `path/to/proof.json`.
Run the following command, and you should see an example Move module generated at `./example_1`.
```bash
IN_VK_PATH=/path/to/vk.json IN_PROOF_PATH=/path/to/proof.json OUT_DIR=./example_1 cargo run
```

The example module should use contains a test that successfully verifies your Groth16 proof and your Groth16 verification key, if they match.
```bash
cd example_1
aptos move test
```
