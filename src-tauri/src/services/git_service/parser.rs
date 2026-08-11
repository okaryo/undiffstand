use crate::domain::{DiffFileSummary, DiffStatus};

pub(super) fn parse_name_status(bytes: &[u8]) -> Vec<DiffFileSummary> {
    let fields: Vec<&[u8]> = bytes
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect();
    let mut files = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let status_text = String::from_utf8_lossy(fields[index]);
        index += 1;
        let status_code = status_text.chars().next().unwrap_or('M');
        let path = fields
            .get(index)
            .map(|field| String::from_utf8_lossy(field).into_owned());
        index += 1;
        let (old_path, new_path, status) = match status_code {
            'A' => (None, path, DiffStatus::Added),
            'D' => (path, None, DiffStatus::Deleted),
            'R' | 'C' => {
                let new_path = fields
                    .get(index)
                    .map(|field| String::from_utf8_lossy(field).into_owned());
                index += 1;
                (
                    path,
                    new_path,
                    if status_code == 'R' {
                        DiffStatus::Renamed
                    } else {
                        DiffStatus::Copied
                    },
                )
            }
            _ => (path.clone(), path, DiffStatus::Modified),
        };
        files.push(DiffFileSummary {
            old_path,
            new_path,
            status,
            additions: None,
            deletions: None,
        });
    }
    files
}

pub(super) fn parse_numstat(bytes: &[u8]) -> Vec<(Option<u64>, Option<u64>)> {
    let fields: Vec<&[u8]> = bytes.split(|byte| *byte == 0).collect();
    let mut stats = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let field = fields[index];
        index += 1;
        if field.is_empty() {
            continue;
        }
        let mut parts = field.splitn(3, |byte| *byte == b'\t');
        let additions = parts.next().unwrap_or_default();
        let deletions = parts.next().unwrap_or_default();
        let path = parts.next().unwrap_or_default();
        if path.is_empty() {
            index = (index + 2).min(fields.len());
        }
        let parse_count = |value: &[u8]| String::from_utf8_lossy(value).parse::<u64>().ok();
        stats.push((parse_count(additions), parse_count(deletions)));
    }
    stats
}

pub(super) fn split_hunks(diff: &str) -> Vec<String> {
    let mut hunks = Vec::new();
    let mut current = String::new();
    for line in diff.lines() {
        if line.starts_with("@@ ") {
            if !current.is_empty() {
                hunks.push(std::mem::take(&mut current));
            }
            current.push_str(line);
            current.push('\n');
        } else if !current.is_empty() {
            current.push_str(line);
            current.push('\n');
        }
    }
    if !current.is_empty() {
        hunks.push(current);
    }
    hunks
}
