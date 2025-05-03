use anyhow::anyhow;
use serde::{Deserialize, Serialize};

const SUBJECT_BINDING_TYPE: &str = "$type";
pub static INDEX_ID_KEY: &str = "_id";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexConfiguration {
    pub name: String,
    pub rdf_type: Vec<String>,
    pub on_path: String,
    pub properties: Vec<RdfProperty>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RdfProperty {
    pub name: String,
    pub paths: Vec<String>,
    pub optional: bool,
}

impl RdfProperty {
    pub fn to_query_op(&self, subject: &str) -> String {
        let path = self
            .paths
            .iter()
            .map(|p| {
                if let Some(stripped) = p.strip_prefix('^') {
                    format!("^<{}>", stripped)
                } else {
                    format!("<{p}>")
                }
            })
            .collect::<Vec<_>>()
            .join("/");
        if self.optional {
            format!("OPTIONAL {{ <{subject}> {path} ?{} }}", self.name)
        } else {
            format!("<{subject}> {path} ?{}", self.name)
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.name == SUBJECT_BINDING_TYPE || self.name == INDEX_ID_KEY {
            return Err(anyhow!(
                "you cannot name a property with {SUBJECT_BINDING_TYPE} or {INDEX_ID_KEY} in your config, because it's used internally."
            ));
        }
        Ok(())
    }
}
