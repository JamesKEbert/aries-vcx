use std::{
    collections::HashMap,
    error,
    fmt::{Display, Formatter},
    marker::PhantomData,
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

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

/// This trait provides a general purpose storage trait that provides CRUD style operations that correspond to a generic [`Record`].
pub trait VCXFrameworkStorage<D: Serialize + DeserializeOwned> {
    /// Adds a record to the storage by id. Will not update an existing record with the same id, otherwise use [`add_or_update_record()`] instead.
    fn add_record(&mut self, record: Record<D>) -> Result<(), StorageError>;

    /// Adds or updates an existing record by id to the storage.
    fn add_or_update_record(&mut self, record: Record<D>) -> Result<(), StorageError>;

    /// Updates a record in the storage. Will not update a non existent record. To update or create if non-existent, use [`add_or_update_record()`] instead.
    fn update_record(&mut self, record: Record<D>) -> Result<(), StorageError>;

    /// Gets a record from the storage by id if it exists.
    fn get_record(&self, id: String) -> Result<Option<Record<D>>, StorageError>;

    // TODO: Pagination
    /// Gets all records from the storage. Pagination not yet implemented
    fn get_all_records(&self) -> Result<Vec<Record<D>>, StorageError>;

    // TODO: Pagination
    // Searches all records in the storage by a given tag key and tag value. Pagination not yet implemented
    fn search_records(
        &self,
        tag_key: &str,
        tag_value: &str,
    ) -> Result<Vec<Record<D>>, StorageError>;

    /// Deletes a record from the storage by id.
    fn delete_record(&mut self, id: String) -> Result<(), StorageError>;
}

// pub trait RecordId: Display + Eq + Hash + Clone {
//     fn get_id(&self) -> String;
// }

// type RecordIdString = String;
// impl RecordId for RecordIdString {
//     fn get_id(&self) -> String {
//         self.clone()
//     }
// }

// type RecordIdUuid = Uuid;
// impl RecordId for RecordIdUuid {
//     fn get_id(&self) -> String {
//         self.to_string()
//     }
// }

// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
// enum RecordId {
//     String(String),
//     Uuid(Uuid),
// }

// impl RecordId {
//     fn get_id(&self) -> String {
//         match self {
//             Self::String(string) => string.clone(),
//             Self::Uuid(uuid) => uuid.to_string(),
//         }
//     }
// }

// impl Display for RecordId {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         match self {
//             _ => write!(f, "{}", self.to_string()),
//         }
//     }
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record<D> {
    id: String,
    data: D,
    tags: HashMap<String, String>,
}

impl<D: Serialize + DeserializeOwned> Record<D> {
    fn new(id: String, data: D, tags: HashMap<String, String>) -> Self {
        Self { id, data, tags }
    }
    fn to_string(&self) -> Result<String, StorageError> {
        serde_json::to_string(self).map_err(|err| StorageError::Serialization(err))
    }
    fn from_string(string: &str) -> Result<Self, StorageError> {
        serde_json::from_str(string).map_err(|err| StorageError::Deserialization(err))
    }
    fn add_or_update_tag(&mut self, tag_key: String, tag_value: String) -> () {
        self.tags.insert(tag_key, tag_value);
    }
    fn get_tag(&self, tag_key: &str) -> Option<&String> {
        self.tags.get(tag_key)
    }
    fn get_tags(&self) -> &HashMap<String, String> {
        &self.tags
    }
    fn delete_tag(&mut self, tag_key: &str) -> () {
        self.tags.remove(tag_key);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidRecordData {
    value: String,
}

fn test() {
    // let id = "IDString".to_owned();
    let id = "IDSTRING".to_string();
    let tags = HashMap::new();
    let record = Record::new(
        id,
        DidRecordData {
            value: "TestVal".to_owned(),
        },
        tags,
    );
    info!("{}", record.id);
    info!("Record: {:#?}", record);
}

// impl Record {
//     fn to_string(&self) -> Result<String, StorageError> {
//         serde_json::to_string(self).map_err(|err| StorageError::Serialization(err))
//     }
//     fn from_string(string: &str) -> Result<Self, StorageError> {
//         serde_json::from_str(string).map_err(|err| StorageError::Deserialization(err))
//     }
//     fn add_or_update_tag(&mut self, tag_key: String, tag_value: String) -> () {
//         self.tags.insert(tag_key, tag_value);
//     }
//     fn get_tag(&self, tag_key: &str) -> Option<&String> {
//         self.tags.get(tag_key)
//     }
//     fn get_tags(&self) -> &HashMap<String, String> {
//         &self.tags
//     }
//     fn delete_tag(&mut self, tag_key: &str) -> () {
//         self.tags.remove(tag_key);
//     }
// }

// pub struct DIDRecord {}

// impl Record for DIDRecord {}

// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// enum RecordId {
//     String(String),
//     Uuid(Uuid),
// }

// impl RecordId {
//     fn get_id(&self) -> String {
//         match self {
//             Self::String(string) => string.clone(),
//             Self::Uuid(uuid) => uuid.to_string(),
//         }
//     }
// }

// impl Display for RecordId {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         match self {
//             _ => write!(f, "{}", self.to_string()),
//         }
//     }
// }

// pub trait RecordData {
//     fn get_id(&self) -> String;
// }

// /// A general-purpose Record that takes generic data for use with [`VCXFrameworkStorage`]. The record provides methods for serializing from and deserializing to a String,
// /// and for getting, updating, and deleting record tags. Tags are used for querying records by metadata. `Record` also takes an ID
// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// pub struct Record<D> {
//     id: RecordId,
//     data: D,
//     tags: HashMap<String, String>,
// }

// impl<D: Serialize + DeserializeOwned + RecordData> Record<D> {
//     fn new(data: D, id: RecordId, tags: Vec<(String, String)>) -> Self {
//         let mut tags_map = HashMap::new();
//         for (tag_key, tag_value) in tags {
//             tags_map.insert(tag_key, tag_value);
//         }
//         Self {
//             id,
//             data,
//             tags: tags_map,
//         }
//     }
//     fn to_string(&self) -> Result<String, StorageError> {
//         serde_json::to_string(self).map_err(|err| StorageError::Serialization(err))
//     }
//     fn from_string(string: &str) -> Result<Self, StorageError> {
//         serde_json::from_str(string).map_err(|err| StorageError::Deserialization(err))
//     }
//     fn add_or_update_tag(&mut self, tag_key: String, tag_value: String) -> () {
//         self.tags.insert(tag_key, tag_value);
//     }
//     fn get_tag(&self, tag_key: &str) -> Option<&String> {
//         self.tags.get(tag_key)
//     }
//     fn get_tags(&self) -> &HashMap<String, String> {
//         &self.tags
//     }
//     fn delete_tag(&mut self, tag_key: &str) -> () {
//         self.tags.remove(tag_key);
//     }
// }

struct InMemoryStorage<D: Serialize + DeserializeOwned> {
    records: HashMap<String, String>,
    tags: Vec<(String, (String, String))>,
    // PhantomData is used so that the Record type must be determined at `new()`, which is required given that the Record type isn't specified in any of the struct fields.
    // This is done so that the type doesn't have to be inferred or manually set later during use.
    _phantom: PhantomData<D>,
}

impl<D: Serialize + DeserializeOwned> InMemoryStorage<D> {
    fn new() -> Self {
        InMemoryStorage::<D> {
            records: HashMap::new(),
            tags: vec![],
            _phantom: PhantomData,
        }
    }

    fn _add_keys(&mut self, tags: HashMap<String, String>, id: &String) -> () {
        for (tag_key, tag_value) in tags {
            self.tags.push((tag_key, (tag_value, id.clone())));
        }
    }

    fn _remove_keys(&mut self, id: &String) -> () {
        self.tags
            .retain(|(_tag_key, (_tag_value, stored_id))| id != stored_id);
    }
}

impl<D: Serialize + DeserializeOwned> VCXFrameworkStorage<D> for InMemoryStorage<D> {
    fn add_record(&mut self, record: Record<D>) -> Result<(), StorageError> {
        if self.records.contains_key(&record.id) {
            return Err(StorageError::DuplicateRecord);
        } else {
            self.records.insert(record.id.clone(), record.to_string()?);
            self._add_keys(record.get_tags().to_owned(), &record.id);
        }
        Ok(())
    }
    fn add_or_update_record(&mut self, record: Record<D>) -> Result<(), StorageError> {
        self.records.insert(record.id.clone(), record.to_string()?);
        self._remove_keys(&record.id);
        self._add_keys(record.get_tags().to_owned(), &record.id);
        Ok(())
    }
    fn update_record(&mut self, record: Record<D>) -> Result<(), StorageError> {
        if self.records.contains_key(&record.id.clone()) {
            self.records.insert(record.id.clone(), record.to_string()?);
            self._remove_keys(&record.id);
            self._add_keys(record.get_tags().to_owned(), &record.id);
            Ok(())
        } else {
            return Err(StorageError::RecordDoesNotExist);
        }
    }
    fn get_record(&self, id: String) -> Result<Option<Record<D>>, StorageError> {
        let record = self.records.get(&id);
        match record {
            Some(retrieved_record) => Ok(Some(Record::from_string(retrieved_record)?)),
            None => Ok(None),
        }
    }

    fn get_all_records(&self) -> Result<Vec<Record<D>>, StorageError> {
        let records = self
            .records
            .iter()
            .map(|(_id, retrieved_record)| Record::from_string(retrieved_record))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(records)
    }

    fn search_records(
        &self,
        tag_key: &str,
        tag_value: &str,
    ) -> Result<Vec<Record<D>>, StorageError> {
        let matching_ids: Vec<String> = self
            .tags
            .iter()
            .filter(|(stored_tag_key, (stored_tag_value, _stored_tag_id))| {
                tag_key == stored_tag_key && tag_value == stored_tag_value
            })
            .map(|tag| tag.1 .1.clone())
            .collect();
        let mut records = vec![];
        for id in matching_ids {
            if let Some(record) = self.get_record(id)? {
                records.push(record);
            }
        }
        Ok(records)
    }

    fn delete_record(&mut self, id: String) -> Result<(), StorageError> {
        self.records.remove(&id);
        self._remove_keys(&id);

        Ok(())
    }
}

// #[cfg(feature = "did_repository")]
// pub mod did_repository {
//     use super::VCXFrameworkStorage;

//     struct DIDRecord {
//         // I would prefer to have this be an actual DID type in the future, but that'll take work on the did_core crates - @JamesKEbert
//         did: String,
//     }

//     /// The `DidRepository` stores all created and known DIDs, and where appropriate, stores full DIDDocs (such as storing a long form did:peer:4 or with TTL caching strategies).
//     /// Otherwise, DID resolution should be done at runtime.
//     struct DidRepository<S: VCXFrameworkStorage<String, SimpleRecord>> {
//         store: S,
//     }

//     impl<S: VCXFrameworkStorage<String, SimpleRecord>> DidRepository<S> {
//         fn new(store: S) -> Self {
//             Self { store }
//         }

//         // fn get_did_record(did: String) -> DIDRecord {}
//     }
// }

#[cfg(test)]
mod tests {
    use crate::test_init;

    use super::*;

    #[test]
    fn testtest() {
        test_init();
        test();
    }
    // #[test]
    // fn test_add_and_read_record() {
    //     test_init();
    //     let mut in_memory_storage = InMemoryStorage::new();
    //     let record = Record::new("Foo".to_owned(), RecordId::String("bob".to_owned()), vec![]);
    //     let id = Uuid::new_v4();

    //     let _ = in_memory_storage.add_record(id, record.clone());
    //     let retrieved_record = in_memory_storage
    //         .get_record(id)
    //         .unwrap()
    //         .expect("Record to exist");
    //     assert_eq!(record, retrieved_record);
    // }

    // #[test]
    // fn test_add_duplicate() {
    //     test_init();
    //     let mut in_memory_storage = InMemoryStorage::new();
    //     let id = Uuid::new_v4();
    //     let record = Record::new("Foo".to_owned(), RecordId::Uuid(id), vec![]);
    //     let _ = in_memory_storage.add_record(id, record.clone());
    //     assert!(matches!(
    //         in_memory_storage.add_record(id, record),
    //         Err(StorageError::DuplicateRecord),
    //     ))
    // }

    // #[test]
    // fn test_add_or_update_record() {
    //     test_init();
    //     let mut in_memory_storage = InMemoryStorage::new();
    //     let record = Record::new("Foo".to_owned(), vec![]);
    //     let id = Uuid::new_v4();
    //     let _ = in_memory_storage.add_or_update_record(id, record.clone());
    //     let retrieved_record = in_memory_storage
    //         .get_record(id)
    //         .unwrap()
    //         .expect("Record to exist");
    //     assert_eq!(record, retrieved_record);
    // }

    // #[test]
    // fn test_update_record() {
    //     test_init();
    //     let mut in_memory_storage = InMemoryStorage::new();
    //     let record = Record::new("Foo".to_owned(), vec![]);
    //     let updated_record = Record::new("Foo2".to_owned(), vec![]);
    //     let id = Uuid::new_v4();
    //     let _ = in_memory_storage.add_record(id, record.clone());
    //     let _ = in_memory_storage.update_record(id, updated_record.clone());
    //     let retrieved_record = in_memory_storage
    //         .get_record(id)
    //         .unwrap()
    //         .expect("Record to exist");
    //     assert_eq!(updated_record, retrieved_record);
    // }

    // #[test]
    // fn test_update_record_no_record() {
    //     test_init();
    //     let mut in_memory_storage = InMemoryStorage::new();
    //     let updated_record = Record::new("Foo".to_owned(), vec![]);
    //     let id = Uuid::new_v4();

    //     assert!(matches!(
    //         in_memory_storage.update_record(id, updated_record),
    //         Err(StorageError::RecordDoesNotExist),
    //     ))
    // }

    // #[test]
    // fn test_delete_record() {
    //     test_init();
    //     let mut in_memory_storage = InMemoryStorage::new();
    //     let record = Record::new("Foo".to_owned(), vec![]);
    //     let id = Uuid::new_v4();
    //     let _ = in_memory_storage.add_record(id, record.clone());
    //     let _ = in_memory_storage
    //         .delete_record(id)
    //         .expect("To delete record");
    //     assert_eq!(None, in_memory_storage.get_record(id).unwrap());
    // }

    // #[test]
    // fn test_delete_record_already_deleted() {
    //     test_init();
    //     let mut in_memory_storage = InMemoryStorage::new();
    //     let record = Record::new("Foo".to_owned(), vec![]);
    //     let id = Uuid::new_v4();
    //     let _ = in_memory_storage.add_record(id, record.clone());
    //     let _ = in_memory_storage.delete_record(id);
    //     let _ = in_memory_storage.delete_record(id);
    //     assert_eq!(None, in_memory_storage.get_record(id).unwrap());
    // }
}
