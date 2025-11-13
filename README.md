# TechConnectChain — Polkadot Product Spec (1 page)

Summary
- TechConnectChain is a parachain on Polkadot designed to manage three student/professional clubs with on-chain membership, verifiable digital credentials (badges/NFTs), DAO-like governance (global + club-scoped), an internal utility token ($TCC) for gamification, and token-gated access to shared resources (GPU time, CTFs, workshops). Runtime-first (FRAME pallets) with XCM-enabled cross‑chain interoperability and Statemint/asset integration.

Primary goals
- Verifiable membership & immutable credentials (soulbound or transferable badges).
- Democratic, auditable decision making: club councils + global referenda.
- Lightweight internal economy ($TCC) to incentivize participation and provide resource access.
- Smooth onboarding: polkadot{.js}/Talisman + optional custodial/social recovery for non-crypto users.
- Parachain with shared security (Polkadot slot) and XCM for asset movement.

Scope / Key features (minimum viable)
- MemberRegistry pallet: account ↔ club affiliations, roles, officer attestations (no PII on-chain).
- Badges pallet: Issue/revoke badges (ERC-721-like via pallet-uniques or custom); support soulbound option; metadata stored on IPFS/Arweave (hash on-chain).
- Token pallet: $TCC as a mapped asset (pallet-assets or ORML multi-currency); treasury, minting authority, basic distribution rules.
- Rewards pallet: rules for awarding $TCC; offchain-worker signed attestations for automated claims.


Non-functional requirements
- Finality: deterministic finality via parachain + relay-chain security; target block time/backing consistent with chosen collator cadence.
- Throughput: modest TPS target (100–1,000) — tune block-size/weights; prioritize deterministic execution and predictable weights.
- Latency: proposal finality windows configurable (e.g., 5–7 days typical for referenda).
- Storage: bounded on-chain data (use BoundedVec, store only metadata hashes); archival nodes optional.
- Privacy: no raw PII on-chain; use hashed pointers to encrypted off-chain blobs; selective disclosure via off-chain ZK proofs as future extension.
- UX: social/custodial onboarding, clear recovery flows, human-readable badge pages.

Tokenomics (starter)
- Symbol: $TCC (internal utility).
- Supply: initial supply 100,000,000 TCC (configurable by governance).
- Allocation (example): Community rewards 50%, Treasury 25%, Ops/Team 15% (vesting), Community Airdrop 10%.
- Sinks: GPU reservations (burn/lock), workshop fees, merch discounts; optional staking for premium access.
- Governance: $TCC does not automatically confer governance votes initially — governance via separate reputation or council tokens to avoid plutocracy (configurable).

Threat model (high-level)
Adversaries
- External: random Sybil actors, bots, griefers, economic attackers (51%/collator bribery), XCM exploiters.
- Internal: malicious club officers, compromised collator keys, buggy runtime upgrade proposals, compromised off-chain attestation service.
Assets at risk
- Token treasury and $TCC supply, badge integrity (fraudulent issuance), reservations/funds locked in pallets, governance capture, member metadata confidentiality.
Attack vectors & mitigations
- Sybil / fake memberships: require officer attestation + optional onboarding KYC out-of-band; rate-limit badge issuance; penalties for false attestations (revocation, reputation slashing).
- Unauthorized badge issuance: permission checks calling MemberRegistry; multisig/officer consensus for high-value badges; on-chain provenance of issuer.
- Reward/claim fraud: require signed off-chain attestations from authorized club officers or cryptographic proofs (offchain-worker + signed receipts); enforce per-epoch limits and replay protection (nonces).
- Treasury theft / collator compromise: treasury behind multisig + timelock (pallet-treasury + pallet-multisig + scheduler), minimum governance quorum, emergency pausable functions.
- Runtime upgrade risks: staged upgrades via governance with canary/test deployments on Rococo; wasm size checks and on-chain governance timelocks to allow human intervention.
- XCM & cross-chain risks: restrict XCM message handlers, whitelisted counterparties, validate asset origin, map assets conservatively; test XCM flows on Rococo thoroughly.
- Smart contract/logic bugs: unit/integration tests, benchmarking for weights, static analysis and external security audits, bug bounty pre-launch.
- Privacy leaks: never store PII plaintext; use content hashes and encryption for off-chain data; disclose selective info via signed proofs only.
Residual risks
- Economic governance capture if token is tradeable and tied to votes — mitigate by separating governance token or using reputation systems and delegation limits.
- Long-range stake attacks are mitigated by Polkadot relay security, but parachain-specific collator/validator compromises are possible; crowdloan/slot governance must be carefully managed.

Operational & compliance notes
- Keep KYC/AML off-chain and store hashes if required; consider legal review for token trading or rewards with monetary value.
- Monitor runtime metrics (failed extrinsics, reservation lock amounts, badge issuance rates) and run on-chain alerts.
- Prepare migration & emergency plans: genesis reconstruction steps, snapshot tools, and public communication templates.

Acceptance criteria (MVP)
- Member registration and officer attestation workflow works end-to-end on Rococo local test.
- Badges minted with IPFS metadata; soulbound enforcement verified.
- $TCC asset created; rewards can be minted by authorized actor; resource reservation using locked tokens functions and refunds tested.
- Club council proposal lifecycle works via pallet-collective; global referenda via pallet-democracy callable.
- SubQuery indexer exposes member list, badge ledger, rewards ledger, and reservations; front-end can display and act on these.

Contact / next deliverable
- This document maps the product constraints to concrete Polkadot runtime choices. I can now produce either (A) concrete pallet function signatures + trait wiring for $TCC issuance & treasury, (B) off‑chain attestation design (message formats, signer flows), or (C) an end-to-end Rococo deployment script for pop-cli. Pick one and I’ll generate the artifacts.