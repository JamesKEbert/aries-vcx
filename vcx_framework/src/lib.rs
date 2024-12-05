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

    pub type Record = String;

    pub trait VCXFrameworkStorage<I> {
        fn add_record(&mut self, id: I, record: Record) -> Result<(), StorageError>;
        fn add_or_update_record(&mut self, id: I, record: Record) -> Result<(), StorageError>;
        fn update_record(&mut self, id: I, record: Record) -> Result<(), StorageError>;
        fn get_record(&self, id: I) -> Result<Option<Record>, StorageError>;
        fn delete_record(&mut self, id: I) -> Result<(), StorageError>;
    }

    struct InMemoryStorage {
        records: HashMap<Uuid, Record>,
    }

    impl InMemoryStorage {
        fn new() -> Self {
            InMemoryStorage {
                records: HashMap::new(),
            }
        }
    }

    impl VCXFrameworkStorage<Uuid> for InMemoryStorage {
        fn add_record(&mut self, id: Uuid, record: Record) -> Result<(), StorageError> {
            if self.records.contains_key(&id.clone()) {
                return Err(StorageError::DuplicateRecord);
            } else {
                self.records.insert(id, record);
            }
            Ok(())
        }
        fn add_or_update_record(&mut self, id: Uuid, record: Record) -> Result<(), StorageError> {
            self.records.insert(id, record);
            Ok(())
        }
        fn update_record(&mut self, id: Uuid, record: Record) -> Result<(), StorageError> {
            if self.records.contains_key(&id.clone()) {
                self.records.insert(id, record);
                Ok(())
            } else {
                return Err(StorageError::RecordDoesNotExist);
            }
        }
        fn get_record(&self, id: Uuid) -> Result<Option<Record>, StorageError> {
            Ok(self.records.get(&id).cloned())
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
            let record: Record = "Foo".to_owned();
            let id = Uuid::new_v4();
            let _ = in_memory_storage.add_record(id, record.clone());
            let retrieved_record: Record = in_memory_storage
                .get_record(id)
                .unwrap()
                .expect("Record to exist");
            assert_eq!(record, retrieved_record);
        }

        #[test]
        fn test_add_duplicate() {
            test_init();
            let mut in_memory_storage = InMemoryStorage::new();
            let record: Record = "Foo".to_owned();
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
            let record: Record = "Foo".to_owned();
            let id = Uuid::new_v4();
            let _ = in_memory_storage.add_or_update_record(id, record.clone());
            let retrieved_record: Record = in_memory_storage
                .get_record(id)
                .unwrap()
                .expect("Record to exist");
            assert_eq!(record, retrieved_record);
        }

        #[test]
        fn test_update_record() {
            test_init();
            let mut in_memory_storage = InMemoryStorage::new();
            let record: Record = "Foo".to_owned();
            let updated_record: Record = "Foo".to_owned();
            let id = Uuid::new_v4();
            let _ = in_memory_storage.add_record(id, record.clone());
            let _ = in_memory_storage.update_record(id, updated_record.clone());
            let retrieved_record: Record = in_memory_storage
                .get_record(id)
                .unwrap()
                .expect("Record to exist");
            assert_eq!(updated_record, retrieved_record);
        }

        #[test]
        fn test_update_record_no_record() {
            test_init();
            let mut in_memory_storage = InMemoryStorage::new();
            let updated_record: Record = "Foo".to_owned();
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
            let record: Record = "Foo".to_owned();
            let id = Uuid::new_v4();
            let _ = in_memory_storage.add_record(id, record.clone());
            let _ = in_memory_storage.delete_record(id);
            assert_eq!(None, in_memory_storage.get_record(id).unwrap());
        }

        #[test]
        fn test_delete_record_already_deleted() {
            test_init();
            let mut in_memory_storage = InMemoryStorage::new();
            let record: Record = "Foo".to_owned();
            let id = Uuid::new_v4();
            let _ = in_memory_storage.add_record(id, record.clone());
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
