use radix_engine::model::{SignedTransaction, Instruction};
use radix_engine::transaction::TransactionBuilder;
use scrypto::prelude::*;

// Used to handle the JSON serialization and deserialization
use serde::{Deserialize, Serialize};

// Used for quick Nonce generation
use rand::Rng;

// Contains the code required to go from a Transaction -> Transaction manifest string
mod utils;
use utils::{DecompileError, decompile}; 

fn main() {
    // Here is a sample key-pair which you can use to do some quick testing if you would like :)
    let private_key: EcdsaPrivateKey = EcdsaPrivateKey::from_bytes(&[124,159,161,54,212,65,63,166,23,54,55,232,131,182,153,141,50,225,214,117,248,140,221,255,157,203,207,51,24,32,244,184]).unwrap();
    let public_key: EcdsaPublicKey = private_key.public_key();

    // Building a sample transaction to create a new account for the above key-pair
    let withdraw_auth: AccessRule = rule!(require(NonFungibleAddress::new(
        ECDSA_TOKEN,
        NonFungibleId::from_bytes(public_key.to_vec())
    )));
    let account_creation_nonce: u64 = rand::thread_rng().gen_range(0..100);
    let account_creation_tx: SignedTransaction = TransactionBuilder::new()
        .call_method(SYSTEM_COMPONENT, "free_xrd", vec![])
        .take_from_worktop(RADIX_TOKEN, |builder, bucket_id| {
            builder.new_account_with_resource(&withdraw_auth, bucket_id)
        })
        .build(account_creation_nonce)
        .sign([&private_key]);

    let account_creation_receipt: Receipt = submit_transaction(&account_creation_tx).unwrap();
    let account_component_address: ComponentAddress = account_creation_receipt.new_components()[0];
    println!("Account {} was created, receipt is: {:?}", account_component_address, account_creation_receipt);

    // A sample transaction where we withdraw some XRD from the account we just created and deposit them into another
    // account in the PTE.
    let xrd_transfer_nonce: u64 = rand::thread_rng().gen_range(0..100);
    let xrd_transfer_tx: SignedTransaction = TransactionBuilder::new()
        .withdraw_from_account_by_amount(dec!("10000"), RADIX_TOKEN, account_component_address)
        .take_from_worktop(RADIX_TOKEN, |builder, bucket_id| {
            builder.call_method(
                ComponentAddress::from_str("02c1d7add487dbcbb8c81da378aa8d4924d9844874d1cc3829a173").unwrap(),
                "deposit",
                args![scrypto::resource::Bucket(bucket_id)]
            )
        })
        .build(xrd_transfer_nonce)
        .sign([&private_key]);

    let xrd_transfer_receipt: Receipt = submit_transaction(&xrd_transfer_tx).unwrap();
    println!("XRD has been transferred, receipt is: {:?}", xrd_transfer_receipt);
}

// =====================================================================================================================
// Additional code required to support the above function
// =====================================================================================================================


/// Submits the transaction to the PTE01 server.
pub fn submit_transaction(transaction: &SignedTransaction) -> Result<Receipt, TransactionSubmissionError> {
    // Getting the nonce used in the transaction from the transaction object itself
    let nonce: u64 = {
        let nonce_instructions: Vec<Instruction> = transaction.transaction.instructions
            .iter()
            .filter(|x| {
                match x {
                    Instruction::Nonce { nonce: _ } => true,
                    _ => false
                }
            })
            .cloned()
            .collect();

        if nonce_instructions.len() == 0 {
            Err(TransactionSubmissionError::NoNonceFound)
        } 
        else if nonce_instructions.len() == 1{ 
            if let Instruction::Nonce { nonce } = nonce_instructions[0] {
                Ok(nonce)
            } else {
                panic!("Expected a nonce");
            }
        } 
        else {
            Err(TransactionSubmissionError::MultipleNonceFound)
        }
    }?;
    let nonce: Nonce = Nonce { value: nonce };

    let signatures: Vec<Signature> = transaction.signatures
        .iter()
        .map(|x| Signature{
            public_key: x.0.to_string(), 
            signature: x.1.to_string()
        })
        .collect();

    // Creating the transaction body object which is what will be submitted to the PTE
    let transaction_body: TransactionBody = TransactionBody {
        manifest: decompile(&transaction.transaction)?,
        nonce: nonce,
        signatures: signatures
    };

    // Submitting the transaction to the PTE's `/transaction` endpoint
    let receipt: Receipt = reqwest::blocking::Client::new()
        .post("https://pte01.radixdlt.com/transaction")
        .json(&transaction_body)
        .send()?
        .json()?;

    return Ok(receipt);
}

/// A struct which describes the Nonce. Required for the TransactionBody struct
#[derive(Serialize, Deserialize, Debug)]
pub struct Nonce {
    value: u64,
}

/// A struct which defines the signature used in the TransactionBody struct.
#[derive(Serialize, Deserialize, Debug)]
pub struct Signature {
    public_key: String,
    signature: String,
}

/// A struct which defines the transaction payload that the PTE's API accepts.
#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionBody {
    manifest: String,
    nonce: Nonce,
    signatures: Vec<Signature>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Receipt {
    pub transaction_hash: String,
    pub status: String,
    pub outputs: Vec<String>,
    pub logs: Vec<String>,
    pub new_packages: Vec<String>,
    pub new_components: Vec<String>,
    pub new_resources: Vec<String>,
}

impl Receipt {
    pub fn new_packages(&self) -> Vec<PackageAddress> {
        return self.new_packages
            .iter()
            .map(|x| PackageAddress::from_str(x).unwrap())
            .collect()
    }
    
    pub fn new_components(&self) -> Vec<ComponentAddress> {
        return self.new_components
            .iter()
            .map(|x| ComponentAddress::from_str(x).unwrap())
            .collect()
    }
    
    pub fn new_resources(&self) -> Vec<ResourceAddress> {
        return self.new_resources
            .iter()
            .map(|x| ResourceAddress::from_str(x).unwrap())
            .collect()
    }
}

/// An enum of the errors which could occur when submitting a transaction to the PTE API.
#[derive(Debug)]
pub enum TransactionSubmissionError {
    NoNonceFound,
    MultipleNonceFound,
    DecompileError(DecompileError),
    HttpRequestError(reqwest::Error)
}

impl From<utils::DecompileError> for TransactionSubmissionError {
    fn from(error: DecompileError) -> TransactionSubmissionError {
        TransactionSubmissionError::DecompileError(error)
    }
}

impl From<reqwest::Error> for TransactionSubmissionError {
    fn from(error: reqwest::Error) -> TransactionSubmissionError {
        TransactionSubmissionError::HttpRequestError(error)
    }
}