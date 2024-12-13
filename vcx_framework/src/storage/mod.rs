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
    fn get_record(&self, id: &String) -> Result<Option<Record<D>>, StorageError>;

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
    fn delete_record(&mut self, id: &String) -> Result<(), StorageError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Record<D> {
    id: String,
    data: D,
    tags: HashMap<String, String>,
}

impl<D: Serialize + DeserializeOwned> Record<D> {
    fn new(id: String, data: D, tags: Option<HashMap<String, String>>) -> Self {
        Self {
            id,
            data,
            tags: tags.unwrap_or(HashMap::new()),
        }
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
    let id = String::from("IDSTRING");
    let tags = HashMap::new();
    let record = Record::new(
        id,
        DidRecordData {
            value: String::from("TestVal"),
        },
        Some(tags),
    );
    info!("{}", record.id);
    info!("Record: {:#?}", record);
}

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
        if self.records.contains_key(&record.id) {
            self.records.insert(record.id.clone(), record.to_string()?);
            self._remove_keys(&record.id);
            self._add_keys(record.get_tags().to_owned(), &record.id);
            Ok(())
        } else {
            return Err(StorageError::RecordDoesNotExist);
        }
    }
    fn get_record(&self, id: &String) -> Result<Option<Record<D>>, StorageError> {
        let record = self.records.get(id);
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
            if let Some(record) = self.get_record(&id)? {
                records.push(record);
            }
        }
        Ok(records)
    }

    fn delete_record(&mut self, id: &String) -> Result<(), StorageError> {
        self.records.remove(id);
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
    #[test]
    fn test_add_and_read_record() {
        test_init();
        let mut in_memory_storage = InMemoryStorage::new();
        let id = String::from("id1");
        let record = Record::new(id.clone(), String::from("foo"), None);

        in_memory_storage.add_record(record.clone()).unwrap();
        let retrieved_record = in_memory_storage
            .get_record(&id)
            .unwrap()
            .expect("Record to exist");
        assert_eq!(record, retrieved_record);
    }

    #[test]
    fn test_add_duplicate() {
        test_init();
        let mut in_memory_storage = InMemoryStorage::new();
        let id = String::from("id1");
        let record = Record::new(id, String::from("foo"), None);

        in_memory_storage.add_record(record.clone()).unwrap();
        assert!(matches!(
            in_memory_storage.add_record(record),
            Err(StorageError::DuplicateRecord),
        ))
    }

    #[test]
    fn test_add_or_update_record() {
        test_init();
        let mut in_memory_storage = InMemoryStorage::new();
        let id = String::from("id1");
        let record = Record::new(id.clone(), String::from("foo"), None);

        in_memory_storage
            .add_or_update_record(record.clone())
            .unwrap();
        let retrieved_record = in_memory_storage
            .get_record(&id)
            .unwrap()
            .expect("Record to exist");
        assert_eq!(record, retrieved_record);
    }

    #[test]
    fn test_update_record() {
        test_init();
        let mut in_memory_storage = InMemoryStorage::new();
        let id = String::from("id1");
        let record = Record::new(id.clone(), String::from("foo"), None);
        let updated_record = Record::new(id.clone(), String::from("Foo2"), None);

        in_memory_storage.add_record(record.clone()).unwrap();
        in_memory_storage
            .update_record(updated_record.clone())
            .unwrap();
        let retrieved_record = in_memory_storage
            .get_record(&id)
            .unwrap()
            .expect("Record to exist");
        assert_eq!(updated_record, retrieved_record);
    }

    #[test]
    fn test_update_record_no_record() {
        test_init();
        let mut in_memory_storage = InMemoryStorage::new();
        let id = String::from("id1");
        let updated_record = Record::new(id.clone(), String::from("Foo2"), None);

        assert!(matches!(
            in_memory_storage.update_record(updated_record),
            Err(StorageError::RecordDoesNotExist),
        ))
    }

    #[test]
    fn test_delete_record() {
        test_init();
        let mut in_memory_storage = InMemoryStorage::new();
        let id = String::from("id1");
        let record = Record::new(id.clone(), String::from("foo"), None);

        in_memory_storage.add_record(record.clone()).unwrap();
        in_memory_storage
            .delete_record(&id)
            .expect("To delete record");
        assert_eq!(None, in_memory_storage.get_record(&id).unwrap());
    }

    #[test]
    fn test_delete_record_already_deleted() {
        test_init();
        let mut in_memory_storage = InMemoryStorage::new();
        let id = String::from("id1");
        let record = Record::new(id.clone(), String::from("foo"), None);

        in_memory_storage.add_record(record.clone()).unwrap();
        in_memory_storage.delete_record(&id).unwrap();
        in_memory_storage.delete_record(&id).unwrap();
        assert_eq!(None, in_memory_storage.get_record(&id).unwrap());
    }
}
