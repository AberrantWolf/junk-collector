//! Execution budgets and bounded preference for finishing ready imports.
use crate::*;
use std::collections::VecDeque;

pub const IMPORT_COPY_JOB_KIND: &str = "import_staging";
pub const PROVIDER_JOB_SLOTS: u32 = 1;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct JobSettings {
    pub processing_workers: u8,
    pub copy_workers: u8,
}

impl Default for JobSettings {
    fn default() -> Self {
        Self {
            processing_workers: 4,
            copy_workers: 1,
        }
    }
}
impl JobSettings {
    pub fn validate(&self) -> Result<()> {
        if !(1..=8).contains(&self.processing_workers) {
            return Err(Error::Invalid(
                "jobs.processing_workers must be between 1 and 8".into(),
            ));
        }
        if self.copy_workers == 0 || self.copy_workers > self.processing_workers {
            return Err(Error::Invalid(
                "jobs.copy_workers must be between 1 and jobs.processing_workers".into(),
            ));
        }
        Ok(())
    }
    pub fn execution_loops(&self) -> usize {
        usize::from(self.processing_workers) + PROVIDER_JOB_SLOTS as usize
    }
}

/// Preferences are transient, shared across execution loops, and never own Jobs.
/// Two continuation admissions allow one copied disc's analysis/publication to
/// finish with one worker, then give copying its turn. Blocked work pins nothing.
#[derive(Default)]
pub struct JobDispatchPolicy {
    pub settings: JobSettings,
    processing_since_copy: u8,
    preferred_since_fifo: u8,
    finishing_import: Option<String>,
    imports: VecDeque<String>,
}
impl JobDispatchPolicy {
    pub fn prefers_copy(&self) -> bool {
        self.processing_since_copy >= 2
    }
    pub fn preferred_imports(&self) -> Vec<&str> {
        if self.preferred_since_fifo >= 2 {
            Vec::new()
        } else {
            self.imports.iter().map(String::as_str).collect()
        }
    }
    pub fn preferred_publication(&self) -> Option<&str> {
        self.finishing_import.as_deref()
    }
    pub fn prefers_metadata(&self) -> bool {
        self.preferred_since_fifo < 2
    }
    pub fn admitted(&mut self, kind: &str, user: UserId, request: &serde_json::Value) {
        if kind == "provider_acquisition" {
            return;
        }
        let import = request
            .get("import_id")
            .and_then(serde_json::Value::as_str)
            .and_then(|id| id.parse::<ImportId>().ok());
        let key = import.map(|id| format!("{user}:{id}"));
        let copies = kind == IMPORT_COPY_JOB_KIND
            || kind == "preservation_import"
                && request
                    .get("inbox_source_locator")
                    .and_then(serde_json::Value::as_str)
                    .is_some();
        if copies {
            self.processing_since_copy = 0;
            if let Some(key) = key {
                self.imports.retain(|entry| entry != &key);
                self.imports.push_back(key);
                if self.imports.len() > 8 {
                    self.imports.pop_front();
                }
            }
        } else {
            // Metadata and ready-disc continuations share one preference budget:
            // alternating the two must not starve ordinary processing.
            let preferred = matches!(kind, "metadata_resolution" | "metadata_existing")
                || matches!(kind, "import_analysis" | "import_publication")
                    && key.as_ref().is_some_and(|key| self.imports.contains(key));
            self.preferred_since_fifo = if preferred && self.preferred_since_fifo < 2 {
                self.preferred_since_fifo + 1
            } else {
                0
            };
            self.processing_since_copy = self.processing_since_copy.saturating_add(1).min(2);
            if let Some(key) = key {
                if kind == "import_analysis" {
                    self.finishing_import = Some(key.clone());
                    self.imports.retain(|entry| entry != &key);
                    self.imports.push_front(key);
                    self.imports.truncate(8);
                } else if kind == "import_publication" {
                    if self.finishing_import.as_ref() == Some(&key) {
                        self.finishing_import = None;
                    }
                    self.imports.retain(|entry| entry != &key);
                }
            }
        }
    }
}
