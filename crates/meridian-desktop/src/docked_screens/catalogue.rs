//! Display facts only. These authored records never activate a hull, shop or journey.
use meridian_core::navigation::SystemId;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldRecord {
    pub id: SystemId,
    pub name: &'static str,
    pub position: [f64; 2],
    pub economy_index: u8,
    pub economy: &'static str,
    pub government_index: u8,
    pub government: &'static str,
    pub technology: u8,
    pub population_billions: f64,
    pub productivity_billion_cr: f64,
    pub radius_km: u32,
    pub ringed: bool,
    pub surface: &'static str,
    pub climate: &'static str,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectRecord<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub family: &'static str,
    pub kind: &'static str,
    pub role: &'static str,
    pub player_flyable: bool,
    pub purchase_policy: &'static str,
    pub source_status: &'static str,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Specification {
    pub object: &'static str,
    pub group: &'static str,
    pub key: &'static str,
    pub value: &'static str,
}
#[derive(Debug)]
pub struct Catalogue {
    pub worlds: Vec<WorldRecord>,
    pub objects: Vec<ObjectRecord<'static>>,
    pub specifications: Vec<Specification>,
    authored: Vec<AuthoredRecord>,
}

/// Desktop-validated package display only; no exchange or flight authority.
#[derive(Debug, Clone)]
pub(crate) struct AuthoredRecord {
    pub id: String,
    pub name: String,
    pub specifications: Vec<String>,
}

impl AuthoredRecord {
    fn object(&self) -> ObjectRecord<'_> {
        ObjectRecord {
            id: &self.id,
            name: &self.name,
            family: "Authored package",
            kind: "ship",
            role: "Inspection only",
            player_flyable: false,
            purchase_policy: "Not admitted",
            source_status: "Explicit local package; not an original fleet entry",
        }
    }
}
fn columns(line: &'static str, count: usize) -> Result<Vec<&'static str>, &'static str> {
    let row: Vec<_> = line.split('\t').collect();
    if row.len() != count || row.iter().any(|c| c.is_empty()) {
        return Err("Malformed original display catalogue");
    }
    Ok(row)
}
fn number<T: std::str::FromStr>(value: &str) -> Result<T, &'static str> {
    value.parse().map_err(|_| "Invalid original display value")
}
impl Catalogue {
    pub fn bundled() -> Result<Self, &'static str> {
        Self::decode(
            include_str!("data/worlds.tsv"),
            include_str!("data/objects.tsv"),
            include_str!("data/specifications.tsv"),
        )
    }
    pub(super) fn decode(
        worlds: &'static str,
        objects: &'static str,
        specs: &'static str,
    ) -> Result<Self, &'static str> {
        let mut result = Self {
            worlds: Vec::with_capacity(2048),
            objects: Vec::with_capacity(171),
            specifications: Vec::new(),
            authored: Vec::new(),
        };
        for (index, line) in worlds.lines().enumerate() {
            let c = columns(line, 16)?;
            let id = SystemId {
                galaxy: number(c[0])?,
                system: number(c[1])?,
            };
            if usize::from(id.galaxy) != index / 256 || usize::from(id.system) != index % 256 {
                return Err("Original world identities are not ordered and unique");
            }
            let position: [f64; 2] = [number(c[3])?, number(c[4])?];
            let population_billions: f64 = number(c[10])?;
            let productivity_billion_cr: f64 = number(c[11])?;
            if position.into_iter().any(|v| !v.is_finite())
                || [population_billions, productivity_billion_cr]
                    .into_iter()
                    .any(|v| !v.is_finite() || v < 0.0)
            {
                return Err("Non-finite original world facts");
            }
            let row = WorldRecord {
                id,
                name: c[2],
                position,
                economy_index: number(c[5])?,
                economy: c[6],
                government_index: number(c[7])?,
                government: c[8],
                technology: number(c[9])?,
                population_billions,
                productivity_billion_cr,
                radius_km: number(c[12])?,
                ringed: number(c[13])?,
                surface: c[14],
                climate: c[15],
            };
            if row.economy_index > 7 || row.government_index > 7 {
                return Err("Invalid original world category");
            }
            result.worlds.push(row);
        }
        if result.worlds.len() != 2048 {
            return Err("Incomplete original world catalogue");
        }
        for (index, line) in objects.lines().enumerate() {
            let c = columns(line, 8)?;
            if c[0] != format!("SH{:03}", index + 1) {
                return Err("Original object identities are not ordered and unique");
            }
            result.objects.push(ObjectRecord {
                id: c[0],
                name: c[1],
                family: c[2],
                kind: c[3],
                role: c[4],
                player_flyable: number(c[5])?,
                purchase_policy: c[6],
                source_status: c[7],
            });
        }
        if result.objects.len() != 171
            || result.objects.iter().filter(|o| o.kind == "ship").count() != 162
        {
            return Err("Incomplete original object taxonomy");
        }
        for line in specs.lines() {
            let c = columns(line, 4)?;
            if !result.objects.iter().any(|o| o.id == c[0]) {
                return Err("Specification refers to an unknown object");
            }
            result.specifications.push(Specification {
                object: c[0],
                group: c[1],
                key: c[2],
                value: c[3],
            });
        }
        if result
            .objects
            .iter()
            .any(|o| !result.specifications.iter().any(|s| s.object == o.id))
        {
            return Err("Missing original object specifications");
        }
        Ok(result)
    }
    pub fn world(&self, id: SystemId) -> Option<&WorldRecord> {
        if id.galaxy > 7 || id.system > 255 {
            return None;
        }
        self.worlds
            .get(usize::from(id.galaxy) * 256 + usize::from(id.system))
    }
    pub fn object(&self, id: &str) -> Option<ObjectRecord<'_>> {
        if let Some(original) = self.objects.iter().copied().find(|o| o.id == id) {
            return Some(original);
        }
        self.authored
            .iter()
            .find(|o| o.id == id)
            .map(AuthoredRecord::object)
    }
    pub(crate) fn add_authored(&mut self, record: AuthoredRecord) -> Result<(), String> {
        let id_key = record.id.trim().to_lowercase();
        let name_key = record.name.trim().to_lowercase();
        let collides = |id: &str, name: &str| {
            let id = id.trim().to_lowercase();
            let name = name.trim().to_lowercase();
            id == id_key || id == name_key || name == id_key || name == name_key
        };
        if self.objects.iter().any(|o| collides(o.id, o.name))
            || self.authored.iter().any(|o| collides(&o.id, &o.name))
        {
            return Err(format!(
                "Authored hull identity conflict: {} ({})",
                record.id, record.name
            ));
        }
        self.authored.push(record);
        Ok(())
    }
    pub(crate) fn authored_specifications(&self, id: &str) -> Option<&[String]> {
        self.authored
            .iter()
            .find(|o| o.id == id)
            .map(|o| o.specifications.as_slice())
    }
    pub(crate) fn object_count(&self) -> usize {
        self.objects.len() + self.authored.len()
    }
    pub(crate) fn ship_count(&self) -> usize {
        self.objects.iter().filter(|o| o.kind == "ship").count() + self.authored.len()
    }
    pub fn distance(&self, from: SystemId, to: SystemId) -> Option<f64> {
        if from.galaxy != to.galaxy {
            return None;
        }
        let a = self.world(from)?;
        let b = self.world(to)?;
        Some((a.position[0] - b.position[0]).hypot(a.position[1] - b.position[1]))
    }
    pub fn search_worlds(&self, galaxy: u8, query: &str) -> Vec<&WorldRecord> {
        let q = query.to_lowercase();
        let mut rows: Vec<_> = self
            .worlds
            .iter()
            .filter(|w| w.id.galaxy == galaxy && w.name.to_lowercase().contains(&q))
            .collect();
        rows.sort_by_key(|w| (!w.name.to_lowercase().starts_with(&q), w.name, w.id.system));
        rows
    }
    pub fn search_ships(&self, query: &str) -> Vec<ObjectRecord<'_>> {
        let q = query.to_lowercase();
        let mut rows: Vec<ObjectRecord<'_>> = self.objects.to_vec();
        rows.extend(self.authored.iter().map(AuthoredRecord::object));
        rows.retain(|o| {
            o.kind == "ship"
                && format!("{} {} {} {}", o.id, o.name, o.family, o.role)
                    .to_lowercase()
                    .contains(&q)
        });
        rows.sort_by_key(|o| {
            (
                o.id.to_lowercase() != q,
                o.name.to_lowercase() != q,
                o.name,
                o.id,
            )
        });
        rows
    }
}
