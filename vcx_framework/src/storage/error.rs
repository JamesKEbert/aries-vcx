use std::{
    error,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub enum StorageError {
    DuplicateRecord,
    RecordDoesNotExist,
    Serialization(serde_json::Error),
    Deserialization(serde_json::Error),
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
            StorageError::Serialization(_err) => {
                write!(f, "Error serializing record")
            }
            StorageError::Deserialization(_err) => {
                write!(f, "Error deserializing record")
            }
        }
    }
}

impl error::Error for StorageError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            StorageError::Serialization(ref err) => Some(err),
            StorageError::Deserialization(ref err) => Some(err),
            _ => None,
        }
    }
}
