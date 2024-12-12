pub mod error {
    use std::fmt::{Display, Formatter};

    use crate::storage::StorageError;

    #[derive(Debug)]
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

pub mod storage;
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
