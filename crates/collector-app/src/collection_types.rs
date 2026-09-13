//! Local collection ownership inspection and assignment policy.
use super::*;

#[derive(Clone, Debug)]
pub struct CollectionTypePendingReference {
    pub id: String,
    pub module: Option<ModuleId>,
    pub operation: Option<OperationId>,
    pub description: String,
}

#[derive(Clone, Debug)]
pub struct CollectionTypeAuthorityFact {
    pub item_id: ItemId,
    pub authority: MetadataAuthority,
}

#[derive(Clone, Debug)]
pub struct CollectionTypeSnapshot {
    pub collection: Collection,
    pub item_domains: Vec<(ItemId, ModuleId)>,
    pub pending: Vec<CollectionTypePendingReference>,
    pub authorities: Vec<CollectionTypeAuthorityFact>,
}

/// Mechanism-only access for the local collection-type administrator. Policy,
/// module availability and eligibility remain in the application service.
pub trait CollectionTypePort: Send + Sync {
    fn collection_type_snapshot(&self, collection: CollectionId) -> Result<CollectionTypeSnapshot>;
    fn publish_collection_binding(
        &self,
        expected: Revision,
        revision: CollectionBindingRevision,
    ) -> Result<Collection>;
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct CollectionTypeInspection {
    pub collection: Collection,
    pub item_domain_counts: std::collections::BTreeMap<String, usize>,
    pub eligible_modules: Vec<ModuleId>,
    pub eligible: bool,
    pub unresolved: Vec<String>,
    pub blocking_ids: Vec<String>,
}

/// Local administrative policy for an exclusively opened, stopped-server store.
/// The caller supplies local administrator authorization and excludes workers for
/// the entire inspection/assignment operation; this is not a remote API service.
pub struct CollectionTypeAdministrationService {
    port: Arc<dyn CollectionTypePort>,
    modules: Arc<Registry>,
}

impl CollectionTypeAdministrationService {
    pub fn new<P: CollectionTypePort + 'static>(port: Arc<P>, modules: Arc<Registry>) -> Self {
        Self { port, modules }
    }

    pub fn inspect(&self, collection: CollectionId) -> Result<CollectionTypeInspection> {
        let snapshot = self.port.collection_type_snapshot(collection)?;
        Ok(self.plan(snapshot, None))
    }

    pub fn assign(
        &self,
        collection: CollectionId,
        expected: Revision,
        module: ModuleId,
        assigned_at: DateTime<Utc>,
    ) -> Result<Collection> {
        if !self.modules.has_module(&module) {
            return Err(Error::Unavailable(format!(
                "collection module {} is not registered",
                module.0
            )));
        }
        let snapshot = self.port.collection_type_snapshot(collection)?;
        if let CollectionBinding::Assigned { module: current } = &snapshot.collection.binding {
            return if current == &module
                && expected.0.checked_add(1) == Some(snapshot.collection.revision.0)
            {
                Ok(snapshot.collection)
            } else {
                Err(Error::Conflict)
            };
        }
        if snapshot.collection.revision != expected {
            return Err(Error::Conflict);
        }
        let plan = self.plan(snapshot, Some(&module));
        if !plan.eligible || !plan.blocking_ids.is_empty() {
            return Err(Error::Invalid(format!(
                "collection cannot be assigned to {}: {}",
                module.0,
                plan.unresolved.join("; ")
            )));
        }
        self.port.publish_collection_binding(
            expected,
            CollectionBindingRevision {
                collection_id: collection,
                revision: Revision(expected.0.checked_add(1).ok_or(Error::Conflict)?),
                binding: CollectionBinding::Assigned { module },
                assigned_at,
                provenance: "explicit_local_administrator_assignment".into(),
            },
        )
    }

    fn plan(
        &self,
        snapshot: CollectionTypeSnapshot,
        chosen: Option<&ModuleId>,
    ) -> CollectionTypeInspection {
        let mut counts = std::collections::BTreeMap::new();
        for (_, module) in &snapshot.item_domains {
            *counts.entry(module.0.clone()).or_insert(0) += 1;
        }
        let mut unresolved = Vec::new();
        let mut blocking_ids = Vec::new();
        for authority in &snapshot.authorities {
            if let Some(overrides) = &authority.authority.domain_overrides {
                let module = &overrides.module;
                unresolved.push(format!(
                    "item {} has current domain metadata authority for module {}",
                    authority.item_id, module.0
                ));
                if let Err(error) = self
                    .modules
                    .validate_metadata_authority(module, &authority.authority)
                {
                    unresolved.push(format!(
                        "item {} has invalid active metadata authority: {}",
                        authority.item_id, error
                    ));
                    blocking_ids.push(authority.item_id.to_string());
                }
            }
            for (index, preserved) in authority
                .authority
                .preserved_field_overrides
                .iter()
                .enumerate()
            {
                unresolved.push(format!(
                    "item {} preserved historical metadata field override {} remains provenance only: {}",
                    authority.item_id,
                    index + 1,
                    preserved.get()
                ));
            }
        }
        if counts.len() > 1 {
            unresolved.push(
                "current items have mixed domain owners; splitting or moving the graph is a separate task"
                    .into(),
            );
        }
        for name in counts.keys() {
            let id = ModuleId(name.clone());
            if !self.modules.has_module(&id) {
                unresolved.push(format!("current items use unavailable module {name}"));
            }
        }
        let mut eligible_modules = self
            .modules
            .descriptors()
            .into_iter()
            .filter(|descriptor| {
                snapshot
                    .item_domains
                    .iter()
                    .all(|(_, module)| module == &descriptor.id)
            })
            .map(|descriptor| descriptor.id)
            .collect::<Vec<_>>();
        for pending in &snapshot.pending {
            let owner = self.pending_owner(pending);
            eligible_modules.retain(|module| self.pending_compatible(pending, module));
            if chosen.is_none() {
                unresolved.push(format!(
                    "{} {} currently targets {}",
                    pending.description,
                    pending.id,
                    owner.map_or_else(
                        || "an unavailable operation owner".into(),
                        |id| format!("module {}", id.0)
                    )
                ));
            }
        }
        for authority in &snapshot.authorities {
            eligible_modules.retain(|module| {
                self.modules
                    .validate_metadata_authority(module, &authority.authority)
                    .is_ok()
            });
        }
        if let Some(module) = chosen {
            for (item, owner) in &snapshot.item_domains {
                if owner != module {
                    blocking_ids.push(item.to_string());
                    unresolved.push(format!("item {} belongs to module {}", item, owner.0));
                }
            }
            for pending in &snapshot.pending {
                let owner = self.pending_owner(pending);
                if !self.pending_compatible(pending, module) {
                    blocking_ids.push(pending.id.clone());
                    unresolved.push(format!(
                        "{} {} targets {}",
                        pending.description,
                        pending.id,
                        owner.map_or_else(
                            || "an unavailable operation owner".into(),
                            |id| format!("module {}", id.0)
                        )
                    ));
                }
            }
            for authority in &snapshot.authorities {
                if let Err(error) = self
                    .modules
                    .validate_metadata_authority(module, &authority.authority)
                {
                    blocking_ids.push(authority.item_id.to_string());
                    unresolved.push(format!(
                        "item {} has invalid or incompatible metadata authority: {}",
                        authority.item_id, error
                    ));
                }
            }
        }
        blocking_ids.sort();
        blocking_ids.dedup();
        let eligible = matches!(
            snapshot.collection.binding,
            CollectionBinding::LegacyUnassigned
        ) && chosen.map_or(!eligible_modules.is_empty(), |module| {
            eligible_modules.contains(module)
        });
        CollectionTypeInspection {
            collection: snapshot.collection,
            item_domain_counts: counts,
            eligible_modules,
            eligible,
            unresolved,
            blocking_ids,
        }
    }

    fn pending_compatible(
        &self,
        pending: &CollectionTypePendingReference,
        module: &ModuleId,
    ) -> bool {
        if pending.module.as_ref().is_some_and(|owner| owner != module) {
            return false;
        }
        if let Some(operation) = &pending.operation {
            return self
                .modules
                .capability_contract(operation)
                .is_ok_and(|contract| {
                    contract.scope == collector_modules::OperationScope::SharedMedia
                        || &contract.owner == module
                });
        }
        pending.module.as_ref() == Some(module)
    }

    fn pending_owner(&self, pending: &CollectionTypePendingReference) -> Option<ModuleId> {
        pending.module.clone().or_else(|| {
            pending.operation.as_ref().and_then(|operation| {
                CapabilityId::new(operation.to_string())
                    .ok()
                    .and_then(|capability| self.modules.capability_owner(&capability).ok())
                    .map(|contract| contract.0)
            })
        })
    }
}

// Shared by foreground commands and durable workers.
impl CollectionService {
    pub(super) fn content_write_eligibility(
        &self,
        actor: &Principal,
        collection_id: CollectionId,
    ) -> Result<ContentWriteEligibility> {
        self.content_write_eligibility_for_user(actor.user_id, collection_id)
    }
    pub(super) fn content_write_eligibility_for_user(
        &self,
        user_id: UserId,
        collection_id: CollectionId,
    ) -> Result<ContentWriteEligibility> {
        if !self
            .reader
            .role(user_id, collection_id)?
            .allows(Action::Curate)
        {
            return Err(Error::Forbidden);
        }
        let collection = self.reader.collection(collection_id)?;
        let owner = match &collection.binding {
            CollectionBinding::Assigned { module } => module.clone(),
            CollectionBinding::LegacyUnassigned => {
                return Err(Error::Unavailable(
                    "collection content is read-only until its legacy type is explicitly assigned"
                        .into(),
                ));
            }
        };
        let descriptor = self
            .modules
            .descriptors()
            .into_iter()
            .find(|descriptor| descriptor.id == owner)
            .ok_or_else(|| {
                Error::Unavailable(format!(
                    "collection owner module {} is not installed; content is read-only",
                    owner.0
                ))
            })?;
        Ok(ContentWriteEligibility {
            binding: collection.binding,
            owner,
            schema_version: descriptor.schema_version,
        })
    }
    pub(super) fn validate_owned_domain(
        &self,
        eligibility: &ContentWriteEligibility,
        domain: &ModuleData,
    ) -> Result<()> {
        if domain.module != eligibility.owner {
            return Err(Error::Invalid(format!(
                "item domain {} conflicts with collection owner {}",
                domain.module.0, eligibility.owner.0
            )));
        }
        if domain.schema_version != eligibility.schema_version {
            return Err(Error::Unavailable(format!(
                "item domain schema {} is unsupported; collection owner {} requires schema {}",
                domain.schema_version, eligibility.owner.0, eligibility.schema_version
            )));
        }
        self.modules.validate(domain)
    }
    pub(super) fn validate_operation_eligibility(
        &self,
        eligibility: &ContentWriteEligibility,
        operation: &OperationId,
    ) -> Result<u32> {
        let contract = self.modules.capability_contract(operation)?;
        if contract.scope == collector_modules::OperationScope::CollectionDomain
            && contract.owner != eligibility.owner
        {
            return Err(Error::Invalid(format!(
                "operation {} belongs to {}, not collection owner {}",
                operation, contract.owner.0, eligibility.owner.0
            )));
        }
        Ok(contract.schema_version)
    }
    pub(super) fn validate_frozen_binding(
        &self,
        eligibility: &ContentWriteEligibility,
        frozen: &CollectionBinding,
    ) -> Result<()> {
        if &eligibility.binding != frozen {
            return Err(Error::Conflict);
        }
        Ok(())
    }
    pub(super) fn content_write_eligibility_for_job(
        &self,
        user_id: UserId,
        collection_id: Option<CollectionId>,
        item_id: Option<ItemId>,
    ) -> Result<ContentWriteEligibility> {
        let collection_id = collection_id
            .ok_or_else(|| Error::Storage("content job has no Collection identity".into()))?;
        let eligibility = self.content_write_eligibility_for_user(user_id, collection_id)?;
        if let Some(item_id) = item_id {
            let item = self.reader.item(item_id)?;
            if item.collection_id != collection_id {
                return Err(Error::Conflict);
            }
            self.validate_owned_domain(&eligibility, &item.domain)?;
        }
        Ok(eligibility)
    }
}

impl CollectionService {
    /// Safe, registered collection-type contracts for authenticated clients.
    /// Configuration and management state are deliberately absent.
    pub fn collection_type_descriptors(&self, _actor: &Principal) -> Vec<ModuleDescriptor> {
        self.modules.descriptors()
    }

    pub(super) fn presented_content_permissions(
        &self,
        role: Role,
        collection: &Collection,
        item_domain: Option<&ModuleData>,
    ) -> (EffectivePermissions, ContentPermissions) {
        let (status, message, administrator_commands) = match &collection.binding {
            CollectionBinding::LegacyUnassigned => (
                ContentAvailabilityStatus::LegacyUnassigned,
                Some("This legacy collection is read-only until an administrator explicitly assigns its collection type with the stopped server.".into()),
                vec![
                    format!("cargo run -p collector-server -- inspect-collection-type --collection-id {}", collection.id),
                    format!("cargo run -p collector-server -- assign-collection-type --collection-id {} --expected-revision {} --module <MODULE_ID>", collection.id, collection.revision.0),
                ],
            ),
            CollectionBinding::Assigned { module } => {
                match self
                    .modules
                    .descriptors()
                    .into_iter()
                    .find(|descriptor| &descriptor.id == module)
                {
                    None => (
                        ContentAvailabilityStatus::OwnerUnavailable,
                        Some(format!(
                            "Collection type {} is not installed; preserved content remains readable but content changes are disabled.",
                            module.0
                        )),
                        vec![],
                    ),
                    Some(descriptor)
                        if item_domain.is_some_and(|domain| {
                            domain.module != descriptor.id
                                || domain.schema_version != descriptor.schema_version
                                || self.modules.inspect(domain).is_err()
                        }) => (
                            ContentAvailabilityStatus::ItemSchemaUnsupported,
                            Some("This item's preserved module data is not supported by the installed collection type; it remains read-only.".into()),
                            vec![],
                        ),
                    Some(_) => (ContentAvailabilityStatus::Available, None, vec![]),
                }
            }
        };
        let can_edit =
            role.allows(Action::Curate) && status == ContentAvailabilityStatus::Available;
        let mut effective_permissions: EffectivePermissions = role.into();
        effective_permissions.can_curate = can_edit;
        (
            effective_permissions,
            ContentPermissions {
                can_edit,
                status,
                message,
                administrator_commands,
            },
        )
    }
}
