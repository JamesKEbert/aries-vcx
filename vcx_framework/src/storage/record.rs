use std::{collections::HashMap, hash::Hash};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::error::StorageError;

/// A general purpose record that can take generic data `D` (as long as it's serializable and deserializable), an id, and a set of tags for applying metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Record<D, TK: Eq + Hash> {
    pub id: String,
    pub data: D,
    pub tags: HashMap<TK, String>,
}

impl<D, TK> Record<D, TK>
where
    D: Serialize + DeserializeOwned + std::fmt::Debug,
    TK: Eq + Hash + Clone + std::fmt::Debug + Serialize + DeserializeOwned,
{
    pub fn new(id: String, data: D, tags: Option<HashMap<TK, String>>) -> Self {
        Self {
            id,
            data,
            tags: tags.unwrap_or_default(),
        }
    }
    pub fn to_string(&self) -> Result<String, StorageError> {
        serde_json::to_string(self).map_err(StorageError::Serialization)
    }
    pub fn from_string(string: &str) -> Result<Self, StorageError> {
        serde_json::from_str(string).map_err(StorageError::Deserialization)
    }
    pub fn add_or_update_tag(&mut self, tag_key: TK, tag_value: String) {
        self.tags.insert(tag_key, tag_value);
    }
    pub fn get_tag(&self, tag_key: &TK) -> Option<&String> {
        self.tags.get(tag_key)
    }
    pub fn get_tags(&self) -> &HashMap<TK, String> {
        &self.tags
    }
    pub fn delete_tag(&mut self, tag_key: &TK) {
        self.tags.remove(tag_key);
    }
}
