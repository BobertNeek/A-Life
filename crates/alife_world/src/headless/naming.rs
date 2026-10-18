//! Player-assigned names are world metadata attached to a stable agent identity.

use super::*;

/// Maximum number of Unicode scalar values in a player-assigned agent name.
pub const MAX_AGENT_NAME_CHARS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RenameAgentError {
    #[error("choose a living agent to rename")]
    InvalidTarget,
    #[error("name cannot be empty")]
    EmptyName,
    #[error("name cannot contain control characters")]
    ControlCharacter,
    #[error("name must contain at most {max_chars} characters")]
    TooLong { max_chars: usize },
    #[error("name is already used by another world object")]
    NameInUse,
    #[error("name is reserved for world-generated objects")]
    ReservedName,
}

impl HeadlessWorld {
    /// Rename a living agent without changing its identity or acquired state.
    ///
    /// Names cannot contain control characters, are trimmed and limited by
    /// Unicode scalar count, and must be unique across all other world objects
    /// under Unicode lowercase comparison. Generated object-name prefixes are
    /// reserved, except when keeping the agent's exact existing label.
    /// Unregistered scenario agents follow the same availability rules as other
    /// world interactions; registered organisms must still be alive.
    pub fn rename_agent(&mut self, id: WorldEntityId, name: &str) -> Result<(), RenameAgentError> {
        let object = self.entity(id).ok_or(RenameAgentError::InvalidTarget)?;
        if !id.is_valid()
            || object.kind != WorldObjectKind::Agent
            || object.consumed
            || object.organism_id.is_none_or(|organism| {
                !organism.is_valid()
                    || self
                        .organism_registry
                        .get(organism)
                        .is_some_and(|record| !record.lifecycle().is_alive())
            })
        {
            return Err(RenameAgentError::InvalidTarget);
        }

        if name.chars().any(char::is_control) {
            return Err(RenameAgentError::ControlCharacter);
        }
        let name = name.trim();
        if name.is_empty() {
            return Err(RenameAgentError::EmptyName);
        }
        if name.chars().count() > MAX_AGENT_NAME_CHARS {
            return Err(RenameAgentError::TooLong {
                max_chars: MAX_AGENT_NAME_CHARS,
            });
        }
        let folded = name.to_lowercase();
        if name != object.label.as_str()
            && (folded.starts_with("voice-token-")
                || folded.starts_with("organism-")
                || self.ecology.spawn_policies.iter().any(|policy| {
                    folded.starts_with(&format!("{}-", policy.label_prefix).to_lowercase())
                }))
        {
            return Err(RenameAgentError::ReservedName);
        }
        if self
            .objects
            .values()
            .any(|other| other.id != id && other.label.to_lowercase() == folded)
        {
            return Err(RenameAgentError::NameInUse);
        }

        // All fallible validation precedes either canonical or derived update.
        let object = self.objects.get_mut(&id.raw()).unwrap();
        let old_label = std::mem::replace(&mut object.label, name.to_owned());
        self.labels.remove(&old_label);
        self.labels.insert(name.to_owned(), id);
        Ok(())
    }
}
