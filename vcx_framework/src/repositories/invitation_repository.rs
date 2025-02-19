use aries_vcx::{
    did_parser_nom::Did,
    did_peer::peer_did::{numalgos::numalgo4::Numalgo4, PeerDid},
    handlers::out_of_band::sender::OutOfBandSender,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::storage::{base::VCXFrameworkStorage, error::StorageError, record::Record};

#[derive(Error, Debug)]
pub enum InvitationRepositoryError {
    #[error("Failed to add or update record")]
    AddOrUpdateRecordFailed(#[source] StorageError),
    #[error("Failed to get Record")]
    GetRecordFailed(#[source] StorageError),
    #[error("Failed to get all Records")]
    GetAllRecordsFailed(#[source] StorageError),
    #[error("Failed to search Records")]
    SearchRecordsFailed(#[source] StorageError),
    #[error("Failed to delete record")]
    DeleteRecordFailed(#[source] StorageError),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum InvitationRecordTagKeys {
    SelfCreated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvitationRecordData {
    pub invite: OutOfBandSender,
    pub self_created: bool,
}

/// The `InvitationRepository` stores all Invitation records and provides methods for creating, updating, searching, and deleting them.
///
/// Takes a generic `S` which is any valid [`VCXFrameworkStorage`] instance.
pub struct InvitationRepository {
    // Perhaps this doesn't have to be boxed to avoid dynamic dispatch, but I struggled to get that to work. If you can do it better, please do!
    store: Box<dyn VCXFrameworkStorage<InvitationRecordData, InvitationRecordTagKeys>>,
}

impl InvitationRepository {
    pub fn new(
        store: Box<dyn VCXFrameworkStorage<InvitationRecordData, InvitationRecordTagKeys>>,
    ) -> Self {
        Self { store }
    }

    pub fn add_or_update_record(
        &self,
        record: Record<InvitationRecordData, InvitationRecordTagKeys>,
    ) -> Result<(), InvitationRepositoryError> {
        let id = record.id.clone();
        trace!(
            "Adding InvitationRecord '{}' to storage:\n{:#?}",
            id,
            record
        );
        self.store
            .add_record(record)
            .map_err(InvitationRepositoryError::AddOrUpdateRecordFailed)?;
        trace!("Added InvitationRecord '{}' to storage", id);
        Ok(())
    }

    pub fn get_record(
        &self,
        id: &Uuid,
    ) -> Result<
        Option<Record<InvitationRecordData, InvitationRecordTagKeys>>,
        InvitationRepositoryError,
    > {
        trace!("Getting InvitationRecord by Id '{}'", id);
        let record = self
            .store
            .get_record(id.to_string().as_str())
            .map_err(InvitationRepositoryError::AddOrUpdateRecordFailed)?;
        trace!("Retrieved Invitation Record '{:#?}'", record);
        Ok(record)
    }

    pub fn get_all_records(
        &self,
    ) -> Result<Vec<Record<InvitationRecordData, InvitationRecordTagKeys>>, InvitationRepositoryError>
    {
        trace!("Getting all InvitationRecords...");
        let records = self
            .store
            .get_all_records()
            .map_err(InvitationRepositoryError::GetAllRecordsFailed)?;
        trace!("Got {} InvitationRecords", { records.len() });
        Ok(records)
    }

    pub fn search_records(
        &self,
        tag_key: InvitationRecordTagKeys,
        tag_value: String,
    ) -> Result<Vec<Record<InvitationRecordData, InvitationRecordTagKeys>>, InvitationRepositoryError>
    {
        trace!(
            "Searching records by Tag Key '{:?}' with value '{}'",
            tag_key,
            tag_value
        );
        let records = self
            .store
            .search_records(&tag_key, &tag_value)
            .map_err(InvitationRepositoryError::SearchRecordsFailed)?;
        trace!("Found {} matching records", records.len());
        Ok(records)
    }

    pub fn delete_record(&self, id: &Uuid) -> Result<(), InvitationRepositoryError> {
        trace!("Deleting InvitationRecord by id '{}'", id);
        self.store
            .delete_record(&id.to_string().as_str())
            .map_err(InvitationRepositoryError::DeleteRecordFailed)?;
        trace!("Deleted InvitationRecord '{}'", id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr};

    use crate::{storage::in_memory_storage::InMemoryStorage, test_init};

    use super::*;
}
