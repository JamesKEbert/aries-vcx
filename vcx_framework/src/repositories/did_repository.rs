// #[cfg(feature = "did_repository")]
// pub mod did_repository {
//     use super::VCXFrameworkStorage;

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct DidRecordData {
//     value: String,
// }

// #[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
// pub enum DidRecordTagKeys {
//     KeyAgreementKey,
// }

//     struct DIDRecord {
//         // I would prefer to have this be an actual DID type in the future, but that'll take work on the did_core crates - @JamesKEbert
//         did: String,
//     }

//     /// The `DidRepository` stores all created and known DIDs, and where appropriate, stores full DIDDocs (such as storing a long form did:peer:4 or with TTL caching strategies).
//     /// Otherwise, DID resolution should be done at runtime.
//     struct DidRepository<S: VCXFrameworkStorage<String, SimpleRecord>> {
//         store: S,
//     }

//     impl<S: VCXFrameworkStorage<String, SimpleRecord>> DidRepository<S> {
//         fn new(store: S) -> Self {
//             Self { store }
//         }

//         // fn get_did_record(did: String) -> DIDRecord {}
//     }
// }
