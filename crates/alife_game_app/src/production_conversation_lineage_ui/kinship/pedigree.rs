//! Scalar identity snapshot. Never clone an organismâ€™s genome or touch its biology.
use super::{Control, HeadlessWorld};
use std::collections::BTreeMap;

pub(super) const PAGE: usize = 3;

#[derive(Debug, Clone)]
pub(super) struct Member {
    pub(super) id: u64,
    pub(super) genome: u64,
    pub(super) parents: Vec<u64>,
    pub(super) founder: bool,
    pub(super) born: u64,
    pub(super) alive: bool,
    pub(super) name: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Relative {
    Individual(u64),
    Unresolved(u64),
}

pub(super) struct Album {
    pub(super) members: BTreeMap<u64, Member>,
    carriers: BTreeMap<u64, Vec<u64>>,
    pub(super) focus: u64,
    pub(super) history: Vec<u64>,
    pub(super) page: usize,
}
impl Album {
    pub(super) fn from_world(world: &HeadlessWorld, selected: Option<u64>) -> Option<Self> {
        let members = world.organism_registry().iter().map(|r| {
            let genome = r.genome();
            let id = r.world_entity_id();
            Member {
                id: id.raw(),
                genome: genome.id.raw(),
                parents: genome.parent_genome_ids.iter().map(|g| g.raw()).collect(),
                founder: genome.parent_genome_ids.is_empty() && !genome.provenance.ordinary_birth,
                born: r.birth_tick().raw(),
                alive: r.lifecycle().is_alive(),
                name: world
                    .entity(id)
                    .map_or_else(|| format!("Individual {}", id.raw()), |o| o.label.clone()),
            }
        });
        Self::new(members, selected)
    }
    pub(super) fn new(
        members: impl IntoIterator<Item = Member>,
        selected: Option<u64>,
    ) -> Option<Self> {
        let members: BTreeMap<_, _> = members.into_iter().map(|m| (m.id, m)).collect();
        let focus = match selected {
            Some(id) if members.contains_key(&id) => id,
            Some(_) => return None,
            None => *members.keys().next()?,
        };
        let mut carriers: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
        for m in members.values() {
            carriers.entry(m.genome).or_default().push(m.id);
        }
        Some(Self {
            members,
            carriers,
            focus,
            history: Vec::new(),
            page: 0,
        })
    }
    pub(super) fn parent(&self, child: &Member, genome: u64) -> Relative {
        let mut candidates = self
            .carriers
            .get(&genome)
            .into_iter()
            .flatten()
            .copied()
            .filter(|&id| id != child.id && self.members[&id].born < child.born);
        match (candidates.next(), candidates.next()) {
            (Some(id), None) => Relative::Individual(id),
            _ => Relative::Unresolved(genome),
        }
    }
    pub(super) fn children(&self) -> Vec<u64> {
        let me = &self.members[&self.focus];
        let mut children: Vec<_> = self
            .members
            .values()
            .filter(|child| {
                child.parents.contains(&me.genome)
                    && self.parent(child, me.genome) == Relative::Individual(me.id)
            })
            .map(|m| (m.born, m.id))
            .collect();
        children.sort_unstable();
        children.into_iter().map(|(_, id)| id).collect()
    }
    pub(super) fn visit(&mut self, id: u64) {
        if id == self.focus || !self.members.contains_key(&id) {
            return;
        }
        if self.history.last() == Some(&id) {
            self.history.pop();
        } else {
            if self.history.len() == 32 {
                self.history.remove(0);
            }
            self.history.push(self.focus);
        }
        self.focus = id;
        self.page = 0;
    }
    pub(super) fn activate(&mut self, action: Control) -> bool {
        let before = (self.focus, self.page);
        match action {
            Control::Visit(id) => self.visit(id),
            Control::Back => {
                if let Some(id) = self.history.pop() {
                    self.focus = id;
                    self.page = 0;
                }
            }
            Control::Earlier => self.page = self.page.saturating_sub(1),
            Control::Later => {
                self.page = (self.page + 1).min(self.children().len().saturating_sub(1) / PAGE)
            }
            _ => {}
        }
        before != (self.focus, self.page)
    }
}
