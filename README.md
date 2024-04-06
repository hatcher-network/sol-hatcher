# Program for Sol-Hatcher

Sol-Hatcher: Creator-User-Judge Tripartite Evolutionary AI App Training Platform

## Setup

### 1. Dump metadata program from mainnet. 
```shell
solana program dump -u m metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s tests/metadata.so
```

The config of the program account itself has been added to Anchor.toml as 
```
[[test.genesis]]
address = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"  
program = "tests/metadata.so"
```
### 2. Build and test

run
```shell
anchor test
```