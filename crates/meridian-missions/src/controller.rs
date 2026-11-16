//! Persisted explicit state and bounded ordered event proposals, never a second simulation.
use crate::{Error, Identity, Limits, Pack, Value, evaluate};
use std::collections::VecDeque;

mod persistence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Active,
    Suspended(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub sequence: u64,
    pub kind: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    JournalAppend(String),
    /// Descriptive request only. There is no generic native dialogue/custody adapter yet.
    DialogueOffer(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    pub(crate) identity: Identity,
    pub(crate) native_binding: Option<(u64, u8)>,
    pub(crate) data: Value,
    pub(crate) revision: u64,
    pub(crate) next_sequence: u64,
    pub(crate) acknowledged_through: u64,
    pub(crate) pending: VecDeque<Event>,
    pub(crate) status: Status,
    pub(crate) journal: Vec<String>,
}
impl Instance {
    /// Creates explicit script storage only; this does not accept a native mission or issue goods.
    pub fn new(pack: &Pack) -> Self {
        Self {
            identity: pack.identity().clone(),
            native_binding: None,
            data: Value::empty_map(),
            revision: 0,
            next_sequence: 0,
            acknowledged_through: 0,
            pending: VecDeque::new(),
            status: Status::Active,
            journal: Vec::new(),
        }
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    pub fn data(&self) -> &Value {
        &self.data
    }
    pub fn journal(&self) -> &[String] {
        &self.journal
    }
    pub fn status(&self) -> &Status {
        &self.status
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn acknowledged_through(&self) -> u64 {
        self.acknowledged_through
    }
    pub fn pending(&self) -> &VecDeque<Event> {
        &self.pending
    }

    /// A proposed queue insertion. The owner publishes it with actual World emission state.
    pub fn with_event(&self, event: Event) -> Result<Self, Error> {
        if self.pending.len() >= 4096 {
            return Err(Error::Limit("pending events"));
        }
        if event.sequence <= self.next_sequence
            || event.sequence >= i64::MAX as u64
            || event.kind.is_empty()
            || event.kind.len() > 160
        {
            return Err(Error::Replay);
        }
        crate::encode(std::slice::from_ref(&event.value), 262_144)?;
        let mut candidate = self.clone();
        candidate.next_sequence = event.sequence;
        candidate.pending.push_back(event);
        Ok(candidate)
    }
    /// Freeze a prefix before any command derives another event; no recursive queue drain.
    pub fn batch(&self) -> Vec<Event> {
        self.pending.iter().take(64).cloned().collect()
    }

    pub fn suspended(&self, reason: &str) -> Result<Self, Error> {
        if reason.is_empty() || reason.len() > 12_000 {
            return Err(Error::Limit("suspension reason"));
        }
        let mut candidate = self.clone();
        candidate.status = Status::Suspended(reason.to_owned());
        Ok(candidate)
    }
    pub fn resumed(&self, pack: &Pack) -> Result<Self, Error> {
        if self.identity != *pack.identity() {
            return Err(Error::Identity);
        }
        if let Some(issue) = pack.execution_issue() {
            return Err(Error::UnsupportedOperation(issue));
        }
        let mut candidate = self.clone();
        candidate.status = Status::Active;
        Ok(candidate)
    }
    pub fn reconcile(&self, pack: Option<&Pack>) -> Result<Self, Error> {
        if pack.is_some_and(|pack| self.identity == *pack.identity()) {
            if let Some(issue) = pack.and_then(Pack::execution_issue) {
                return self.suspended(&issue);
            }
            return Ok(self.clone());
        }
        self.suspended("Restore the exact saved mission version and its requirements.")
    }

    /// Native faults keep the rejected data/actions/acknowledgement out of the suspension
    /// proposal. Publication remains explicit; a failed writer is never retried here.
    pub fn prepare_dispatch(
        &self,
        pack: Option<&Pack>,
        sequence: u64,
        snapshot: &Value,
    ) -> Result<PendingTransition, Error> {
        if !matches!(self.status, Status::Active) {
            return Err(Error::Suspended);
        }
        if self.pending.front().map(|e| e.sequence) != Some(sequence) {
            return Err(Error::Stale);
        }
        let Some(pack) = pack.filter(|p| p.identity() == &self.identity) else {
            return Ok(PendingTransition {
                before: self.clone(),
                candidate: self.reconcile(None)?,
                actions: Vec::new(),
            });
        };
        match self.prepare(pack, sequence, snapshot) {
            Ok(proposal) => Ok(proposal),
            Err(
                error @ (Error::Execution { .. }
                | Error::Codec(_)
                | Error::Pack(_)
                | Error::Limit(_)
                | Error::UnsupportedOperation(_)),
            ) => {
                let reason = error.to_string().chars().take(1024).collect::<String>();
                Ok(PendingTransition {
                    before: self.clone(),
                    candidate: self.suspended(&reason)?,
                    actions: Vec::new(),
                })
            }
            Err(error) => Err(error),
        }
    }

    /// Builds state/actions against an immutable query snapshot. Actions still need a real adapter.
    pub fn prepare(
        &self,
        pack: &Pack,
        sequence: u64,
        snapshot: &Value,
    ) -> Result<PendingTransition, Error> {
        if !matches!(self.status, Status::Active) {
            return Err(Error::Suspended);
        }
        if self.identity != *pack.identity() {
            return Err(Error::Identity);
        }
        if let Some(issue) = pack.execution_issue() {
            return Err(Error::UnsupportedOperation(issue));
        }
        let event = self
            .pending
            .iter()
            .find(|e| e.sequence == sequence)
            .ok_or(Error::Replay)?;
        if self.pending.front().map(|e| e.sequence) != Some(sequence) {
            return Err(Error::Stale);
        }
        let mut candidate = self.clone();
        candidate.pending.retain(|e| e.sequence != sequence);
        if sequence <= self.acknowledged_through {
            return Ok(PendingTransition {
                before: self.clone(),
                candidate,
                actions: Vec::new(),
            });
        }
        let Some(handler) = pack.manifest().handlers.get(&event.kind) else {
            return Ok(PendingTransition {
                before: self.clone(),
                candidate,
                actions: Vec::new(),
            });
        };
        let result = evaluate(
            pack.source(),
            handler,
            &event.value,
            snapshot,
            &self.data,
            Limits::default(),
        )?;
        let mut actions = Vec::new();
        for command in result.commands {
            actions.push(parse_action(command, pack)?);
        }
        candidate.data = result.data;
        candidate.revision = candidate
            .revision
            .checked_add(1)
            .filter(|v| *v < i64::MAX as u64)
            .ok_or(Error::Limit("script revision"))?;
        candidate.acknowledged_through = candidate.acknowledged_through.max(sequence);
        // Journal effects are staged in the same instance. Dialogue requests remain unexecuted.
        for action in &actions {
            if let Action::JournalAppend(text) = action {
                if candidate.journal.len() >= 512 {
                    return Err(Error::Limit("Journal budget exceeded."));
                }
                candidate.journal.push(text.clone());
            }
        }
        candidate.to_bytes()?;
        Ok(PendingTransition {
            before: self.clone(),
            candidate,
            actions,
        })
    }
}

pub struct PendingTransition {
    before: Instance,
    pub(crate) candidate: Instance,
    actions: Vec<Action>,
}
impl PendingTransition {
    pub fn actions(&self) -> &[Action] {
        &self.actions
    }
    pub fn candidate(&self) -> &Instance {
        &self.candidate
    }
    /// Storage-only commit. A dialogue proposal requires the real native adapter, never this method.
    pub fn commit_storage(self, current: &mut Instance) -> Result<(), Error> {
        if *current != self.before {
            return Err(Error::Stale);
        }
        if self
            .actions
            .iter()
            .any(|a| matches!(a, Action::DialogueOffer(_)))
        {
            return Err(Error::UnsupportedOperation(
                "dialogue.offer native adapter".into(),
            ));
        }
        *current = self.candidate;
        Ok(())
    }
    pub fn publish_storage(
        self,
        current: &mut Instance,
        publish: impl FnOnce(&[u8]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        if *current != self.before {
            return Err(Error::Stale);
        }
        if self
            .actions
            .iter()
            .any(|a| matches!(a, Action::DialogueOffer(_)))
        {
            return Err(Error::UnsupportedOperation(
                "dialogue.offer native adapter".into(),
            ));
        }
        let bytes = self.candidate.to_bytes()?;
        publish(&bytes)?;
        *current = self.candidate;
        Ok(())
    }
}
fn parse_action(command: Value, pack: &Pack) -> Result<Action, Error> {
    let fields = command
        .as_map()
        .ok_or(Error::Pack("Invalid script command."))?;
    let op = fields
        .get("op")
        .and_then(Value::string)
        .ok_or(Error::Pack("Invalid script command."))?;
    let (field, capability) = match op {
        "journal.append" => ("text", "story-v2"),
        "dialogue.offer" => ("id", "story-v2"),
        _ => return Err(Error::UnsupportedOperation(op.to_owned())),
    };
    if fields.len() != 2 || !pack.manifest().capabilities.iter().any(|c| c == capability) {
        return Err(Error::Pack("Command fields/capability differ."));
    }
    let text = fields
        .get(field)
        .and_then(Value::string)
        .ok_or(Error::Pack("Command text must be a string."))?;
    if text.is_empty() || text.len() > if op == "journal.append" { 12_000 } else { 160 } {
        return Err(Error::Limit("command text"));
    }
    Ok(if op == "journal.append" {
        Action::JournalAppend(text.to_owned())
    } else {
        Action::DialogueOffer(text.to_owned())
    })
}
