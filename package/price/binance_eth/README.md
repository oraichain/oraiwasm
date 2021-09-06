---
iwp: iwp-1
title: CosmWasm IDE - powered py Oraichain
description: A special project made to simplify the development process of wasm smart contracts for Cosmos-based networks, powered by Oraichain.
author: Tu Pham, Duc Pham, Diep Nguyen, Chung Dao, Thao Nguyen.
discussions-to: InterWasm DAO
status: Draft
type: Grant
created: 2021-08-25
---

## Summary

This proposal is created to ask for grants to develop CosmWasm IDE, a hub for developers to develop, deploy and simulate wasm smart contracts for Cosmos SDK based blockchains through CosmWasm. 

## Motivation

Oraichain is currently using the wasmd module to create different decentralized services such as price feed, VRF for the community through the wasm smart contracts. Nevertheless, when developers tried to build and deploy the first smart contract, it took them a considerate amount of time to prepare the working environments to develop and build the contracts. Meanwhile, deploying a wasm file was also a challenging task due to the lack of libraries supporting it. Another problem was the lack of simulation tools for multiple contracts interacting with one another and maintaining their states. Indeed, there was no way to conduct stateful testings, and the only possible option was to deploy the contracts onto a test network, which consumed a great deal of time. Moreover, unit tests simply cannot cover all the cases when we run the contract in production. Because of such reasons, developers must labor through many sporadic and intricate steps in which the outputs are not completely packaged, requiring unfriendly hand-coding through CLI commands.

As a result, we decided to build an application called CosmWasm IDE serving as a quick, convenient and specific set of tools solely for the community to fill the gap between idea and deployment, providing all the tools that developers need to build and test their smart contracts. It solves the developing environment and compatibility issues by offering a seamless environment to write code, build, deploy as well as simulate smart contracts online. Moreover, it also helps reduce the development and testing time, while ensuring the contracts run as intended with no severe or unexpected bugs when in production by using the IDE simulation feature.

## Specification

Use Gitpod template for server-side-development environment. Each user can have access to his or or workspace with fully customized VS Code extensions, terminals and other tools suitable to build and deploy wasm smart contracts.

Integrate Keplr wallet extension to deploy the contracts onto different Cosmos-based networks.

Build a Simulate VS Code extension and install it into the Gitpod template so users can simulate running the contracts as if they have deployed them onto the network.

## Team(Optional)

Tu Pham, Duc Pham, Diep Nguyen, Chung Dao, Thao Nguyen.

## Grant

Expected amount: $35k

Duration: 4 weeks

Form of payment: ATOM
