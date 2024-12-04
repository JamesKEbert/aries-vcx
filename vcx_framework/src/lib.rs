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
    use std::{collections::HashMap, error::Error};

    use uuid::Uuid;

    pub type Record = String;

    pub trait VCXFrameworkStorage<I> {
        fn add_record(self, id: I, record: Record) -> Result<(), Box<dyn Error>>;
        fn add_or_update_record(self, id: I, record: Record) -> Result<(), Box<dyn Error>>;
        fn update_record(self, id: I, record: Record) -> Result<(), Box<dyn Error>>;
        fn get_record(self, id: I) -> Result<Option<Record>, Box<dyn Error>>;
        fn delete_record(self, id: I) -> Result<(), Box<dyn Error>>;
    }

    struct InMemoryStorage {
        records: HashMap<Uuid, Record>,
    }

    impl VCXFrameworkStorage<Uuid> for InMemoryStorage {
        fn add_record(mut self, id: Uuid, record: Record) -> Result<(), Box<dyn Error>> {
            if !self.records.contains_key(&id.clone()) {
                Err("Storage Already Contains Record");
            } else {
                self.records.insert(id, record);
            }
            Ok(())
        }
        fn add_or_update_record(mut self, id: Uuid, record: Record) -> Result<(), Box<dyn Error>> {
            self.records.insert(id, record);
            Ok(())
        }
        fn update_record(mut self, id: Uuid, record: Record) -> Result<(), Box<dyn Error>> {
            self.records.insert(id, record);
            Ok(())
        }
        fn get_record(mut self, id: Uuid) -> Result<Option<Record>, Box<dyn Error>> {
            Ok(self.records.get(&id).cloned())
        }
        fn delete_record(mut self, id: Uuid) -> Result<(), Box<dyn Error>> {
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
        fn test_create_or_update_record() {
            test_init();
        }

        #[test]
        fn test_create_and_read_record() {
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
