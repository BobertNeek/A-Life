//! Opt-in body-only contact response. No semantic target or brain policy.
use super::*;

impl HeadlessWorld {
    pub(super) fn apply_body_only_contact_response(
        &mut self,
        organism: OrganismId,
        own: WorldEntityId,
        outcome_tick: Tick,
        mut event: BodyEventDelta,
        unlocalized_damage: f32,
    ) -> Result<(BodyEventDelta, Option<BodyOnlyContactResponse>), ScaffoldContractError> {
        let record = self
            .organism_registry
            .get(organism)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let biology = record.biochemistry();
        let phenotype = record.phenotype();
        let capacity = biology
            .graph_state()
            .locomotor_capacity(&phenotype.chemistry.biochemical)?
            .map(|chemical| {
                let organ = biology.body.organ(alife_core::OrganKind::Locomotor);
                let reserve = (organ.energy / (0.5 + 0.5 * phenotype.body.metabolic_efficiency))
                    .clamp(0.0, 1.0);
                chemical * reserve * organ.integrity
            });
        let gain = record
            .embodiment()
            .effector_gain(alife_core::EffectorCapability::Translation);
        let carried_positions = self.carried_interval_positions(organism, None);
        // Do not convert self-carried surface damage into an invented external
        // normal or a runaway fallback. Only separately supplied acute damage
        // is unlocalized; actual non-carried contacts are recomputed below.
        let withdrawal =
            self.apply_contact_withdrawal(organism, own, capacity, gain, unlocalized_damage)?;
        let requested_sleep_recovery = event.sleep_recovery;
        if let Some(reflex) = &withdrawal {
            event.energy = (event.energy - reflex.energy_cost).clamp(-1.0, 1.0);
            for source in &reflex.sources {
                event.damage = event.damage.max(self.objects[&source.raw()].hazard_pain);
            }
            self.commit_current_interval_velocity(organism, reflex.displacement)?;
            self.commit_carried_interval_velocity(organism, &carried_positions);
        }
        event =
            normalize_sleep_recovery_for_motion(event, withdrawal.iter().map(|r| r.displacement));
        let sleep_recovery_interrupted = event.damage > 0.0 && requested_sleep_recovery > 0.0;
        // Fresh physical dose interrupts recovery even when motion is blocked.
        // Chronic accumulated injury alone never triggers this response.
        if event.damage > 0.0 {
            event.sleep_recovery = 0.0;
        }
        event.validate_contract()?;
        if withdrawal.is_none() && event.damage <= 0.0 {
            return Ok((event, None));
        }
        Ok((
            event,
            Some(BodyOnlyContactResponse {
                schema_version: 1,
                organism_id: organism,
                outcome_tick,
                withdrawal,
                requested_sleep_recovery,
                sleep_recovery_interrupted,
                applied_body_event: event,
            }),
        ))
    }

    pub(super) fn apply_contact_withdrawal(
        &mut self,
        organism: OrganismId,
        own: WorldEntityId,
        locomotor_capacity: Option<f32>,
        effector_gain: f32,
        acute_damage: f32,
    ) -> Result<Option<ContactWithdrawalReceipt>, ScaffoldContractError> {
        let body = self
            .objects
            .get(&own.raw())
            .ok_or(ScaffoldContractError::InvalidId)?;
        let start = body.position;
        let yaw = body.body_yaw;
        let sources = self
            .objects
            .values()
            .filter(|o| {
                o.id != own
                    && o.carried_by != Some(organism)
                    && !o.consumed
                    && o.kind == WorldObjectKind::Hazard
                    && o.hazard_pain > 0.0
                    && distance(o.position, start) <= o.radius
            })
            .map(|o| o.id)
            .collect::<Vec<_>>();
        if sources.len() > ContactWithdrawalReceipt::MAX_SOURCES {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        if sources.is_empty() && acute_damage <= 0.0 {
            return Ok(None);
        }
        let mut receipt = ContactWithdrawalReceipt {
            direction_basis: WithdrawalDirectionBasis::LocalizedContact,
            sources,
            displacement: Vec3f::ZERO,
            blocked: true,
            energy_cost: 0.0,
        };
        let mut direction = Vec3f::ZERO;
        let mut unknown_direction = receipt.sources.is_empty();
        for source in &receipt.sources {
            let object = &self.objects[&source.raw()];
            let away = subtract(start, object.position);
            let norm = away.x.hypot(away.z);
            // No arbitrary direction for coincident or vertically aligned sources.
            if norm <= 1e-6 {
                unknown_direction = true;
                continue;
            }
            direction.x += away.x / norm * object.hazard_pain;
            direction.z += away.z / norm * object.hazard_pain;
        }
        if unknown_direction {
            if acute_damage <= 0.0 && receipt.sources.is_empty() {
                return Ok(Some(receipt));
            }
            direction = Vec3f::new(-yaw.cos(), 0.0, -yaw.sin());
            receipt.direction_basis = WithdrawalDirectionBasis::AcuteDamageHeadingFallback;
        }
        let norm = direction.x.hypot(direction.z);
        let capacity =
            locomotor_capacity.unwrap_or(1.0).clamp(0.0, 1.0) * effector_gain.clamp(0.0, 1.0);
        if norm <= 1e-6 || capacity <= 0.0 || self.player_held_object() == Some(own) {
            return Ok(Some(receipt));
        }
        // One short interval, never a path search, repeated escape plan or teleport.
        let step = MOVE_STEP * capacity;
        let proposed = add(
            start,
            Vec3f::new(direction.x / norm * step, 0.0, direction.z / norm * step),
        );
        receipt.energy_cost = 0.02 * capacity;
        let destination = if let Some(terrain) = &self.terrain {
            let Some(grounded) = terrain.resolve_move(start, proposed) else {
                return Ok(Some(receipt));
            };
            grounded
        } else {
            proposed
        };
        // Conflicting contact normals cannot be resolved by inventing a route.
        if receipt.sources.iter().any(|id| {
            let origin = self.objects[&id.raw()].position;
            let away = subtract(start, origin);
            let movement = subtract(destination, start);
            if away.x.hypot(away.z) <= 1e-6 {
                distance(origin, destination) <= distance(origin, start)
            } else {
                away.x * movement.x + away.z * movement.z <= 0.0
            }
        }) || self.first_motion_blocker(start, destination).is_some()
            || !self.carried_pose_reachable(organism, destination, yaw)
        {
            return Ok(Some(receipt));
        }
        // Do not push peers or enter an unrelated painful contact. This response
        // has no authority to solve obstacle avoidance or choose an escape goal.
        // Existing overlap may separate monotonically, as in ordinary movement.
        // Own carried hazards move with the already validated grip pose.
        for other in self.objects.values().filter(|o| o.id != own && !o.consumed) {
            if other.kind == WorldObjectKind::Agent
                || (other.kind == WorldObjectKind::Hazard
                    && other.carried_by != Some(organism)
                    && other.hazard_pain > 0.0
                    && !receipt.sources.contains(&other.id))
            {
                let radius = other.radius
                    + if other.kind == WorldObjectKind::Agent {
                        self.locomotion_body_radius()
                    } else {
                        0.0
                    };
                if motion_sphere_entry(start, destination, other.position, radius).is_some() {
                    return Ok(Some(receipt));
                }
            }
        }
        self.objects
            .get_mut(&own.raw())
            .ok_or(ScaffoldContractError::InvalidId)?
            .position = destination;
        self.sync_carried_pose(organism)?;
        receipt.displacement = subtract(destination, start);
        receipt.blocked = false;
        Ok(Some(receipt))
    }
}
