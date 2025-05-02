use anyhow::anyhow;
use serde::{Deserialize, Serialize};

const SUBJECT_BINDING: &str = "$sbm";
const SUBJECT_BINDING_TYPE: &str = "$type";

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
    pub fn to_query_op(&self) -> String {
        let path = self
            .paths
            .iter()
            .map(|p| format!("<{p}>"))
            .collect::<Vec<_>>()
            .join("/");
        if self.optional {
            format!("OPTIONAL {{ ?{SUBJECT_BINDING} {path} ?{} }}", self.name)
        } else {
            format!("?{SUBJECT_BINDING} {path} ?{}", self.name)
        }
    }
    pub fn get_var(&self) -> String {
        format!("?{}", self.name)
    }
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.name == SUBJECT_BINDING || self.name == SUBJECT_BINDING_TYPE {
            return Err(anyhow!(
                "you cannot name a property with {SUBJECT_BINDING} or {SUBJECT_BINDING_TYPE} in your config, because it's used internally."
            ));
        }
        Ok(())
    }
}

impl IndexConfiguration {
    pub fn to_type_op(&self) -> String {
        let target_types = self
            .rdf_type
            .iter()
            .map(|t| format!("<{t}>"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            r#"
          VALUES ?{SUBJECT_BINDING_TYPE} {{
                {target_types}
            }}
        "#
        )
    }
}
