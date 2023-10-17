# Decentralized Random Generator using Pedersen Commitments and Multi-Party Computation

## Abstract

The project introduces a proof of concept to create a decentralized, secure, and tamper-proof random number generation system using Pedersen commitments and multi-party computation. Traditionally, Pedersen commitments are used in commit-reveal schemes, enabling a party to commit to a value without revealing it. The project extends this concept to the decentralized domain, by using a group of non-related peers to collaboratively generate an aggregated commitment for a random value. Leveraging the homomorphic properties of Pedersen commitments, the system calculates an aggregated random value without any participant having to disclose their individual input until the reveal stage.

Decentralized random generators can enable DeFi protocols and their users to participate in yield farming and liquidity provisions without fear of manipulation. By generating random numbers in a decentralized manner, DeFi protocols can distribute rewards, allocate resources, and conduct auctions fairly.

## Project Description

The system involves a network of nodes $`(A_1...A_N)`$ collaborating to create an aggregated commitment for a random value. Each participating node generates a random number $`(R_1...R_N)`$ and a corresponding Pedersen commitment $`(C_1...C_N)`$. These commitments are aggregated into a single commitment $`C_A`$ corresponding to the combined random values $`(R_A = R_1 + ... + R_N)`$, without revealing any individual random value.

## How it Works

A client initiates the process by calling the `commit-random` method to any node of the system, `dealer` or party $`A_i`$, which initiates the random number generation process - generates a random value $`R_i`$ and creates a Pedersen commitment $`C_i`$ to this value. The method then communicates with all the other peer nodes by calling the corresponding `co-commit-random` methods. These co-commitments are combined, ensuring the aggregation of random values, returning a list of $`M`$ co-commitments (where $`M`$ is a threshold of $`2/3`$ of all nodes $`N`$) between its own random value and the value of each node $`C_{ij}`$. The aggregation of these co-commitments is represented as follows:
$$\sum_{j=1, i \ne j}^{M = 2/3 \times N} C_{ij}$$

This scheme introduces a side effect for a `dealer` to overcommitting as its random value is added to each of other nodes' commitments. However, this does not compromise the security of the system as the its subtracted as part of the co-commitment process - to result is the correct commitment corresponding to $`R_A`$. To validate the co-commitment as part of the proof process, a client can reconstruct all the commitments and also subtract the commitment of the `dealer`.

Finally, the `reveal-random` method allows for the unveiling of the original Pedersen opening of all the involved nodes and should be called by the client individually to each of the committing party.

### Functionality

The system is implemented as an HTTP JSON server based on the Axum library, offering three key methods:

1. **commit-random:** This method generates a random number and generates the aggregated Pedersen commitment to the number. It then sends requests to other peer nodes for co-commitment, resulting in an aggregated commitment. The method returns the aggregated commitment for additive operations involving their random value and those of other peers, together with a `commitment_id` for further management.

2. **co-commit-random:** This method supports co-commitment between a received Pedersen commitment and a newly generated number from a peer node. It returns the combined commitment resulting from the collaboration of two commitments.

3. **reveal-random:** This method receives a `commitment_id` and reveals the original Pedersen opening and commitment. The opening includes the secret random value and the blinding factor used to generate the commitment, ensuring transparency and integrity in the random number generation process.

### State Management

A shared state is managed through the Axum state functionality, supported by the *moka::Cache in-memory cache library. `commitment_id` parameter is used for storage and retrieval of corresponding commitments from the cache. The initial value is generated in `commit-random` as a non-related random UUID.

Once the `reveal-random` method is invoked, the commitments associated with the provided `commitment_id` are purged from the cache. Additionally, commitments automatically expire if a client abandons the process.

## Configuration

The project utilizes Docker containers, where each container is configured with appropriate hostname and other essential parameters, all specified in the docker-compose file and can be run on the local machine using `docker-compose up`.

## Code Quality and Testing

Unit tests were added to cover the major happy flows, validating only the fundamental functionalities of the system. Limited tests were added for different levels, including library, routes, and end-to-end scenarios. The testing suite utilized the Mockito mock library and the built-in testing infrastructure of Axum.

Some parts of the code require more attention and object oriented refactoring, to enable more robust testing with dependency injection.

### Running tests

- To run unit tests, run `cargo test`
- For end-to-end testing:
  - first initiate the Docker containers using `docker-compose up`
  - then run e2e tests using the command `cargo test -- --ignored`

## Limitations

### Authentication and Authorization

The system lacks node membership, authentication and authorization mechanisms, leaving it vulnerable to potential man-in-the-middle attacks or malicious manipulation by the `dealer`.

To address this, two mitigating strategies were implemented:

1. The introduction of `nodes` and `node` methods allows clients to retrieve addresses of all nodes, to cross-verify node identities with the addresses returned by `commit-random`.

2. The `reveal-random` method requires client interaction with each node, bypassing the `dealer`, reducing the dealer's ability to compromise reveal and proof process.
