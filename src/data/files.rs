use std::fmt::Display;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, UNIX_EPOCH};

use chrono::Utc;
use humansize::format_size;
use itertools::Itertools;
use kv::Json;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use terminal_size::*;

use crate::nexus::NexusClient;
use crate::{Cacheable, CompoundKey};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FileInfo {
    category_id: u32,
    category_name: Option<String>,
    changelog_html: Option<String>,
    content_preview_link: String,
    description: String,
    external_virus_scan_url: String,
    file_id: usize,
    file_name: String,
    id: Vec<usize>,
    is_primary: bool,
    mod_version: String,
    name: String,
    size_in_bytes: u64,
    size_kb: usize,
    size: u64,
    uploaded_time: String,
    uploaded_timestamp: usize,
    uuid: Option<String>,
    version: String,
}

impl FileInfo {
    pub fn description_wrapped(&self) -> String {
        let width: usize = if let Some((Width(w), Height(_h))) = terminal_size() {
            w as usize - 2
        } else {
            72
        };
        textwrap::fill(&self.description, width)
    }

    pub fn freshness(&self) -> String {
        let d = UNIX_EPOCH + Duration::from_secs(self.uploaded_timestamp as u64);
        let datetime = chrono::DateTime::<Utc>::from(d);
        chrono_humanize::HumanTime::from(datetime).to_string()
    }

    pub fn print_compact_info(&self) {
        println!(
            "{}: {} @ {} (cat: {})",
            self.file_id,
            self.name.bold().green(),
            self.version,
            self.category_id
        );
        println!(
            "\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\",
            self.content_preview_link,
            self.file_name.blue().bold(),
        );
        println!(
            "{} / {}",
            format_size(self.size_in_bytes, humansize::DECIMAL),
            self.freshness()
        );
    }

    pub fn print_full_info(&self) {
        println!(
            "{} @ {}  id: {}",
            self.name.bold().green(),
            self.version,
            self.file_id
        );
        println!(
            "\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\",
            self.content_preview_link,
            self.file_name.blue().bold(),
        );
        println!("{}", format_size(self.size_in_bytes, humansize::DECIMAL));
        println!("{}", self.freshness());
        println!("\n{}\n", self.description_wrapped());

        // changelog; same hack as mod markdown

        let Some(text) = self.changelog_html.clone() else {
            return;
        };
        let text: String = text.split('\n').collect::<Vec<&str>>().join("\n- ");
        let text = format!("## CHANGELOG\n\n - {text}");

        let subproc = Command::new("mdcat").arg("-").stdin(Stdio::piped()).spawn();
        if let Ok(mut subproc) = subproc {
            let substdin = subproc.stdin.as_mut().unwrap();
            if substdin.write_all(text.as_bytes()).is_ok() {
                let _result = substdin.flush();
                let _output = subproc.wait_with_output();
                return;
            }
        }

        let subproc = Command::new("glow").arg("-").stdin(Stdio::piped()).spawn();
        if let Ok(mut subproc) = subproc {
            let substdin = subproc.stdin.as_mut().unwrap();
            if substdin.write_all(text.as_bytes()).is_ok() {
                let _result = substdin.flush();
                let _output = subproc.wait_with_output();
                return;
            }
        }

        // Fall back to printing the html.
        println!("{text}");
    }
}

impl Display for FileInfo {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FileUpdates {
    old_file_id: usize,
    new_file_id: usize,
    old_file_name: String,
    new_file_name: String,
    uploaded_timestamp: usize,
    uploaded_time: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct Files {
    domain_name: String,
    mod_id: u32,
    etag: String,
    files: Vec<FileInfo>,
    file_updates: Vec<FileUpdates>,
}

impl Files {
    pub fn files(&self) -> &Vec<FileInfo> {
        &self.files
    }

    pub fn current_files(&self) -> Vec<FileInfo> {
        let mut files: Vec<&FileInfo> = self
            .files()
            .iter()
            .filter(|xs| xs.category_id != 7)
            .collect();
        files.sort_by_key(|xs| xs.uploaded_timestamp);
        files.iter().rev().map(|xs| (*xs).clone()).collect_vec()
    }

    pub fn file_by_id(&self, file_id: usize) -> Option<FileInfo> {
        self.files.iter().find(|xs| xs.file_id == file_id).cloned()
    }

    pub fn primary_file(&self) -> Option<FileInfo> {
        self.files.iter().find(|xs| xs.is_primary).cloned()
    }
}

impl Default for Files {
    fn default() -> Self {
        Files {
            domain_name: "".to_string(),
            mod_id: 0,
            etag: "".to_string(),
            files: Vec::new(),
            file_updates: Vec::new(),
        }
    }
}

impl Cacheable for Files {
    type K = CompoundKey;

    fn bucket_name() -> &'static str {
        "files"
    }

    fn get(key: &CompoundKey, refresh: bool, nexus: &mut NexusClient) -> Option<Box<Self>> {
        super::get::<Self>(key, refresh, nexus)
    }

    fn fetch(
        key: &CompoundKey,
        nexus: &mut NexusClient,
        etag: Option<String>,
    ) -> Option<Box<Self>> {
        // The game & modid are *not* included in the response data. This is okay, but I want it.
        nexus
            .files(&key.domain_name, key.mod_id, etag)
            .map(|mut v| {
                v.domain_name = key.domain_name.clone();
                v.mod_id = key.mod_id;
                Box::new(v)
            })
    }

    fn key(&self) -> CompoundKey {
        CompoundKey {
            domain_name: self.domain_name.clone(),
            mod_id: self.mod_id,
        }
    }

    fn etag(&self) -> &str {
        &self.etag
    }

    fn set_etag(&mut self, etag: &str) {
        self.etag = etag.to_string()
    }

    fn store(&self) -> anyhow::Result<usize> {
        let bucket = super::bucket::<Self>().unwrap();
        if bucket
            .set(&&*self.key().to_string(), &Json(self.clone()))
            .is_ok()
        {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    fn update(&self, other: &Self) -> Self {
        other.clone()
    }
}
