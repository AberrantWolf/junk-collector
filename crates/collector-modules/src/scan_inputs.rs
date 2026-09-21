//! Name-level operation choices and normalization. No payload reads or claims of validation.
use crate::{IngestAnchorChoice, ModuleError, ModuleResult};
use collector_core::{PackageInventory, RawJson};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScanInputs {
    #[schema(value_type = Object)]
    pub selection: RawJson,
    pub files: Vec<IngestAnchorChoice>,
    pub blockers: Vec<String>,
}
impl ScanInputs {
    pub fn new(input: &RawJson) -> ModuleResult<Self> {
        if input.get().len() > 64 * 1024 {
            return Err(ModuleError::Invalid("Selection is oversized".into()));
        }
        let value: serde_json::Value = input
            .parse()
            .map_err(|e| ModuleError::Invalid(format!("Invalid selection: {e}")))?;
        if !value.is_object() {
            return Err(ModuleError::Invalid(
                "Selection must be a bounded object".into(),
            ));
        }
        Ok(Self {
            selection: input.clone(),
            files: vec![],
            blockers: vec![],
        })
    }
    pub fn file(
        &mut self,
        inventory: &PackageInventory,
        field: &str,
        eligible: impl Fn(&str) -> bool,
    ) -> ModuleResult<()> {
        let mut input: serde_json::Value = self
            .selection
            .parse()
            .map_err(|e| ModuleError::Invalid(format!("Invalid selection: {e}")))?;
        let selected = input
            .get(field)
            .and_then(serde_json::Value::as_u64)
            .and_then(|v| usize::try_from(v).ok());
        if input.get(field).is_some() && selected.is_none() {
            self.blockers
                .push("The file index must be a nonnegative integer.".into());
            return Ok(());
        }
        self.files = inventory
            .files
            .iter()
            .enumerate()
            .filter(|(_, v)| eligible(v.display_name.as_str()))
            .map(|(index, file)| IngestAnchorChoice {
                file_index: index,
                label: file.display_name.as_str().into(),
                selected: selected == Some(index),
            })
            .collect();
        let chosen = selected.or_else(|| (self.files.len() == 1).then(|| self.files[0].file_index));
        if let Some(index) =
            chosen.filter(|index| self.files.iter().any(|v| v.file_index == *index))
        {
            input[field] = index.into();
            for file in &mut self.files {
                file.selected = file.file_index == index;
            }
        } else {
            self.blockers.push(
                if self.files.is_empty() {
                    "No eligible package file is available."
                } else if selected.is_some() {
                    "The selected file is not eligible for this operation."
                } else {
                    "Choose the package file to check."
                }
                .into(),
            );
        }
        self.selection = RawJson::from_value(input);
        Ok(())
    }
    pub fn normalize<T: serde::de::DeserializeOwned + Serialize>(&mut self) -> ModuleResult<()> {
        if !self.blockers.is_empty() {
            return Ok(());
        }
        let typed: T = self
            .selection
            .parse()
            .map_err(|e| ModuleError::Invalid(e.to_string()))?;
        self.selection = RawJson::from_value(
            serde_json::to_value(typed).map_err(|e| ModuleError::Invalid(e.to_string()))?,
        );
        Ok(())
    }
}
