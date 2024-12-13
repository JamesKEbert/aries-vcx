use std::hash::Hash;

use serde::{de::DeserializeOwned, Serialize};

use super::{error::StorageError, record::Record};

/// This trait provides a general purpose storage trait that provides CRUD style operations that correspond to a generic [`Record`].
/// It also takes a generic `TK` that is the valid enum to be used for this [`Record`]'s tag keys
pub trait VCXFrameworkStorage<D, TK>
where
    D: Serialize + DeserializeOwned + std::fmt::Debug,
    TK: Eq + Hash + Clone + std::fmt::Debug + Serialize + DeserializeOwned,
{
    /// Adds a record to the storage. Will not update an existing record with the same id, otherwise use [`add_or_update_record()`] instead.
    fn add_record(&mut self, record: Record<D, TK>) -> Result<(), StorageError>;

    /// Adds or updates an existing record to the storage.
    fn add_or_update_record(&mut self, record: Record<D, TK>) -> Result<(), StorageError>;

    /// Updates a record in the storage. Will not update a non existent record. To update or create if non-existent, use [`add_or_update_record()`] instead.
    fn update_record(&mut self, record: Record<D, TK>) -> Result<(), StorageError>;

    /// Gets a record from the storage by id if it exists.
    fn get_record(&self, id: &str) -> Result<Option<Record<D, TK>>, StorageError>;

    // TODO: Pagination
    /// Gets all records from the storage. Pagination not yet implemented
    fn get_all_records(&self) -> Result<Vec<Record<D, TK>>, StorageError>;

    // TODO: Pagination
    // Searches all records in the storage by a given tag key and tag value. The tag key must be one of the TK enum. Pagination not yet implemented
    fn search_records(
        &self,
        tag_key: &TK,
        tag_value: &str,
    ) -> Result<Vec<Record<D, TK>>, StorageError>;

    /// Deletes a record from the storage by id.
    fn delete_record(&mut self, id: &str) -> Result<(), StorageError>;
}
