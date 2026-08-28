use serde::Deserialize;
#[derive(Debug, Clone, Deserialize)]
pub struct Chapter { pub title: String, pub start_offset_ms: u64, pub length_ms: u64, #[serde(default)] chapters: Option<Vec<Chapter>> }

/// Flatten the (possibly nested) `chapter_info` from a license response.
pub fn from_license(ci: &Option<serde_json::Value>) -> Vec<Chapter> {
    let mut out = Vec::new();
    if let Some(list) = ci.as_ref().and_then(|v| v.get("chapters")).and_then(|c| serde_json::from_value::<Vec<Chapter>>(c.clone()).ok()) {
        flatten(list, &mut out);
    }
    out.sort_by_key(|c| c.start_offset_ms);
    out
}
fn flatten(list: Vec<Chapter>, out: &mut Vec<Chapter>) {
    for mut c in list {
        let kids = c.chapters.take();
        let has_kids = kids.as_ref().is_some_and(|k| !k.is_empty());
        if !has_kids || c.length_ms > 0 { out.push(c); }
        if let Some(k) = kids { flatten(k, out); }
    }
}
pub fn index_at(chapters: &[Chapter], ms: u64) -> Option<usize> {
    chapters.iter().rposition(|c| c.start_offset_ms <= ms)
}
