use serde::{Deserialize, Deserializer};

fn none_if_empty<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    Ok(s.filter(|str| !str.is_empty()))
}

#[derive(Deserialize)]
struct MyStruct {
    #[serde(deserialize_with = "none_if_empty")]
    _x: Option<String>,
}
