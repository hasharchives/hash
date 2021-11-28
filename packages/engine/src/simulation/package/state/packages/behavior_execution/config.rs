use std::collections::HashMap;
// use std::convert::TryFrom;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

use super::fields::behavior::BehaviorMap;
use super::{Error, Result};
use crate::Language;

// TODO: Package's experiment init message should have payload with
//       Vec of behavior descriptions in `behavior_descs`.
#[derive(Serialize, Deserialize)]
pub struct BehaviorDescription {
    pub index: BehaviorIndex,
    pub name: String,
    pub short_names: Vec<String>,
    pub source: String,
    pub required_field_keys: Vec<String>,
    pub language: Language, // serde serialized to "Python", "JavaScript" or "Rust"
    pub dyn_access: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BehaviorIndex(u16, u16);

impl BehaviorIndex {
    pub fn lang_index(&self) -> u16 {
        self.0
    }

    pub fn lang_behavior_index(&self) -> u16 {
        self.1
    }
}

pub struct BehaviorIndices {
    name_to_index: HashMap<Vec<u8>, BehaviorIndex>,
    index_to_name: HashMap<BehaviorIndex, String>,
}

impl BehaviorIndices {
    pub(crate) fn from_behaviors(behaviors: &BehaviorMap) -> Result<BehaviorIndices> {
        let mut lang_counts = [0_u16; Language::NUM];

        let mut index_to_name = HashMap::new();
        let mut name_to_index = HashMap::new();
        for behavior in behaviors.iter_behaviors() {
            let shared = behavior.shared();
            let lang_index = Language::from_file_name(&shared.name)
                .map_err(|_| Error::from(format!("Invalid behavior name: \"{}\"", &shared.name)))?
                .as_index();
            let behavior_index = BehaviorIndex(lang_index as u16, lang_counts[lang_index]);
            lang_counts[lang_index] += 1;

            index_to_name.insert(behavior_index, shared.name.clone());
            name_to_index.insert(shared.name.clone().into_bytes(), behavior_index);
            for alt_name in shared.shortnames.iter() {
                name_to_index.insert(alt_name.clone().into_bytes(), behavior_index);
            }
        }

        Ok(BehaviorIndices {
            index_to_name,
            name_to_index,
        })
    }

    pub fn get_index<K: Deref<Target = [u8]>>(&self, key: &K) -> Option<&BehaviorIndex> {
        self.name_to_index.get(key.deref())
    }

    pub fn get_name(&self, behavior_index: &BehaviorIndex) -> Option<&String> {
        self.index_to_name.get(behavior_index)
    }
}

#[derive(Clone)]
pub struct BehaviorName(Vec<u8>);

impl BehaviorName {
    pub fn from_string(s: String) -> BehaviorName {
        BehaviorName(s.as_bytes().to_vec())
    }

    pub fn from_str<K: AsRef<str>>(s: K) -> BehaviorName {
        BehaviorName(s.as_ref().as_bytes().to_vec())
    }

    pub fn as_str(&self) -> &str {
        // Safe as creation only possible through strings
        std::str::from_utf8(&self.0).unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendableBehaviorKeys {
    field_names: Vec<String>,
    dyn_access: bool,
    built_in_key_use: Option<Vec<String>>,
}

pub fn exp_init_message(
    behavior_indices: &BehaviorIndices,
    behavior_map: &BehaviorMap,
) -> Result<Vec<BehaviorDescription>> {
    let behavior_descriptions = behavior_map
        .inner
        .iter()
        .map(|(file_name, behavior)| {
            let shared = behavior.shared();
            let keys = behavior.keys();

            let language = Language::from_file_name(file_name)
                .map_err(|_| Error::from("Couldn't get language from behavior file name"))?;
            let index = behavior_indices
                .name_to_index
                .get(shared.name.as_bytes())
                .ok_or(Error::from("Couldn't get index from behavior name"))?
                .clone();
            let source = shared
                .behavior_src
                .clone()
                .ok_or(Error::from("SharedBehavior didn't have an attached source"))?;
            let required_field_keys = keys
                .inner
                .iter()
                .map(|(key, _)| key.value().to_string())
                .chain(
                    keys.built_in_key_use
                        .iter()
                        .flat_map(|keys| keys.into_iter().cloned()),
                )
                .collect::<Vec<_>>();

            Ok(BehaviorDescription {
                index,
                name: shared.name.to_string(),
                short_names: shared.shortnames.clone(),
                source,
                required_field_keys,
                language,
                dyn_access: keys.dyn_access,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(behavior_descriptions)
}
