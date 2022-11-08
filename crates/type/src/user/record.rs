// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::fmt;

use serde::{
    de::{self, Deserializer, MapAccess, Visitor},
    Deserialize, Serialize,
};

/// A user record
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    /// OpenID Connect identity subjects uniquely identifying the users with access
    pub subjects: Vec<String>,
}

impl Record {
    pub fn new(subject: String) -> Self {
        Self {
            subjects: vec![subject],
        }
    }

    pub fn contains_subject(&self, subject: &str) -> bool {
        self.subjects.iter().any(|s| s == subject)
    }
}

/// Custom Deserializer, in order to allow either the old singular Subject or the new
/// plural Subjects fields.
impl<'de> Deserialize<'de> for Record {
    fn deserialize<D>(deserializer: D) -> Result<Record, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Subject,
            Subjects,
        }

        struct RecordVisitor;

        #[allow(single_use_lifetimes)]
        impl<'de> Visitor<'de> for RecordVisitor {
            type Value = Record;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("user record struct")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Record, V::Error>
            where
                V: MapAccess<'de>,
            {
                if let Some(key) = map.next_key()? {
                    match key {
                        Field::Subject => {
                            let subject = map.next_value()?;
                            return Ok(Record {
                                subjects: vec![subject],
                            });
                        }
                        Field::Subjects => {
                            let subjects = map.next_value()?;
                            return Ok(Record { subjects });
                        }
                    }
                }
                Err(de::Error::missing_field("subjects"))
            }
        }

        const FIELDS: &[&str] = &["subject", "subjects"];
        deserializer.deserialize_struct("Record", FIELDS, RecordVisitor)
    }
}
