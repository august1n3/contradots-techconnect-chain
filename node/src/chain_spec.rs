use polkadot_sdk::*;

use techconnectchain_runtime as runtime;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::MultiSigner;
use sp_runtime::traits::IdentifyAccount;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;

/// Relay chain to connect to.
pub const RELAY_CHAIN: &str = "rococo-local";

/// ChainSpec extensions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
pub struct Extensions {
    #[serde(alias = "relayChain", alias = "RelayChain")]
    pub relay_chain: String,
    #[serde(alias = "paraId", alias = "ParaId")]
    pub para_id: u32,
}

// --- Helper: derive AccountId from seed --- //
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> runtime::AccountId
where
    MultiSigner: From<TPublic>,
{
    let pair = sr25519::Pair::from_string(&format!("//{}", seed), None)
        .expect("valid dev seed");
    let pub_key = pair.public();
    let signer: MultiSigner = pub_key.into();
    signer.into_account()
}

// --- Core genesis builder (shared) --- //
fn build_genesis(endowed: Vec<runtime::AccountId>, sudo_key: runtime::AccountId) -> runtime::GenesisConfig {
    // Balances vector must be Vec<(AccountId, Balance)>
    let balances: Vec<(runtime::AccountId, runtime::Balance)> = endowed
        .iter()
        .cloned()
        .map(|acct| (acct, 1_000_000_000_000_000u128)) // 1e15 units — adjust as needed
        .collect();

    runtime::GenesisConfig {
        system: runtime::SystemConfig {
            code: runtime::WASM_BINARY.expect("WASM not built").to_vec(),
        },
        timestamp: runtime::TimestampConfig {
            // Optional: set an initial moment
            minimum_period: 0,
        },
        balances: runtime::BalancesConfig {
            balances,
        },
        sudo: runtime::SudoConfig {
            key: Some(sudo_key),
        },
        // Your member registry has no genesis config; default is fine:
        member_registry: runtime::MemberRegistryConfig {},
    }
}

// --- Development chain spec --- //
pub fn development_chain_spec() -> ChainSpec {
    let sudo = get_account_id_from_seed::<sr25519::Public>("Alice");
    let endowed = vec![
        sudo.clone(),
        get_account_id_from_seed::<sr25519::Public>("Bob"),
        get_account_id_from_seed::<sr25519::Public>("Charlie"),
        get_account_id_from_seed::<sr25519::Public>("Dave"),
        get_account_id_from_seed::<sr25519::Public>("Eve"),
        get_account_id_from_seed::<sr25519::Public>("Ferdie"),
    ];

    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "UNIT".into());
    properties.insert("tokenDecimals".into(), 12.into());
    properties.insert("ss58Format".into(), 42.into());

    ChainSpec::builder(
        runtime::WASM_BINARY.expect("WASM not built"),
        Extensions {
            relay_chain: RELAY_CHAIN.into(),
            para_id: runtime::PARACHAIN_ID,
        },
    )
    .with_name("Development")
    .with_id("dev")
    .with_chain_type(ChainType::Development)
    .with_genesis_config(build_genesis(endowed, sudo))
    .with_properties(properties)
    .build()
}

// --- Local testnet chain spec --- //
pub fn local_chain_spec() -> ChainSpec {
    let sudo = get_account_id_from_seed::<sr25519::Public>("Alice");
    let endowed = vec![
        sudo.clone(),
        get_account_id_from_seed::<sr25519::Public>("Bob"),
        get_account_id_from_seed::<sr25519::Public>("Charlie"),
    ];

    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "UNIT".into());
    properties.insert("tokenDecimals".into(), 12.into());
    properties.insert("ss58Format".into(), 42.into());

    ChainSpec::builder(
        runtime::WASM_BINARY.expect("WASM not built"),
        Extensions {
            relay_chain: RELAY_CHAIN.into(),
            para_id: runtime::PARACHAIN_ID,
        },
    )
    .with_name("Local Testnet")
    .with_id("local_testnet")
    .with_chain_type(ChainType::Local)
    .with_genesis_config(build_genesis(endowed, sudo))
    .with_protocol_id("template-local")
    .with_properties(properties)
    .build()
}