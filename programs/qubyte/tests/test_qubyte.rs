#![cfg(feature = "test-sbf")]

use {
    anchor_lang::{
        prelude::ProgramError, solana_program::instruction::Instruction, AccountDeserialize,
        InstructionData, ToAccountMetas,
    },
    mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk},
    pqcrypto_falcon::falcon512,
    pqcrypto_traits::sign::{DetachedSignature, PublicKey},
    qubyte::{auth, constants::*, error::QubyteError, state::*},
    solana_account::Account,
    solana_pubkey::Pubkey,
    solana_sdk_ids::system_program,
    solana_sha256_hasher::hash,
};

const SOL: u64 = 1_000_000_000;

struct Falcon {
    pk: Vec<u8>,
    sk: falcon512::SecretKey,
}

impl Falcon {
    fn new() -> Self {
        let (pk, sk) = falcon512::keypair();
        Self {
            pk: pk.as_bytes().to_vec(),
            sk,
        }
    }

    fn hash(&self) -> [u8; 32] {
        hash(&self.pk).to_bytes()
    }

    fn sign(&self, message: &[u8; 32]) -> Vec<u8> {
        let sig = falcon512::detached_sign(message, &self.sk);
        let mut out = sig.as_bytes().to_vec();
        out.resize(solana_falcon512::FALCON_512_SIGNATURE_LEN, 0);
        out
    }

    fn identity(&self) -> Pubkey {
        Pubkey::find_program_address(
            &[IDENTITY_SEED, &[IDENTITY_VERSION], &self.hash()],
            &qubyte::ID,
        )
        .0
    }
}

const FOREVER: u64 = u64::MAX;

fn mollusk() -> Mollusk {
    Mollusk::new(&qubyte::ID, "qubyte")
}

fn system_account(lamports: u64) -> Account {
    Account::new(lamports, 0, &system_program::id())
}

fn err(e: QubyteError) -> Check<'static> {
    Check::err(ProgramError::from(anchor_lang::error::Error::from(e)))
}

fn account(result: &mollusk_svm::result::InstructionResult, key: &Pubkey) -> Account {
    result
        .resulting_accounts
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, a)| a.clone())
        .expect("account in result")
}

fn read_identity(account: &Account) -> Identity {
    bytemuck::pod_read_unaligned(&account.data[8..])
}

fn init_identity(mollusk: &Mollusk, falcon: &Falcon) -> Account {
    let payer = Pubkey::new_unique();
    let ix = Instruction::new_with_bytes(
        qubyte::ID,
        &qubyte::instruction::InitIdentity {
            falcon_pubkey: falcon.pk.clone(),
            pubkey_hash: falcon.hash(),
        }
        .data(),
        qubyte::accounts::InitIdentity {
            payer,
            identity: falcon.identity(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    let result = mollusk.process_and_validate_instruction(
        &ix,
        &[
            (payer, system_account(10 * SOL)),
            (falcon.identity(), Account::default()),
            keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
    println!("init_identity CU: {}", result.compute_units_consumed);
    account(&result, &falcon.identity())
}

#[test]
fn init_identity_stores_prepared_key() {
    let mollusk = mollusk();
    let falcon = Falcon::new();
    let identity = read_identity(&init_identity(&mollusk, &falcon));

    assert_eq!(identity.pubkey_hash, falcon.hash());
    assert_eq!(identity.nonce, 0);
    assert_eq!(identity.version, IDENTITY_VERSION);
}

#[test]
fn init_identity_rejects_hash_mismatch() {
    let mollusk = mollusk();
    let falcon = Falcon::new();
    let other = Falcon::new();
    let payer = Pubkey::new_unique();
    let ix = Instruction::new_with_bytes(
        qubyte::ID,
        &qubyte::instruction::InitIdentity {
            falcon_pubkey: falcon.pk.clone(),
            pubkey_hash: other.hash(),
        }
        .data(),
        qubyte::accounts::InitIdentity {
            payer,
            identity: other.identity(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    mollusk.process_and_validate_instruction(
        &ix,
        &[
            (payer, system_account(10 * SOL)),
            (other.identity(), Account::default()),
            keyed_account_for_system_program(),
        ],
        &[err(QubyteError::PubkeyHashMismatch)],
    );
}

const NAME: &str = "Qubyte";
const SYMBOL: &str = "QBT";
const URI: &str = "ipfs://qubyte";

fn pending_launch(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[PENDING_LAUNCH_SEED, mint.as_ref()], &qubyte::ID).0
}

struct Authorize<'a> {
    signer: &'a Falcon,
    signed_mint: Pubkey,
    signed_nonce: u64,
    valid_until_slot: u64,
}

fn authorize_setup(
    falcon: &Falcon,
    identity: &Account,
    mint: Pubkey,
    how: Authorize,
) -> (Instruction, Vec<(Pubkey, Account)>) {
    let payer = Pubkey::new_unique();
    let metadata_hash = auth::metadata_hash(NAME, SYMBOL, URI);
    let signature = how.signer.sign(&auth::launch_message(
        &qubyte::ID,
        &falcon.identity(),
        &how.signed_mint,
        &metadata_hash,
        how.valid_until_slot,
        how.signed_nonce,
    ));
    let ix = Instruction::new_with_bytes(
        qubyte::ID,
        &qubyte::instruction::AuthorizeLaunch {
            metadata_hash,
            valid_until_slot: how.valid_until_slot,
            nonce: how.signed_nonce,
            signature,
        }
        .data(),
        qubyte::accounts::AuthorizeLaunch {
            payer,
            identity: falcon.identity(),
            mint,
            pending_launch: pending_launch(&mint),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    let accounts = vec![
        (payer, system_account(SOL)),
        (falcon.identity(), identity.clone()),
        (mint, Account::default()),
        (pending_launch(&mint), Account::default()),
        keyed_account_for_system_program(),
    ];
    (ix, accounts)
}

fn honest(falcon: &Falcon, mint: Pubkey) -> Authorize<'_> {
    Authorize {
        signer: falcon,
        signed_mint: mint,
        signed_nonce: 0,
        valid_until_slot: FOREVER,
    }
}

#[test]
fn authorize_launch_records_pending_and_bumps_nonce() {
    let mollusk = mollusk();
    let falcon = Falcon::new();
    let identity = init_identity(&mollusk, &falcon);
    let mint = Pubkey::new_unique();
    let (ix, accounts) = authorize_setup(&falcon, &identity, mint, honest(&falcon, mint));

    let result = mollusk.process_and_validate_instruction(&ix, &accounts, &[Check::success()]);
    println!("authorize_launch CU: {}", result.compute_units_consumed);

    let pending =
        PendingLaunch::try_deserialize(&mut &account(&result, &pending_launch(&mint)).data[..])
            .unwrap();
    assert_eq!(pending.creator, falcon.identity());
    assert_eq!(
        pending.metadata_hash,
        auth::metadata_hash(NAME, SYMBOL, URI)
    );
    assert_eq!(
        read_identity(&account(&result, &falcon.identity())).nonce,
        1
    );
}

#[test]
fn authorize_launch_rejects_signature_from_other_key() {
    let mollusk = mollusk();
    let falcon = Falcon::new();
    let identity = init_identity(&mollusk, &falcon);
    let mint = Pubkey::new_unique();
    let impostor = Falcon::new();
    let (ix, accounts) = authorize_setup(&falcon, &identity, mint, honest(&impostor, mint));
    mollusk.process_and_validate_instruction(&ix, &accounts, &[err(QubyteError::InvalidSignature)]);
}

#[test]
fn authorize_launch_rejects_signature_for_other_mint() {
    let mollusk = mollusk();
    let falcon = Falcon::new();
    let identity = init_identity(&mollusk, &falcon);
    let how = Authorize {
        signer: &falcon,
        signed_mint: Pubkey::new_unique(),
        signed_nonce: 0,
        valid_until_slot: FOREVER,
    };
    let (ix, accounts) = authorize_setup(&falcon, &identity, Pubkey::new_unique(), how);
    mollusk.process_and_validate_instruction(&ix, &accounts, &[err(QubyteError::InvalidSignature)]);
}

#[test]
fn authorize_launch_rejects_expired_signature() {
    let mut mollusk = mollusk();
    let falcon = Falcon::new();
    let identity = init_identity(&mollusk, &falcon);
    mollusk.warp_to_slot(1_001);
    let mint = Pubkey::new_unique();
    let how = Authorize {
        signer: &falcon,
        signed_mint: mint,
        signed_nonce: 0,
        valid_until_slot: 1_000,
    };
    let (ix, accounts) = authorize_setup(&falcon, &identity, mint, how);
    mollusk.process_and_validate_instruction(&ix, &accounts, &[err(QubyteError::SignatureExpired)]);
}

#[test]
fn authorize_launch_rejects_replay() {
    let mollusk = mollusk();
    let falcon = Falcon::new();
    let identity = init_identity(&mollusk, &falcon);
    let mint = Pubkey::new_unique();
    let (ix, accounts) = authorize_setup(&falcon, &identity, mint, honest(&falcon, mint));
    let first = mollusk.process_and_validate_instruction(&ix, &accounts, &[Check::success()]);

    let replay: Vec<_> = accounts
        .into_iter()
        .map(|(k, a)| {
            if k == falcon.identity() {
                (k, account(&first, &k))
            } else {
                (k, a)
            }
        })
        .collect();
    mollusk.process_and_validate_instruction(&ix, &replay, &[err(QubyteError::InvalidNonce)]);
}

#[test]
fn launch_rejects_metadata_other_than_signed() {
    let mollusk = mollusk();
    let falcon = Falcon::new();
    let identity = init_identity(&mollusk, &falcon);
    let mint = Pubkey::new_unique();
    let (ix, accounts) = authorize_setup(&falcon, &identity, mint, honest(&falcon, mint));
    let authorized = mollusk.process_and_validate_instruction(&ix, &accounts, &[Check::success()]);

    let payer = Pubkey::new_unique();
    let fee_vault = Pubkey::find_program_address(&[FEE_VAULT_SEED, mint.as_ref()], &qubyte::ID).0;
    let coin_state = Pubkey::find_program_address(&[COIN_STATE_SEED, mint.as_ref()], &qubyte::ID).0;
    let ix = Instruction::new_with_bytes(
        qubyte::ID,
        &qubyte::instruction::Launch {
            name: "Rug".into(),
            symbol: SYMBOL.into(),
            uri: URI.into(),
        }
        .data(),
        qubyte::accounts::Launch {
            payer,
            mint,
            pending_launch: pending_launch(&mint),
            fee_vault,
            coin_state,
            treasury: TREASURY,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    let accounts = vec![
        (payer, system_account(SOL)),
        (mint, Account::default()),
        (
            pending_launch(&mint),
            account(&authorized, &pending_launch(&mint)),
        ),
        (fee_vault, Account::default()),
        (coin_state, Account::default()),
        (TREASURY, system_account(0)),
        keyed_account_for_system_program(),
    ];
    mollusk.process_and_validate_instruction(&ix, &accounts, &[err(QubyteError::MetadataMismatch)]);
}
