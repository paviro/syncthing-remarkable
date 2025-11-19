use serde::Serialize;

#[derive(Serialize)]
pub struct FolderStatusQuery<'a> {
    pub folder: &'a str,
}

#[derive(Serialize)]
pub struct EventsQuery {
    pub since: u64,
    pub limit: u32,
}

#[derive(Serialize)]
pub struct EventStreamQuery<'a> {
    pub since: u64,
    pub limit: u32,
    pub timeout: u64,
    #[serde(rename = "events", skip_serializing_if = "Option::is_none")]
    pub events: Option<&'a [&'a str]>,
}

#[derive(Serialize)]
pub struct CompletionQuery<'a> {
    pub device: &'a str,
    pub folder: &'a str,
}

