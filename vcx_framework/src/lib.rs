pub mod error {
    use std::fmt::{Display, Formatter};

    use crate::storage::StorageError;

    #[derive(Debug, Clone, Copy)]
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

    #[derive(Debug, Clone, Copy)]
    pub enum StorageError {
        Duplicate,
    }

    impl Display for StorageError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                StorageError::Duplicate => {
                    write!(f, "Create record failed due to record already existing")
                }
            }
        }
    }

    pub type Record = String;

    pub trait VCXFrameworkStorage<I> {
        fn add_record(self, id: I, record: Record) -> Result<(), StorageError>;
        fn add_or_update_record(self, id: I, record: Record) -> Result<(), StorageError>;
        fn update_record(self, id: I, record: Record) -> Result<(), StorageError>;
        fn get_record(self, id: I) -> Result<Option<Record>, StorageError>;
        fn delete_record(self, id: I) -> Result<(), StorageError>;
    }

    struct InMemoryStorage {
        records: HashMap<Uuid, Record>,
    }

    impl VCXFrameworkStorage<Uuid> for InMemoryStorage {
        fn add_record(mut self, id: Uuid, record: Record) -> Result<(), StorageError> {
            if self.records.contains_key(&id.clone()) {
                return Err(StorageError::Duplicate);
            } else {
                self.records.insert(id, record);
            }
            Ok(())
        }
        fn add_or_update_record(mut self, id: Uuid, record: Record) -> Result<(), StorageError> {
            self.records.insert(id, record);
            Ok(())
        }
        fn update_record(mut self, id: Uuid, record: Record) -> Result<(), StorageError> {
            self.records.insert(id, record);
            Ok(())
        }
        fn get_record(self, id: Uuid) -> Result<Option<Record>, StorageError> {
            Ok(self.records.get(&id).cloned())
        }
        fn delete_record(mut self, id: Uuid) -> Result<(), StorageError> {
            self.records.remove(&id);
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::test_init;

        use super::*;

        #[test]
        fn test_create_and_read_record() {
            test_init();
        }

        #[test]
        fn test_create_duplicate() {
            test_init();
        }

        #[test]
        fn test_create_or_update_record() {
            test_init();
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
