use std::{
    error,
    fmt::{Display, Formatter},
};

use serde::{Deserialize, Serialize};

use crate::storage::{base::VCXFrameworkStorage, error::StorageError, record::Record};

#[derive(Debug)]
pub enum DidRepositoryError {
    AddOrUpdateRecordFailed(StorageError),
    GetRecordFailed(StorageError),
    GetAllRecordsFailed(StorageError),
    SearchRecordsFailed(StorageError),
    DeleteRecordFailed(StorageError),
}

impl Display for DidRepositoryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DidRepositoryError::AddOrUpdateRecordFailed(_err) => {
                write!(f, "Failed to add or update record")
            }
            DidRepositoryError::GetRecordFailed(_err) => {
                write!(f, "Failed to get Record")
            }
            DidRepositoryError::GetAllRecordsFailed(_err) => {
                write!(f, "Failed to get all Record")
            }
            DidRepositoryError::SearchRecordsFailed(_err) => {
                write!(f, "Failed to search Records")
            }
            DidRepositoryError::DeleteRecordFailed(_err) => {
                write!(f, "Failed to delete record")
            }
        }
    }
}

impl error::Error for DidRepositoryError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            DidRepositoryError::AddOrUpdateRecordFailed(ref err) => Some(err),
            DidRepositoryError::GetRecordFailed(ref err) => Some(err),
            DidRepositoryError::GetAllRecordsFailed(ref err) => Some(err),
            DidRepositoryError::SearchRecordsFailed(ref err) => Some(err),
            DidRepositoryError::DeleteRecordFailed(ref err) => Some(err),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum DidRecordTagKeys {
    KeyAgreementKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DidRecordData {
    // I would prefer to have this be an actual DID type in the future, but that'll take work on the did_core crates - @JamesKEbert
    did: String,
}

/// The `DidRepository` stores all created and known DIDs, and where appropriate, stores full DIDDocs (such as storing a long form did:peer:4 or with TTL caching strategies).
/// Otherwise, DID resolution should be done at runtime.
///
/// Takes a generic `S` which is any valid [`VCXFrameworkStorage`] instance.
pub struct DidRepository<S: VCXFrameworkStorage<DidRecordData, DidRecordTagKeys>> {
    store: S,
}

impl<S: VCXFrameworkStorage<DidRecordData, DidRecordTagKeys>> DidRepository<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn add_or_update_record(
        &mut self,
        record: Record<DidRecordData, DidRecordTagKeys>,
    ) -> Result<(), DidRepositoryError> {
        let id = record.id.clone();
        trace!("Adding DidRecord '{}' to storage:\n{:#?}", id, record);
        self.store
            .add_record(record)
            .map_err(DidRepositoryError::AddOrUpdateRecordFailed)?;
        trace!("Added DidRecord '{}' to storage", id);
        Ok(())
    }

    pub fn get_record(
        &self,
        did: &str,
    ) -> Result<Option<Record<DidRecordData, DidRecordTagKeys>>, DidRepositoryError> {
        trace!("Getting DidRecord by DID '{}'", did);
        let record = self
            .store
            .get_record(did)
            .map_err(DidRepositoryError::AddOrUpdateRecordFailed)?;
        trace!("Retrieved DidRecord '{:#?}'", record);
        Ok(record)
    }

    pub fn get_all_records(
        &self,
    ) -> Result<Vec<Record<DidRecordData, DidRecordTagKeys>>, DidRepositoryError> {
        trace!("Getting all DidRecords...");
        let records = self
            .store
            .get_all_records()
            .map_err(DidRepositoryError::GetAllRecordsFailed)?;
        trace!("Got {} DidRecords", { records.len() });
        Ok(records)
    }

    pub fn search_records(
        &self,
        tag_key: DidRecordTagKeys,
        tag_value: String,
    ) -> Result<Vec<Record<DidRecordData, DidRecordTagKeys>>, DidRepositoryError> {
        trace!(
            "Searching records by Tag Key '{:?}' with value '{}'",
            tag_key,
            tag_value
        );
        let records = self
            .store
            .search_records(&tag_key, &tag_value)
            .map_err(DidRepositoryError::SearchRecordsFailed)?;
        trace!("Found {} matching records", records.len());
        Ok(records)
    }

    fn delete_record(&mut self, did: &str) -> Result<(), DidRepositoryError> {
        trace!("Deleting DidRecord by DID '{}'", did);
        self.store
            .delete_record(&did)
            .map_err(DidRepositoryError::DeleteRecordFailed)?;
        trace!("Deleted DidRecord '{}'", did);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{storage::in_memory_storage::InMemoryStorage, test_init};

    use super::*;
    #[test]
    fn did_repository() {
        test_init();
        let in_memory_storage = InMemoryStorage::<DidRecordData, DidRecordTagKeys>::new();
        let mut did_repository = DidRepository::new(in_memory_storage);
        let did = String::from("did:peer:4zQmcQCH8nWEBBA6BpSEDxHyhPwHdi5CVGcvsZcjhb618zbA:z5CTtVoAxKjH1V1sKizLy5kLvV6AbmACYfcGmfVUDGn4A7BpnVQEESXEYYUG7W479kDHaqLnk7NJuu4w7ftTd9REipB2CQgW9fjzPvmsXyyHzot9o1tgYHNnqFDXgCXwFYJfjkzz3m6mex1WMN4XHWWNM4NB7exDA2maVGis7gJnVAiNrBExaihyeKJ4nBXrB3ArQ1TyuZ39F9qTeCSrBntTTa85wtUtHz5M1oE7Sj1CZeAEQzDnAMToP9idSrSXUo5z8q9Un325d8MtQgxyKGW2a9VYyW189C722GKQbGQSU3dRSwCanVHJwCh9q2G2eNVPeuydAHXmouCUCq3cVHeUkatv73DSoBV17LEJgq8dAYfvSAutG7LFyvrRW5wNjcQMT7WdFHRCqhtzz18zu6fSTQWM4PQPLMVEaKbs51EeYGiGurhu1ChQMjXqnpcRcpCP7RAEgyWSjMER6e3gdCVsBhQSoqGk1UN8NfVah8pxGg2i5Gd1754Ys6aBEhTashFa47Ke7oPoZ6LZiRMETYhUr1cQY65TQhMzyrR6RzLudeRVgcRdKiTTmP2fFi5H8nCHPSGb4wncUxgn3N5CbFaUC");
        let data = DidRecordData { did: did.clone() };
        let record = Record::new(did.clone(), data, None);

        // Add and get record test
        did_repository
            .add_or_update_record(record.clone())
            .expect("No errors");

        let retrieved_record = did_repository
            .get_record(&did)
            .expect("No errors")
            .expect("For some record to be retrieved");

        assert_eq!(record, retrieved_record);

        // Test get all records
        let all_records = did_repository.get_all_records().expect("No errors");
        assert_eq!(vec![record], all_records);

        // Test delete record
        did_repository.delete_record(&did).expect("No errors");

        assert_eq!(None, did_repository.get_record(&did).expect("No errors"))
    }

    #[test]
    fn test_search_did_repository() {
        test_init();
        let in_memory_storage = InMemoryStorage::<DidRecordData, DidRecordTagKeys>::new();
        let mut did_repository = DidRepository::new(in_memory_storage);
        let did = String::from("did:peer:4zQmcQCH8nWEBBA6BpSEDxHyhPwHdi5CVGcvsZcjhb618zbA:z5CTtVoAxKjH1V1sKizLy5kLvV6AbmACYfcGmfVUDGn4A7BpnVQEESXEYYUG7W479kDHaqLnk7NJuu4w7ftTd9REipB2CQgW9fjzPvmsXyyHzot9o1tgYHNnqFDXgCXwFYJfjkzz3m6mex1WMN4XHWWNM4NB7exDA2maVGis7gJnVAiNrBExaihyeKJ4nBXrB3ArQ1TyuZ39F9qTeCSrBntTTa85wtUtHz5M1oE7Sj1CZeAEQzDnAMToP9idSrSXUo5z8q9Un325d8MtQgxyKGW2a9VYyW189C722GKQbGQSU3dRSwCanVHJwCh9q2G2eNVPeuydAHXmouCUCq3cVHeUkatv73DSoBV17LEJgq8dAYfvSAutG7LFyvrRW5wNjcQMT7WdFHRCqhtzz18zu6fSTQWM4PQPLMVEaKbs51EeYGiGurhu1ChQMjXqnpcRcpCP7RAEgyWSjMER6e3gdCVsBhQSoqGk1UN8NfVah8pxGg2i5Gd1754Ys6aBEhTashFa47Ke7oPoZ6LZiRMETYhUr1cQY65TQhMzyrR6RzLudeRVgcRdKiTTmP2fFi5H8nCHPSGb4wncUxgn3N5CbFaUC");
        let data = DidRecordData { did: did.clone() };
        let mut tags = HashMap::new();
        tags.insert(
            DidRecordTagKeys::KeyAgreementKey,
            String::from("z6MkuNenWjqDeZ4DjkHoqX6WdDYTfUUqcR7ASezo846GHe74"),
        );
        let record = Record::new(did.clone(), data, Some(tags));

        // Add and get record test
        did_repository
            .add_or_update_record(record.clone())
            .expect("No errors");
        let records = did_repository
            .search_records(
                DidRecordTagKeys::KeyAgreementKey,
                String::from("z6MkuNenWjqDeZ4DjkHoqX6WdDYTfUUqcR7ASezo846GHe74"),
            )
            .expect("No Errors");
        assert_eq!(vec![record], records);
    }
}
