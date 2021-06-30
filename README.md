# Simple Solana Dapp Dice Game
Based on another project. 


## Quick Start

The following dependencies are required to build and run this example,
depending on your OS, they may already be installed:

```bash
$ node --version
$ npm --version
$ docker -v
$ wget --version
$ rustup --version
$ rustc --version
$ cargo --version
```

If this is your first time using Docker or Rust, these [Installation Notes](README-installation-notes.md) might be helpful.

If not already, install (current version of) Solana CLI 

```
sh -c "$(curl -sSfL https://release.solana.com/v1.6.1/install)"
```

This code is designed to be used with the tutorial, but if you just want to 'make it do something':

```
cd solana-dice-game
npm install
npm run cluster_devnet
npm run keypair
npm run airdrop
npm run build_advdice
npm run deploy_advdice
npm run dice_advance -- 1 30 10
```



