use std::{
    collections::HashMap,
    error,
    fmt::{Display, Formatter},
    marker::PhantomData,
};

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageError {
    DuplicateRecord,
    RecordDoesNotExist,
}

impl Display for StorageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::DuplicateRecord => {
                write!(f, "Create record failed due to record already existing")
            }
            StorageError::RecordDoesNotExist => {
                write!(f, "Record does not exist")
            }
        }
    }
}

impl error::Error for StorageError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            _ => None,
        }
    }
}

/// This trait provides a general purpose storage trait that provides CRUD style operations that correspond to a generic [`Record`].
pub trait VCXFrameworkStorage<I, R: Record> {
    /// Adds a record to the storage by id. Will not update an existing record with the same id, otherwise use [`add_or_update_record()`] instead.
    fn add_record(&mut self, id: I, record: R) -> Result<(), StorageError>;

    /// Adds or updates an existing record by id to the storage.
    fn add_or_update_record(&mut self, id: I, record: R) -> Result<(), StorageError>;

    /// Updates a record in the storage. Will not update a non existent record. To update or create if non-existent, use [`add_or_update_record()`] instead.
    fn update_record(&mut self, id: I, record: R) -> Result<(), StorageError>;

    /// Gets a record from the storage by id if it exists.
    fn get_record(&self, id: I) -> Result<Option<R>, StorageError>;

    /// Deletes a record from the storage by id.
    fn delete_record(&mut self, id: I) -> Result<(), StorageError>;
}

/// A generic Record trait for use with [`VCXFrameworkStorage`], the only requirement being the ability to serialize and deserialize from and to a String
pub trait Record {
    fn to_string(&self) -> String;
    fn from_string(string: String) -> Self;
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleRecord {
    value: String,
}

impl Record for SimpleRecord {
    fn to_string(&self) -> String {
        self.value.to_string()
    }
    fn from_string(string: String) -> Self {
        Self { value: string }
    }
}

struct InMemoryStorage<R: Record> {
    records: HashMap<Uuid, String>,
    // PhantomData is used so that the Record type must be determined at `new()`, which is required given that the Record type isn't specified in any of the struct fields.
    // This is done so that the type doesn't have to be inferred or manually set later during use.
    _phantom: PhantomData<R>,
}

impl<R: Record> InMemoryStorage<R> {
    fn new() -> Self {
        InMemoryStorage::<R> {
            records: HashMap::new(),
            _phantom: PhantomData,
        }
    }
}

impl<R: Record> VCXFrameworkStorage<Uuid, R> for InMemoryStorage<R> {
    fn add_record(&mut self, id: Uuid, record: R) -> Result<(), StorageError> {
        if self.records.contains_key(&id.clone()) {
            return Err(StorageError::DuplicateRecord);
        } else {
            self.records.insert(id, record.to_string());
        }
        Ok(())
    }
    fn add_or_update_record(&mut self, id: Uuid, record: R) -> Result<(), StorageError> {
        self.records.insert(id, record.to_string());
        Ok(())
    }
    fn update_record(&mut self, id: Uuid, record: R) -> Result<(), StorageError> {
        if self.records.contains_key(&id.clone()) {
            self.records.insert(id, record.to_string());
            Ok(())
        } else {
            return Err(StorageError::RecordDoesNotExist);
        }
    }
    fn get_record(&self, id: Uuid) -> Result<Option<R>, StorageError> {
        let record = self.records.get(&id);
        match record {
            Some(retrieved_record) => Ok(Some(R::from_string(retrieved_record.to_owned()))),
            None => Ok(None),
        }
    }
    fn delete_record(&mut self, id: Uuid) -> Result<(), StorageError> {
        self.records.remove(&id);
        Ok(())
    }
}

#[cfg(feature = "did_registry")]
pub mod did_registry {
    use super::{SimpleRecord, VCXFrameworkStorage};

    /// The `DIDRegistry` stores all created and known DIDs, and where appropriate, stores full DIDDocs (such as in some DID Peer scenarios or via a TTL resolution cache).
    /// Otherwise, DID resolution should be done at runtime.
    struct DidRegistry<S: VCXFrameworkStorage<String, SimpleRecord>> {
        store: S,
    }

    impl<S: VCXFrameworkStorage<String, SimpleRecord>> DidRegistry<S> {
        fn new(store: S) -> Self {
            Self { store }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_init;

    use super::*;

    #[test]
    fn test_add_and_read_record() {
        test_init();
        let mut in_memory_storage = InMemoryStorage::new();
        let record = SimpleRecord {
            value: "Foo".to_owned(),
        };
        let id = Uuid::new_v4();

        let _ = in_memory_storage.add_record(id, record.clone());
        let retrieved_record = in_memory_storage
            .get_record(id)
            .unwrap()
            .expect("Record to exist");
        assert_eq!(record, retrieved_record);
    }

    #[test]
    fn test_add_duplicate() {
        test_init();
        let mut in_memory_storage = InMemoryStorage::new();
        let record = SimpleRecord {
            value: "Foo".to_owned(),
        };
        let id = Uuid::new_v4();
        let _ = in_memory_storage.add_record(id, record.clone());
        assert_eq!(
            Err(StorageError::DuplicateRecord),
            in_memory_storage.add_record(id, record),
        )
    }

    #[test]
    fn test_add_or_update_record() {
        test_init();
        let mut in_memory_storage = InMemoryStorage::new();
        let record = SimpleRecord {
            value: "Foo".to_owned(),
        };
        let id = Uuid::new_v4();
        let _ = in_memory_storage.add_or_update_record(id, record.clone());
        let retrieved_record = in_memory_storage
            .get_record(id)
            .unwrap()
            .expect("Record to exist");
        assert_eq!(record, retrieved_record);
    }

    #[test]
    fn test_update_record() {
        test_init();
        let mut in_memory_storage = InMemoryStorage::new();
        let record = SimpleRecord {
            value: "Foo".to_owned(),
        };
        let updated_record = SimpleRecord {
            value: "Foo2".to_owned(),
        };
        let id = Uuid::new_v4();
        let _ = in_memory_storage.add_record(id, record.clone());
        let _ = in_memory_storage.update_record(id, updated_record.clone());
        let retrieved_record = in_memory_storage
            .get_record(id)
            .unwrap()
            .expect("Record to exist");
        assert_eq!(updated_record, retrieved_record);
    }

    #[test]
    fn test_update_record_no_record() {
        test_init();
        let mut in_memory_storage = InMemoryStorage::new();
        let updated_record = SimpleRecord {
            value: "Foo2".to_owned(),
        };
        let id = Uuid::new_v4();

        assert_eq!(
            Err(StorageError::RecordDoesNotExist),
            in_memory_storage.update_record(id, updated_record)
        )
    }

    #[test]
    fn test_delete_record() {
        test_init();
        let mut in_memory_storage = InMemoryStorage::new();
        let record = SimpleRecord {
            value: "Foo".to_owned(),
        };
        let id = Uuid::new_v4();
        let _ = in_memory_storage.add_record(id, record.clone());
        let _ = in_memory_storage
            .delete_record(id)
            .expect("To delete record");
        assert_eq!(None, in_memory_storage.get_record(id).unwrap());
    }

    #[test]
    fn test_delete_record_already_deleted() {
        test_init();
        let mut in_memory_storage = InMemoryStorage::new();
        let record = SimpleRecord {
            value: "Foo".to_owned(),
        };
        let id = Uuid::new_v4();
        let _ = in_memory_storage.add_record(id, record.clone());
        let _ = in_memory_storage.delete_record(id);
        let _ = in_memory_storage.delete_record(id);
        assert_eq!(None, in_memory_storage.get_record(id).unwrap());
    }
}
