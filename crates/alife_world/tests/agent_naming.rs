use alife_core::{OrganismId, Vec3f, WorldEntityId};
use alife_world::{
    HeadlessScenarioBuilder, HeadlessWorld, RenameAgentError, TerrainZoneKind, MAX_AGENT_NAME_CHARS,
};

fn world() -> HeadlessWorld {
    HeadlessScenarioBuilder::new(73_205)
        .agent("agent", OrganismId(7), Vec3f::ZERO)
        .agent("Élodie", OrganismId(8), Vec3f::new(4.0, 0.0, 0.0))
        .food("İRIS", Vec3f::new(1.0, 0.0, 0.0), 0.5)
        .build()
        .unwrap()
}

#[test]
fn rename_trims_and_updates_the_canonical_label_and_lookup_with_the_same_identity() {
    let mut world = world();
    let id = world.entity_id("agent").unwrap();
    let mut expected = world.entity(id).unwrap().clone();
    expected.label = "Pip the Second".into();
    world.rename_agent(id, "\u{2003} Pip the Second  ").unwrap();
    assert_eq!(world.entity(id), Some(&expected));
    assert_eq!(world.entity_id("Pip the Second"), Some(id));
    assert_eq!(world.entity_id("agent"), None);

    // Renaming oneself, including a case-only change, remains valid.
    world.rename_agent(id, "Pip the Second").unwrap();
    world.rename_agent(id, "PIP THE SECOND").unwrap();
    assert_eq!(world.entity_id("Pip the Second"), None);
    assert_eq!(world.entity_id("PIP THE SECOND"), Some(id));

    // The retired label is available to another individual.
    let other = world.entity_id("Élodie").unwrap();
    world.rename_agent(other, "agent").unwrap();
    assert_eq!(world.entity_id("agent"), Some(other));
}

#[test]
fn invalid_names_and_collisions_leave_both_world_state_and_lookup_unchanged() {
    let mut world = world();
    let id = world.entity_id("agent").unwrap();
    let before = world.canonical_signature_digest().unwrap();
    let objects = world.object_snapshots();
    for (name, error) in [
        (String::new(), RenameAgentError::EmptyName),
        (" \u{2003} ".into(), RenameAgentError::EmptyName),
        ("Pi\0p".into(), RenameAgentError::ControlCharacter),
        ("Pip\nPup".into(), RenameAgentError::ControlCharacter),
        ("\nPip\n".into(), RenameAgentError::ControlCharacter),
        ("\tPip".into(), RenameAgentError::ControlCharacter),
        ("Pip\u{7f}".into(), RenameAgentError::ControlCharacter),
        (
            "🦎".repeat(MAX_AGENT_NAME_CHARS + 1),
            RenameAgentError::TooLong {
                max_chars: MAX_AGENT_NAME_CHARS,
            },
        ),
        ("ÉLODIE".into(), RenameAgentError::NameInUse),
        // Unicode lowercase expands capital dotted I into two scalar values.
        ("i\u{307}ris".into(), RenameAgentError::NameInUse),
    ] {
        assert_eq!(world.rename_agent(id, &name), Err(error), "{name:?}");
        assert_eq!(world.canonical_signature_digest().unwrap(), before);
        assert_eq!(world.object_snapshots(), objects);
        for object in &objects {
            assert_eq!(world.entity_id(&object.label), Some(object.id));
        }
        if !objects.iter().any(|object| object.label == name.trim()) {
            assert_eq!(world.entity_id(name.trim()), None);
        }
    }
}

#[test]
fn names_are_bounded_by_unicode_scalars_and_use_unicode_lowercase_comparison() {
    let mut world = world();
    let id = world.entity_id("agent").unwrap();
    let at_limit = "🦎".repeat(MAX_AGENT_NAME_CHARS);
    world.rename_agent(id, &at_limit).unwrap();
    assert_eq!(world.entity_id(&at_limit), Some(id));

    // Combining marks count individually; visible graphemes are not the bound.
    let combining_limit = "e\u{301}".repeat(MAX_AGENT_NAME_CHARS / 2);
    world.rename_agent(id, &combining_limit).unwrap();
    assert_eq!(
        world.rename_agent(id, &(combining_limit.clone() + "x")),
        Err(RenameAgentError::TooLong {
            max_chars: MAX_AGENT_NAME_CHARS,
        })
    );
    assert_eq!(world.entity_id(&combining_limit), Some(id));

    world.rename_agent(id, "Straße").unwrap();
    let other = world.entity_id("Élodie").unwrap();
    world.rename_agent(other, "STRASSE").unwrap();
    assert_eq!(world.entity_id("Straße"), Some(id));
    assert_eq!(world.entity_id("STRASSE"), Some(other));
}

#[test]
fn rename_rejects_missing_and_non_agent_targets_without_mutation() {
    let mut world = world();
    let before = world.canonical_signature_digest().unwrap();
    for id in [
        WorldEntityId(0),
        WorldEntityId(u64::MAX),
        world.entity_id("İRIS").unwrap(),
    ] {
        assert_eq!(
            world.rename_agent(id, "Pip"),
            Err(RenameAgentError::InvalidTarget)
        );
        assert_eq!(world.canonical_signature_digest().unwrap(), before);
        assert_eq!(world.entity_id("Pip"), None);
    }
}

#[test]
fn generated_object_prefixes_are_reserved_without_mutating_world_state_or_lookup() {
    let mut world = HeadlessScenarioBuilder::new(73_206)
        .agent("agent", OrganismId(7), Vec3f::ZERO)
        .terrain_zone(
            1,
            "grove",
            TerrainZoneKind::Grove,
            Vec3f::ZERO,
            2.0,
            0.9,
            0.0,
        )
        .resource_spawn_policy("Berry", 1, 1, 2, 0.4)
        .resource_spawn_policy("İRIS", 1, 50, 2, 0.4)
        .build()
        .unwrap();
    let id = world.entity_id("agent").unwrap();
    let before = world.canonical_signature_digest().unwrap();
    let objects = world.object_snapshots();
    for name in [
        "voice-token-7",
        " VOICE-TOKEN-future ",
        "organism-999",
        "ORGANISM-future",
        "Berry-0",
        "BERRY-future",
        "i\u{307}ris-0",
    ] {
        assert_eq!(
            world.rename_agent(id, name),
            Err(RenameAgentError::ReservedName),
            "{name:?}"
        );
        assert_eq!(world.canonical_signature_digest().unwrap(), before);
        assert_eq!(world.object_snapshots(), objects);
        assert_eq!(world.entity_id("agent"), Some(id));
        assert_eq!(world.entity_id(name.trim()), None);
    }

    // The hyphen is part of each reserved prefix, so ordinary names stay usable.
    for name in ["voice-token", "organism", "Berry", "Berry Blossom"] {
        world.rename_agent(id, name).unwrap();
        assert_eq!(world.entity_id(name), Some(id));
    }
}

#[test]
fn existing_generated_agent_names_can_be_kept_without_opening_a_reserved_alias() {
    let mut world = HeadlessScenarioBuilder::new(73_207)
        .agent("organism-9", OrganismId(9), Vec3f::ZERO)
        .agent("VOICE-TOKEN-7", OrganismId(8), Vec3f::new(4.0, 0.0, 0.0))
        .build()
        .unwrap();
    let child = world.entity_id("organism-9").unwrap();
    let legacy = world.entity_id("VOICE-TOKEN-7").unwrap();
    let before = world.canonical_signature_digest().unwrap();
    world.rename_agent(child, " organism-9 ").unwrap();
    world.rename_agent(legacy, "VOICE-TOKEN-7").unwrap();
    assert_eq!(world.canonical_signature_digest().unwrap(), before);

    for (id, name) in [(child, "organism-10"), (legacy, "voice-token-7")] {
        assert_eq!(
            world.rename_agent(id, name),
            Err(RenameAgentError::ReservedName)
        );
        assert_eq!(world.canonical_signature_digest().unwrap(), before);
        assert_eq!(world.entity_id("organism-9"), Some(child));
        assert_eq!(world.entity_id("VOICE-TOKEN-7"), Some(legacy));
        assert_eq!(world.entity_id(name), None);
    }
}
