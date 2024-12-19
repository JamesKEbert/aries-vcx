use aries_vcx::{
    did_parser_nom::Did,
    did_peer::peer_did::{numalgos::numalgo4::Numalgo4, PeerDid},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::storage::{base::VCXFrameworkStorage, error::StorageError, record::Record};

#[derive(Error, Debug)]
pub enum ConnectionRepositoryError {
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
pub enum ConnectionRecordTagKeys {
    OurDid,
    TheirDid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionRecordData {
    pub our_did: PeerDid<Numalgo4>,
    pub their_did: Did,
}

/// The `ConnectionRepository` stores all connection records and provides methods for creating, updating, searching, and deleting them.
///
/// Takes a generic `S` which is any valid [`VCXFrameworkStorage`] instance.
pub struct ConnectionRepository<
    S: VCXFrameworkStorage<ConnectionRecordData, ConnectionRecordTagKeys>,
> {
    store: S,
}

impl<S: VCXFrameworkStorage<ConnectionRecordData, ConnectionRecordTagKeys>>
    ConnectionRepository<S>
{
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn add_or_update_record(
        &mut self,
        record: Record<ConnectionRecordData, ConnectionRecordTagKeys>,
    ) -> Result<(), ConnectionRepositoryError> {
        let id = record.id.clone();
        trace!(
            "Adding ConnectionRecord '{}' to storage:\n{:#?}",
            id,
            record
        );
        self.store
            .add_record(record)
            .map_err(ConnectionRepositoryError::AddOrUpdateRecordFailed)?;
        trace!("Added ConnectionRecord '{}' to storage", id);
        Ok(())
    }

    pub fn get_record(
        &self,
        id: &Uuid,
    ) -> Result<
        Option<Record<ConnectionRecordData, ConnectionRecordTagKeys>>,
        ConnectionRepositoryError,
    > {
        trace!("Getting ConnectionRecord by Id '{}'", id);
        let record = self
            .store
            .get_record(id.to_string().as_str())
            .map_err(ConnectionRepositoryError::AddOrUpdateRecordFailed)?;
        trace!("Retrieved DidRecord '{:#?}'", record);
        Ok(record)
    }

    pub fn get_all_records(
        &self,
    ) -> Result<Vec<Record<ConnectionRecordData, ConnectionRecordTagKeys>>, ConnectionRepositoryError>
    {
        trace!("Getting all DidRecords...");
        let records = self
            .store
            .get_all_records()
            .map_err(ConnectionRepositoryError::GetAllRecordsFailed)?;
        trace!("Got {} DidRecords", { records.len() });
        Ok(records)
    }

    pub fn search_records(
        &self,
        tag_key: ConnectionRecordTagKeys,
        tag_value: String,
    ) -> Result<Vec<Record<ConnectionRecordData, ConnectionRecordTagKeys>>, ConnectionRepositoryError>
    {
        trace!(
            "Searching records by Tag Key '{:?}' with value '{}'",
            tag_key,
            tag_value
        );
        let records = self
            .store
            .search_records(&tag_key, &tag_value)
            .map_err(ConnectionRepositoryError::SearchRecordsFailed)?;
        trace!("Found {} matching records", records.len());
        Ok(records)
    }

    pub fn delete_record(&mut self, did: &str) -> Result<(), ConnectionRepositoryError> {
        trace!("Deleting DidRecord by DID '{}'", did);
        self.store
            .delete_record(&did)
            .map_err(ConnectionRepositoryError::DeleteRecordFailed)?;
        trace!("Deleted DidRecord '{}'", did);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr};

    use crate::{storage::in_memory_storage::InMemoryStorage, test_init};

    use super::*;
}
