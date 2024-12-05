pub mod error {
    use std::fmt::{Display, Formatter};

    use crate::storage::StorageError;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum VCXFrameworkError {
        Storage(StorageError),
    }

    impl Display for VCXFrameworkError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                VCXFrameworkError::Storage(storage_error) => StorageError::fmt(storage_error, f),
            }
        }
    }

    impl std::error::Error for VCXFrameworkError {}
}

pub mod messaging_module {

    #[cfg(test)]
    mod tests {
        use crate::test_init;

        use super::*;

        #[test]
        fn test_encrypt_message() {
            test_init();
        }
    }
}

pub mod storage {
    use std::{
        collections::HashMap,
        fmt::{Display, Formatter},
        marker::{self, PhantomData},
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
}

pub mod did_registry {
    use std::error::Error;

    pub struct DidRegistry {}

    impl DidRegistry {
        pub fn create_did() -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::test_init;

        use super::*;

        #[test]
        fn test_create_and_read_did() {
            test_init();
        }

        #[test]
        fn test_create_or_update_did() {
            test_init();
        }

        #[test]
        fn test_update_did() {
            test_init();
        }

        #[test]
        fn test_delete_did() {
            test_init();
        }
    }
}

#[cfg(test)]
fn test_init() {
    env_logger::builder().is_test(true).try_init().ok();
}
