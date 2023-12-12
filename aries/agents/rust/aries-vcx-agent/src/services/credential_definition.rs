use std::sync::{Arc, Mutex};

use aries_vcx::common::primitives::credential_definition::{CredentialDef, CredentialDefConfig};
use aries_vcx_core::{
    anoncreds::credx_anoncreds::IndyCredxAnonCreds,
    ledger::indy_vdr_ledger::{DefaultIndyLedgerRead, DefaultIndyLedgerWrite},
    wallet::indy::IndySdkWallet,
};

use crate::{
    error::*,
    storage::{object_cache::ObjectCache, Storage},
};

pub struct ServiceCredentialDefinitions {
    ledger_read: Arc<DefaultIndyLedgerRead>,
    ledger_write: Arc<DefaultIndyLedgerWrite>,
    anoncreds: IndyCredxAnonCreds,
    wallet: Arc<IndySdkWallet>,
    cred_defs: ObjectCache<CredentialDef>,
}

impl ServiceCredentialDefinitions {
    pub fn new(
        ledger_read: Arc<DefaultIndyLedgerRead>,
        ledger_write: Arc<DefaultIndyLedgerWrite>,
        anoncreds: IndyCredxAnonCreds,
        wallet: Arc<IndySdkWallet>,
    ) -> Self {
        Self {
            cred_defs: ObjectCache::new("cred-defs"),
            ledger_read,
            ledger_write,
            anoncreds,
            wallet,
        }
    }

    pub async fn create_cred_def(&self, config: CredentialDefConfig) -> AgentResult<String> {
        let cd = CredentialDef::create(
            self.wallet.as_ref(),
            self.ledger_read.as_ref(),
            &self.anoncreds,
            "".to_string(),
            config,
            true,
        )
        .await?;
        self.cred_defs.insert(&cd.get_cred_def_id(), cd)
    }

    pub async fn publish_cred_def(&self, thread_id: &str) -> AgentResult<()> {
        let cred_def = self.cred_defs.get(thread_id)?;
        let cred_def = cred_def
            .publish_cred_def(
                self.wallet.as_ref(),
                self.ledger_read.as_ref(),
                self.ledger_write.as_ref(),
            )
            .await?;
        self.cred_defs.insert(thread_id, cred_def)?;
        Ok(())
    }

    pub fn cred_def_json(&self, thread_id: &str) -> AgentResult<String> {
        self.cred_defs
            .get(thread_id)?
            .get_data_json()
            .map_err(|err| err.into())
    }

    pub fn find_by_schema_id(&self, schema_id: &str) -> AgentResult<Vec<String>> {
        let schema_id = schema_id.to_string();
        let f = |(id, m): (&String, &Mutex<CredentialDef>)| -> Option<String> {
            let cred_def = m.lock().unwrap();
            if cred_def.get_schema_id() == schema_id {
                Some(id.clone())
            } else {
                None
            }
        };
        self.cred_defs.find_by(f)
    }
}