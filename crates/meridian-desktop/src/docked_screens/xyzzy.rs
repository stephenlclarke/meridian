//! UI review intents and read-only projections, never secret grants or work capabilities.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XyzzyAction {
    Enter,
    MaximumSupportedHardware,
    RestoreStarter,
    ToggleUnlimitedFuel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XyzzyOffer {
    pub action: XyzzyAction,
    /// None only when the real current core review admitted this exact action.
    pub refusal: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct XyzzyPanel {
    pub active: bool,
    pub commander_name: String,
    pub label: String,
    pub cash_tenths: u32,
    pub cash_granted_once: bool,
    pub unlimited_fuel: bool,
    pub jump_limit_ly: f64,
    pub full_legacy_outfit_available: bool,
    pub offers: Vec<XyzzyOffer>,
}

impl XyzzyPanel {
    pub(super) fn refusal(&self, action: XyzzyAction) -> Option<String> {
        self.offers
            .iter()
            .find(|offer| offer.action == action)
            .map_or_else(
                || Some("This XYZZY action has no current core offer".into()),
                |offer| offer.refusal.clone(),
            )
    }
}

pub(super) struct InventoryEntry {
    pub key: &'static str,
    pub behavior: &'static str,
    pub reason: &'static str,
    pub state: &'static str,
}
impl InventoryEntry {
    pub fn unavailable(&self) -> bool {
        self.state == "DEPENDENCY"
            || self.state == "NOT_APPLICABLE_CURRENT_SH025"
            || matches!(self.key, "0" | "7")
    }
}

pub(super) fn inventory_entries() -> impl Iterator<Item = InventoryEntry> {
    include_str!("data/xyzzy-actions.tsv")
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let key = fields.next()?;
            let behavior = fields.next()?;
            let state = fields.next()?;
            let reason = fields.next()?;
            Some(InventoryEntry {
                key,
                behavior,
                reason,
                state,
            })
        })
}
pub(super) fn dependencies() -> impl Iterator<Item = InventoryEntry> {
    inventory_entries().filter(InventoryEntry::unavailable)
}

/// Recognize the original suffix without retaining typed user text.
pub(super) fn advance_prefix(prefix: &mut usize, text: &str) -> (bool, bool) {
    let mut consumed = *prefix != 0;
    let sequence = b"xyzzy";
    for ch in text.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii() && ch as u8 == sequence[*prefix] {
            *prefix += 1;
            consumed = true;
            if *prefix == sequence.len() {
                *prefix = 0;
                return (true, true);
            }
        } else {
            *prefix = usize::from(ch == 'x');
            consumed |= ch == 'x';
        }
    }
    (consumed, false)
}
