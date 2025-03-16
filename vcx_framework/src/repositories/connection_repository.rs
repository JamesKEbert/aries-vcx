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

// Invitation DID is here to allow for connection reuse so that connections can be searched by initiating invitation DID
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ConnectionRecordTagKeys {
    InvitationDid,
    OurDid,
    TheirDid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionRole {
    Requester,
    Responder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionRecordData {
    pub role: ConnectionRole,
    pub our_did: PeerDid<Numalgo4>,
    pub their_did: Did,
    pub invitation_did: Did,
}

/// The `ConnectionRepository` stores all connection records and provides methods for creating, updating, searching, and deleting them.
pub struct ConnectionRepository {
    // Perhaps this doesn't have to be boxed to avoid dynamic dispatch, but I struggled to get that to work. If you can do it better, please do!
    store: Box<dyn VCXFrameworkStorage<ConnectionRecordData, ConnectionRecordTagKeys>>,
}

impl ConnectionRepository {
    pub fn new(
        store: Box<dyn VCXFrameworkStorage<ConnectionRecordData, ConnectionRecordTagKeys>>,
    ) -> Self {
        Self { store }
    }

    pub fn add_or_update_record(
        &self,
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
        trace!("Retrieved ConnectionRecord '{:#?}'", record);
        Ok(record)
    }

    pub fn get_all_records(
        &self,
    ) -> Result<Vec<Record<ConnectionRecordData, ConnectionRecordTagKeys>>, ConnectionRepositoryError>
    {
        trace!("Getting all ConnectionRecords...");
        let records = self
            .store
            .get_all_records()
            .map_err(ConnectionRepositoryError::GetAllRecordsFailed)?;
        trace!("Got {} ConnectionRecords", { records.len() });
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

    pub fn delete_record(&self, id: &Uuid) -> Result<(), ConnectionRepositoryError> {
        trace!("Deleting ConnectionRecord by ID '{}'", id);
        self.store
            .delete_record(&id.to_string().as_str())
            .map_err(ConnectionRepositoryError::DeleteRecordFailed)?;
        trace!("Deleted ConnectionRecord '{}'", id);
        Ok(())
    }
}
