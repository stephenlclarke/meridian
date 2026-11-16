//! In-memory original pack loading. No filesystem, implicit latest-version or geometry fallback.
use crate::{Error, Value, encode, validate};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub definition: String,
    pub version: String,
    pub digest: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub namespace: String,
    pub mission: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub handlers: BTreeMap<String, String>,
    pub module: String,
}
#[derive(Debug)]
pub struct Pack {
    identity: Identity,
    manifest: Manifest,
    source: Vec<u8>,
}
impl Pack {
    /// Load only the original source-bound T01 content. Other bundled entries do not become
    /// executable merely because their raw files are present in the content library.
    pub fn bundled_flight_reception() -> Result<Self, Error> {
        let original = meridian_mission_content::BUNDLED_PACKS
            .iter()
            .find(|p| p.definition == "l337.training/flight-reception" && p.version == "1.0.0")
            .ok_or(Error::Pack("original T01 content missing"))?;
        meridian_mission_content::Registry::new(std::slice::from_ref(original))
            .map_err(|_| Error::Pack("original T01 content hash/identity refusal"))?;
        let manifest = original
            .manifest_bytes()
            .map_err(|_| Error::Pack("original T01 manifest missing"))?;
        let entry = original
            .entry_bytes()
            .map_err(|_| Error::Pack("original T01 entry missing"))?;
        let loaded = Self::load(
            manifest,
            &BTreeMap::from([(original.entry_path.to_owned(), entry.to_vec())]),
        )?;
        if loaded.identity.digest
            != "bc01834e04f87816c8021b76fabe3d9b8f3af394ff8fcd420c4b7c3acf1350d4"
        {
            return Err(Error::Identity);
        }
        Ok(loaded)
    }
    /// Caller supplies frozen resource bytes. Missing/extra resources and unsupported model/module
    /// compilation refuse instead of producing a digest for a different bundle.
    pub fn load(
        manifest_bytes: &[u8],
        resources: &BTreeMap<String, Vec<u8>>,
    ) -> Result<Self, Error> {
        if manifest_bytes.len() > 262_144 {
            return Err(Error::Limit("manifest bytes"));
        }
        let text =
            std::str::from_utf8(manifest_bytes).map_err(|_| Error::Pack("manifest UTF-8"))?;
        let root: toml::Table =
            toml::from_str(text).map_err(|_| Error::Pack("invalid TOML manifest"))?;
        validate_core(&root)?;
        if root.get("schemaVersion").and_then(toml::Value::as_integer) != Some(2)
            || root.get("missionAPI").and_then(toml::Value::as_integer) != Some(2)
        {
            return Err(Error::Pack("unsupported manifest schema/API"));
        }
        // Canonical geometry and closed module compilation must be implemented before those
        // bundles can be loaded. Raw resource hashes are never substituted for their digests.
        for key in [
            "models",
            "npcs",
            "encounters",
            "worldJobs",
            "board",
            "freight",
            "trial",
            "dialogs",
        ] {
            if root.contains_key(key) {
                return Err(Error::Pack("unsupported pack resource/native family"));
            }
        }
        let namespace = field(&root, "id")?.to_owned();
        let version = field(&root, "version")?.to_owned();
        let mission = root
            .get("mission")
            .and_then(toml::Value::as_table)
            .ok_or(Error::Pack("missing mission"))?;
        let mission = field(mission, "id")?.to_owned();
        if !local_id(&namespace) || !local_id(&mission) || !valid_version(&version) {
            return Err(Error::Pack("invalid pack identity"));
        }
        let module = field(&root, "module")?.to_owned();
        if !resource_path(&module) || !module.ends_with(".lua") || module == "manifest.toml" {
            return Err(Error::Pack("invalid module path"));
        }
        let capabilities = root
            .get("capabilities")
            .and_then(toml::Value::as_array)
            .ok_or(Error::Pack("missing capabilities"))?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or(Error::Pack("invalid capability"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !capabilities.iter().any(|c| c == "script-lua-v1") {
            return Err(Error::Pack("missing Lua capability"));
        }
        let rows = root
            .get("handlers")
            .and_then(toml::Value::as_array)
            .ok_or(Error::Pack("missing handlers"))?;
        if !(1..=64).contains(&rows.len()) {
            return Err(Error::Pack("invalid handler count"));
        }
        let mut handlers = BTreeMap::new();
        for row in rows {
            let row = row.as_table().ok_or(Error::Pack("invalid handler"))?;
            if row.len() != 2 {
                return Err(Error::Pack("unknown handler field"));
            }
            let event = field(row, "event")?;
            let function = field(row, "function")?;
            if !supported_event(event)
                || !crate::evaluator::handler_name(function)
                || handlers
                    .insert(event.to_owned(), function.to_owned())
                    .is_some()
            {
                return Err(Error::Pack("invalid/duplicate handler"));
            }
        }
        let declared: Vec<String> = handlers.values().cloned().collect();
        let entry = resources
            .get(&module)
            .ok_or(Error::Pack("missing module"))?;
        if entry.len() > 262_144 || entry.contains(&0) || std::str::from_utf8(entry).is_err() {
            return Err(Error::Pack(
                "Mission Lua must be UTF-8 text without NUL bytes.",
            ));
        }
        let (source, extras) = compile_modules(&root, &module, entry, resources)?;
        validate(&source, &declared)?;
        let digest = pack_digest(manifest_bytes, &module, entry, &extras)?;
        Ok(Self {
            identity: Identity {
                definition: format!("{namespace}/{mission}"),
                version: version.clone(),
                digest,
            },
            manifest: Manifest {
                namespace,
                mission,
                version,
                capabilities,
                handlers,
                module,
            },
            source,
        })
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn source(&self) -> &[u8] {
        &self.source
    }
    pub fn execution_issue(&self) -> Option<String> {
        let missing = self
            .manifest
            .capabilities
            .iter()
            .filter(|capability| {
                ![
                    "script-lua-v1",
                    "story-v2",
                    "durable-commands-v2",
                    "contracts-basic-v2",
                    "native-flight-reception-v1",
                    "pack-modules-v2",
                ]
                .contains(&capability.as_str())
            })
            .cloned()
            .collect::<Vec<_>>();
        (!missing.is_empty())
            .then(|| format!("Not available in this build: {}", missing.join(", ")))
    }
}
fn field<'a>(map: &'a toml::Table, key: &str) -> Result<&'a str, Error> {
    map.get(key)
        .and_then(toml::Value::as_str)
        .ok_or(Error::Pack("missing/string manifest field"))
}
fn local_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'.'))
}
fn valid_version(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.bytes().all(|b| b.is_ascii_digit())
                && (p.len() == 1 || !p.starts_with('0'))
        })
}
fn supported_event(event: &str) -> bool {
    [
        "mission.accepted",
        "station.docked",
        "station.undocked",
        "system.entered",
        "contact.requested",
        "dialogue.confirmed",
        "timer.fired",
        "mission.abandoned",
        "encounter.admitted",
        "encounter.pending",
        "actor.destroyed",
        "actor.removed",
        "ship.changed",
        "equipment.changed",
        "gear.consumed",
        "equipment.issued",
        "equipment.returned",
        "equipment.released",
        "actor.intent.accepted",
        "actor.arrived",
        "actor.target.lost",
        "actor.damaged",
        "actor.transit.requested",
        "actor.transit.opened",
        "actor.transit.departed",
        "actor.transit.arrived",
        "component.destroyed",
        "weapon.fired",
        "mission.receipted",
        "dialogue.chosen",
        "book.unlocked",
        "collection.delivered",
        "campaign.child.started",
        "courier.terminated",
        "provider.transition",
        "provider.closed",
        "procurement.terminated",
        "freight.terminated",
    ]
    .contains(&event)
}
fn validate_core(root: &toml::Table) -> Result<(), Error> {
    let allowed = [
        "schemaVersion",
        "missionAPI",
        "id",
        "version",
        "minimumEngine",
        "capabilities",
        "dependencies",
        "priority",
        "source",
        "license",
        "environments",
        "module",
        "mission",
        "handlers",
        "flightReception",
        "models",
        "modules",
        "npcs",
        "encounters",
        "worldJobs",
        "board",
        "freight",
        "trial",
        "dialogs",
    ];
    if root.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err(Error::Pack("unsupported/unknown manifest field"));
    }
    if field(root, "minimumEngine")? != "2.0.0"
        || !root
            .get("priority")
            .and_then(toml::Value::as_integer)
            .is_some_and(|v| (0..=1000).contains(&v))
        || !(1..=4096).contains(&field(root, "source")?.len())
        || !(1..=4096).contains(&field(root, "license")?.len())
    {
        return Err(Error::Pack("invalid engine/scope declarations"));
    }
    let environment = root
        .get("environments")
        .and_then(toml::Value::as_array)
        .ok_or(Error::Pack("missing environments"))?;
    if !(1..=2).contains(&environment.len())
        || environment
            .iter()
            .any(|v| !matches!(v.as_str(), Some("space" | "orbital-station")))
    {
        return Err(Error::Pack("invalid environments"));
    }
    let capabilities = root
        .get("capabilities")
        .and_then(toml::Value::as_array)
        .ok_or(Error::Pack("missing capabilities"))?;
    if capabilities.len() > 64
        || capabilities
            .iter()
            .enumerate()
            .any(|(i, v)| !v.as_str().is_some_and(local_id) || capabilities[..i].contains(v))
    {
        return Err(Error::Pack("invalid/duplicate capabilities"));
    }
    if let Some(dependencies) = root.get("dependencies") {
        if !dependencies.as_array().is_some_and(Vec::is_empty) {
            return Err(Error::Pack(
                "pack dependencies are not supported by this adapter",
            ));
        }
    }
    let mission = root
        .get("mission")
        .and_then(toml::Value::as_table)
        .ok_or(Error::Pack("missing mission"))?;
    if mission.keys().any(|k| {
        ![
            "id",
            "title",
            "briefing",
            "destination",
            "reward",
            "milestones",
        ]
        .contains(&k.as_str())
    }) || !(1..=64).contains(&field(mission, "title")?.chars().count())
        || !(1..=12000).contains(&field(mission, "briefing")?.chars().count())
        || field(mission, "destination")?.len() > 2048
        || field(mission, "reward")?.len() > 2048
    {
        return Err(Error::Pack("invalid/unsupported mission declarations"));
    }
    let milestones = mission
        .get("milestones")
        .and_then(toml::Value::as_array)
        .ok_or(Error::Pack("missing milestones"))?;
    if !(1..=64).contains(&milestones.len())
        || milestones.iter().any(|v| {
            !v.as_str()
                .is_some_and(|s| (1..=256).contains(&s.chars().count()))
        })
    {
        return Err(Error::Pack("invalid milestones"));
    }
    if let Some(profile) = root.get("flightReception") {
        let profile = profile
            .as_table()
            .ok_or(Error::Pack("invalid T01 profile"))?;
        if profile.len() != 1 || field(profile, "profile")? != "native-main-berth-flight-v1" {
            return Err(Error::Pack("unsupported T01 profile"));
        }
    }
    Ok(())
}
fn resource_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 240
        && !path.contains(['\\', ':', '\0'])
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'/'))
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
fn pack_digest(
    manifest: &[u8],
    module: &str,
    source: &[u8],
    extras: &BTreeMap<String, Vec<u8>>,
) -> Result<String, Error> {
    let mut digest = Sha256::new();
    for (name, bytes) in [("manifest.toml", manifest), (module, source)]
        .into_iter()
        .chain(
            extras
                .iter()
                .map(|(name, bytes)| (name.as_str(), bytes.as_slice())),
        )
    {
        let length = i64::try_from(bytes.len()).map_err(|_| Error::Limit("resource size"))?;
        digest.update(encode(
            &[Value::String(name.to_owned()), Value::Integer(length)],
            1_048_576,
        )?);
        digest.update(bytes);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn compile_modules(
    root: &toml::Table,
    entry_path: &str,
    entry: &[u8],
    resources: &BTreeMap<String, Vec<u8>>,
) -> Result<(Vec<u8>, BTreeMap<String, Vec<u8>>), Error> {
    let mut modules = BTreeMap::new();
    let mut extras = BTreeMap::new();
    if let Some(definitions) = root.get("modules") {
        let definitions = definitions
            .as_array()
            .ok_or(Error::Pack("Invalid mission module definitions."))?;
        if definitions.len() > 16 {
            return Err(Error::Pack("Invalid mission module definitions."));
        }
        for definition in definitions {
            let definition = definition
                .as_table()
                .ok_or(Error::Pack("Invalid mission module definitions."))?;
            let id = field(definition, "id")?;
            let path = field(definition, "path")?;
            if definition.len() != 2
                || !module_id(id)
                || !resource_path(path)
                || !path.ends_with(".lua")
                || path == entry_path
                || modules.contains_key(id)
                || extras.contains_key(path)
            {
                return Err(Error::Pack("Invalid mission module definitions."));
            }
            let bytes = resources
                .get(path)
                .ok_or(Error::Pack("missing declared module"))?;
            if bytes.len() > 262_144 || bytes.contains(&0) || std::str::from_utf8(bytes).is_err() {
                return Err(Error::Pack(
                    "Mission modules must be UTF-8 text without NUL bytes.",
                ));
            }
            modules.insert(id.to_owned(), path.to_owned());
            extras.insert(path.to_owned(), bytes.clone());
        }
    }
    if resources.len() != extras.len() + 1 {
        return Err(Error::Pack("missing/extra pack resources"));
    }
    if !modules.is_empty()
        && !root
            .get("capabilities")
            .and_then(toml::Value::as_array)
            .is_some_and(|c| c.iter().any(|v| v.as_str() == Some("pack-modules-v2")))
    {
        return Err(Error::Pack(
            "Invalid, duplicate or unsupported pack module declarations.",
        ));
    }
    if modules.is_empty() {
        return Ok((entry.to_vec(), extras));
    }
    let mut source = include_bytes!("module_prelude.lua").to_vec();
    for (id, path) in modules {
        source.extend(format!("\n-- {path}\n__mission_factories['{id}']=function()\n").as_bytes());
        source.extend(
            extras
                .get(&path)
                .ok_or(Error::Pack("missing declared module"))?,
        );
        source.extend(b"\nend\n");
    }
    source.extend(b"\nreturn (function()\n");
    source.extend(entry);
    source.extend(b"\nend)()\n");
    if source.len() > 262_144 {
        return Err(Error::Pack(
            "The complete compiled mission module set exceeds256KiB.",
        ));
    }
    Ok((source, extras))
}
fn module_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value.as_bytes()[0].is_ascii_alphabetic()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'))
}
