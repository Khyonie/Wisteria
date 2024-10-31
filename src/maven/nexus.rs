use reqwest::blocking::get;
use serde::Deserialize;
use serde_json::from_str;

#[derive(Deserialize)]
pub struct NexusEntry
{
    version: String
}

#[derive(Deserialize)]
pub struct NexusItems
{
    items: Vec<NexusEntry>
}

pub fn items(url: &str, group_id: &str, artifact_id: &str) -> Result<Vec<NexusEntry>, String>
{
    let url = format!("{url}?group={group_id}&name={artifact_id}");
    println!("{url}");

    let response = get(&url).map_err(| e | format!("{e}"))?
        .text().unwrap();

    println!("{response}");

    let items: NexusItems = from_str(&response).map_err(| e | format!("{e}"))?;

    println!("Nexus items");
    for i in &items.items
    {
        println!("{}", i.version);
    }

    Ok(items.items)
}
