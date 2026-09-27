//! Complete mutations of factory-held Techno identity and its accounting.
//!
//! The registry owns queue/charge kernels. This world-facing owner completes
//! their constructor, disposal, ready projection and successor work before an
//! operation returns. Native StartProduction/AbandonProduction/StartNextQueued:
//! 0x004C9C70 / 0x004CA0E0 / 0x004CA5A0; see FACTORY_CREDIT_SYSTEM_GHIDRA_REPORT.md.
//! Delivery selection and placement keep their existing phase positions and
//! call settlement only after their successful world effects have committed.

use super::CancelOutcome;
use super::factory::{time_to_build, time_to_build_inputs};
use super::production_queue::{credits_entry_for_owner, credits_for_owner};
use super::production_tech::{
    build_option_for_owner, production_category_for_object, should_use_relaxed_build_mode,
    supports_live_production,
};
use super::production_types::{BuildMode, ProductionCategory};
use crate::rules::object_type::ObjectCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::intern::InternedId;
use crate::sim::world::{SimSoundEvent, Simulation};

/// Enqueue a specific unit type.
pub fn enqueue_by_type(sim: &mut Simulation, rules: &RuleSet, owner: &str, type_id: &str) -> bool {
    let relaxed: bool = should_use_relaxed_build_mode(sim, rules, owner);
    let mode = if relaxed {
        BuildMode::PrototypeRelaxed
    } else {
        BuildMode::Strict
    };
    if let Some(opt) = build_option_for_owner(sim, rules, owner, type_id, mode) {
        if !opt.enabled {
            return false;
        }
    } else {
        return false;
    }
    let Some(obj) = rules.object(type_id) else {
        return false;
    };
    if !supports_live_production(obj) {
        return false;
    }
    let queue_category = production_category_for_object(obj);
    let owner_credits = credits_for_owner(sim, owner);
    if obj.cost <= 0 || owner_credits < obj.cost {
        return false;
    }
    // The upfront debit is RETIRED at the authority flip: the per-step `advance_one_step`
    // (driven by `step_all` at the Phase-7 head) charges the cost down over the build
    // against the one wallet (`house.economy.credits`). Enqueue only checks affordability (the
    // can-afford-to-START gate above) and appends the queue item.
    let owner_id = sim.interner.intern(owner);
    let type_interned = sim.interner.intern(type_id);
    let enqueue_order = next_enqueue_order(sim);
    let cost = obj.cost.max(0);
    // P5d: append directly to the registry queue-of-record (create-or-append). With no
    // active build the registry arms it inline (the retired reconcile SEED); otherwise it
    // joins the FIFO tail. No upfront debit (the per-step charge owns the cost).
    let started = sim.production.factory_shadow.enqueue(
        owner_id,
        queue_category,
        type_interned,
        enqueue_order,
        cost,
    );
    if started {
        start_active_production(sim, rules, owner_id, queue_category, type_interned)
            .expect("validated StartProduction type must construct one Techno");
    }
    true
}

fn next_enqueue_order(sim: &mut Simulation) -> u64 {
    let order = sim.production.next_enqueue_order;
    sim.production.next_enqueue_order = sim.production.next_enqueue_order.saturating_add(1);
    order
}
/// Start the build an active factory head holds. `HouseClass::Begin_Production
/// @ 0x004FA350` runs `FactoryClass::StartProduction @ 0x004C9C70`, which calls
/// `type->CreateInstance(owner)` and stores the result at `Factory+0x58`, then
/// the build start `0x004C9EA0` (Ghidra label `FactoryClass__SetRate`) at
/// `0x004FA628`, which arms the step rate and timer from that object's
/// `Time_To_Build`. Queued tail entries start only when
/// `FactoryClass::StartNextQueued @ 0x004CA5A0` promotes them, which runs the
/// same Begin_Production (`0x004CA60A`).
///
/// Begin_Production's network headstart (`0x004FA631..0x004FA68F`) never runs:
/// it needs the factory unsuspended just before the start (read at
/// `0x004FA622`), and every path there has just suspended it (StartProduction's
/// create path, `0x004C9D72`) or resumed a suspended one (`0x004FA5A8..0x004FA5C6`).
/// A queue append (`0x004C9D22..0x004C9D2E`) returns at `0x004FA612` first, and
/// a promotion's StartProduction never appends (`0x004C9CC9..0x004C9CCF`).
fn start_active_production(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner_id: InternedId,
    category: ProductionCategory,
    type_id: InternedId,
) -> Option<u64> {
    let obj = sim.object_type(type_id, rules)?;
    let owner = sim.interner.resolve(owner_id).to_string();
    let type_name = sim.interner.resolve(type_id).to_string();
    // A factory-held object is still in limbo and has no cell authority. Zero
    // is only inert storage here; the result-bearing Unlimbo later installs the
    // selected delivery/placement coordinate on this same stable identity.
    let stable_id = sim.construct_object_limbo_at_height(&type_name, &owner, 0, 0, 0, 0, rules)?;
    let linked = sim
        .production
        .factory_shadow
        .link_active_entity(owner_id, category, stable_id);
    if linked != Some(stable_id) {
        let _ = sim.discard_constructed_limbo(stable_id);
        return None;
    }
    let time_to_build = time_to_build(&time_to_build_inputs(sim, rules, owner_id, category, obj));
    let frame = sim.session.binary_frame;
    sim.production
        .factory_shadow
        .start_rate(owner_id, category, time_to_build, frame);
    Some(stable_id)
}
/// Cancel the most recently queued item for this owner.
///
/// Post-flip refund rule: a queued (tail) item was never charged, so removing it
/// refunds NOTHING; only the active build (a single-item queue, where the most-recent
/// item IS the front) is abandoned with the C8 PARTIAL refund (`original_balance -
/// balance`) routed through the registry against the one wallet (`house.economy.credits`).
pub fn cancel_last_for_owner(sim: &mut Simulation, _rules: &RuleSet, owner: &str) -> bool {
    let owner_id = sim.interner.intern(owner);
    // P5d: the registry owns the queue-of-record. `cancel_last` finds the global-max stamp
    // across the owner's factories (tail-back, else the active build) and removes it — a
    // tail item uncharged (QueuedRemoved), the active build with the C8 PARTIAL refund. The
    // abandon arm only fires for an empty tail, so no StartNextQueued advance is needed.
    let mut registry = std::mem::take(&mut sim.production.factory_shadow);
    let outcome = if let Some(house) = sim.houses.get_mut(&owner_id) {
        registry.cancel_last(owner_id, &mut house.economy)
    } else {
        let mut throwaway = crate::sim::economy::Economy::default();
        registry.cancel_last(owner_id, &mut throwaway)
    };
    registry.prune_all_idle();
    sim.production.factory_shadow = registry;
    if let CancelOutcome::AbandonedActive {
        entity_id: Some(entity_id),
        ..
    } = outcome
    {
        let discarded = sim.discard_constructed_limbo(entity_id);
        debug_assert!(
            discarded,
            "AbandonProduction destroys the held limbo object"
        );
    }
    matches!(
        outcome,
        CancelOutcome::QueuedRemoved | CancelOutcome::AbandonedActive { .. }
    )
}

/// Route a cancel of `type_id` for (owner, category) through the registry `cancel_one`
/// (the single precedence source: queued-tail FIRST, else active-abandon), charging the
/// C8 partial refund (or none, for a queued copy) against the ONE wallet
/// (`house.economy.credits`). The private caller completes held-object disposal and promotion before returning.
fn registry_cancel_active(
    sim: &mut Simulation,
    owner_id: InternedId,
    category: ProductionCategory,
    type_id: InternedId,
) -> CancelOutcome {
    let mut registry = std::mem::take(&mut sim.production.factory_shadow);
    let outcome = if let Some(house) = sim.houses.get_mut(&owner_id) {
        registry.cancel_one(owner_id, category, type_id, &mut house.economy)
    } else {
        // No house to refund into; the cancel still resolves the registry deterministically.
        let mut throwaway = crate::sim::economy::Economy::default();
        registry.cancel_one(owner_id, category, type_id, &mut throwaway)
    };
    sim.production.factory_shadow = registry;
    outcome
}

/// Cancel one queued/active production of `type_id` for this owner (right-click cameo).
///
/// Routed through the registry `cancel_one` (the single precedence source): a QUEUED
/// tail copy is removed FIRST (FIRST front-to-back match, NO refund — a queued item was
/// never charged), else the ACTIVE build is abandoned with the C8 PARTIAL refund
/// (`original_balance - balance`) into the one wallet (`house.economy.credits`). This replaces
/// the legacy `.rev()` last-match + full-cost refund (a DRIFT under the per-step charge).
/// When neither matches (or the build is complete-but-held), falls back to the
/// completed-building ready queue.
pub fn cancel_by_type_for_owner(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    type_id: &str,
) -> bool {
    let owner_id = sim.interner.intern(owner);
    let type_interned = sim.interner.intern(type_id);
    // The registry is keyed by ProductionCategory; resolve it from the type (the same
    // routing `enqueue_by_type` used, so the item is in this category's queue).
    let category = match rules.object(type_id) {
        Some(obj) => production_category_for_object(obj),
        None => return cancel_ready_by_type_for_owner(sim, rules, owner, type_id),
    };

    match registry_cancel_active(sim, owner_id, category, type_interned) {
        CancelOutcome::QueuedRemoved => {
            // A queued (tail) copy was removed in the registry; the active build keeps
            // running. Sweep any now-idle factory (none here, but keep it uniform).
            sim.production.factory_shadow.prune_all_idle();
            true
        }
        CancelOutcome::AbandonedActive { entity_id, .. } => {
            if let Some(entity_id) = entity_id {
                let discarded = sim.discard_constructed_limbo(entity_id);
                debug_assert!(
                    discarded,
                    "AbandonProduction destroys the held limbo object"
                );
            }
            // C7: the active build was abandoned (object cleared, tail intact). Promote the
            // next queued entry into the active slot, cost-seeded and started.
            advance_after_delivery(sim, rules, owner_id, category);
            sim.production.factory_shadow.prune_all_idle();
            true
        }
        CancelOutcome::NoMatch => {
            // Not an active/queued build (or a complete-but-held one) -> the ready queue.
            cancel_ready_by_type_for_owner(sim, rules, owner, type_id)
        }
    }
}

/// Cancel a completed building from the ready_by_owner queue (awaiting placement).
/// Used as fallback when `cancel_by_type_for_owner` finds nothing in the build queue.
fn cancel_ready_by_type_for_owner(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    type_id: &str,
) -> bool {
    let owner_id = sim.interner.intern(owner);
    let type_interned = sim.interner.intern(type_id);
    let Some(ready_queue) = sim.production.ready_by_owner.get_mut(&owner_id) else {
        return false;
    };
    // Remove last instance of this type (consistent with queue cancel using .rev()).
    let ready_idx = ready_queue
        .iter()
        .enumerate()
        .rev()
        .find(|(_, tid)| **tid == type_interned)
        .map(|(i, _)| i);
    let Some(idx) = ready_idx else {
        return false;
    };
    ready_queue.remove(idx);
    if ready_queue.is_empty() {
        sim.production.ready_by_owner.remove(&owner_id);
    }
    let category = rules
        .object(type_id)
        .map(production_category_for_object)
        .unwrap_or(ProductionCategory::Building);
    let held_entity_id = sim
        .production
        .factory_shadow
        .view(owner_id, category)
        .and_then(|view| view.object)
        .filter(|object| object.type_id == type_interned)
        .and_then(|object| object.entity_id);
    // Refund full cost.
    if let Some(obj) = rules.object(type_id) {
        *credits_entry_for_owner(sim, owner) += obj.cost.max(0);
    }
    if let Some(entity_id) = held_entity_id {
        let discarded = sim.discard_constructed_limbo(entity_id);
        debug_assert!(
            discarded,
            "ready-building cancel destroys Factory+0x58 object"
        );
    }
    advance_after_delivery(sim, rules, owner_id, category);
    true
}
/// C7 StartNextQueued after a successful delivery (or a completed-but-undeliverable refund):
/// clear the delivered active object and promote the next queued entry into the active slot,
/// cost-seeded from `rules` and started at this frame.
///
/// Residual: gamemd delivers a human player's finished unit through a PLACE event
/// that `StripClass::AI` queues (`0x006A8EB8..0x006A8F18`). Its execution
/// (`0x004C710B` -> `HouseClass::Place_Production @ 0x004FB0E0`) unlimbos the unit
/// and promotes the next build (Abandon_Production at `0x004FB663` ->
/// StartNextQueued at `0x004FAC96`). VERA delivers and promotes in the completion
/// frame, so for every unit a human builds, the unit appears and the next queued
/// build starts earlier by the event's scheduling delay (untraced).
fn advance_after_delivery(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner_id: InternedId,
    category: ProductionCategory,
) {
    let next_cost = sim
        .production
        .factory_shadow
        .peek_next_queued(owner_id, category)
        .map(|t| sim.object_type(t, rules).map_or(0, |o| o.cost.max(0)))
        .unwrap_or(0);
    let promoted = sim
        .production
        .factory_shadow
        .clear_active_and_advance(owner_id, category, next_cost);
    if let Some(type_id) = promoted {
        start_active_production(sim, rules, owner_id, category, type_id)
            .expect("validated promoted production type must construct one Techno");
    }
}
pub(super) fn active_entity_id(
    sim: &Simulation,
    owner_id: InternedId,
    category: ProductionCategory,
) -> Option<u64> {
    sim.production
        .factory_shadow
        .view(owner_id, category)
        .and_then(|view| view.object.and_then(|object| object.entity_id))
        .filter(|&stable_id| sim.substrate.entities.contains(stable_id))
}
fn discard_active_factory_entity(
    sim: &mut Simulation,
    owner_id: InternedId,
    category: ProductionCategory,
) {
    if let Some(stable_id) = active_entity_id(sim, owner_id, category) {
        let discarded = sim.discard_constructed_limbo(stable_id);
        debug_assert!(
            discarded,
            "factory-held object must remain in limbo until delivery"
        );
    }
}
fn consume_ready_building(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner_id: crate::sim::intern::InternedId,
    type_id: crate::sim::intern::InternedId,
    category: ProductionCategory,
) -> bool {
    let Some(ready_queue) = sim.production.ready_by_owner.get_mut(&owner_id) else {
        return false;
    };
    let Some(index) = ready_queue.iter().position(|&queued| queued == type_id) else {
        return false;
    };
    ready_queue.remove(index);
    if ready_queue.is_empty() {
        sim.production.ready_by_owner.remove(&owner_id);
    }
    advance_after_delivery(sim, rules, owner_id, category);
    true
}
/// Publish the completion edge once, without releasing the held object.
/// Native counts completion before delivery; refusal must not count again.
pub(super) fn publish_completion(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    category: ProductionCategory,
) {
    let Some(type_id) = sim
        .production
        .factory_shadow
        .view(owner, category)
        .and_then(|view| view.object.map(|object| object.type_id))
    else {
        return;
    };
    if !sim
        .production
        .factory_shadow
        .account_completed_object_once(owner, category)
    {
        return;
    }
    if let Some(house) = sim.houses.get_mut(&owner) {
        house.stats.built = house.stats.built.saturating_add(1);
    }
    if rules
        .object(sim.interner.resolve(type_id))
        .is_some_and(|object| object.category == ObjectCategory::Building)
    {
        sim.production
            .ready_by_owner
            .entry(owner)
            .or_default()
            .push_back(type_id);
        sim.sound_events
            .push(SimSoundEvent::BuildingComplete { owner });
    }
}

/// Delivery effects already committed; retain the revealed identity and start
/// its successor. A refused vehicle delivery must never call this operation.
pub(super) fn release_delivered_mobile(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    category: ProductionCategory,
) {
    advance_after_delivery(sim, rules, owner, category);
}

/// Terminal mobile failure refunds the authored full cost, destroys the held
/// graph and starts the successor. This is distinct from a retryable refusal.
pub(super) fn refund_failed_delivery(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    category: ProductionCategory,
) {
    let type_id = sim
        .production
        .factory_shadow
        .view(owner, category)
        .and_then(|view| view.object.map(|object| object.type_id));
    if let Some(object) = type_id.and_then(|type_id| rules.object(sim.interner.resolve(type_id))) {
        let owner_name = sim.interner.resolve(owner).to_string();
        *credits_entry_for_owner(sim, &owner_name) += object.cost.max(0);
    }
    discard_active_factory_entity(sim, owner, category);
    advance_after_delivery(sim, rules, owner, category);
}

/// Matching identity captured by a successful placement admission. The caller
/// cannot combine an entity ID with another owner's type or factory category.
pub(super) struct ReadyFactoryObject {
    owner: InternedId,
    category: ProductionCategory,
    type_id: InternedId,
    entity_id: u64,
}

impl ReadyFactoryObject {
    pub(super) fn entity_id(&self) -> u64 {
        self.entity_id
    }

    /// Building Unlimbo/build-up/superweapon effects precede factory release.
    pub(super) fn release_after_placement(self, sim: &mut Simulation, rules: &RuleSet) -> bool {
        consume_ready_building(sim, rules, self.owner, self.type_id, self.category)
    }

    /// Primary and autofill overlays are already stamped when the constructor
    /// identity is consumed. Ready removal and successor construction follow.
    pub(super) fn consume_after_wall_stamp(self, sim: &mut Simulation, rules: &RuleSet) -> bool {
        let _ = sim.discard_constructed_limbo(self.entity_id);
        consume_ready_building(sim, rules, self.owner, self.type_id, self.category)
    }
}

pub(super) fn ready_object(
    sim: &Simulation,
    owner: InternedId,
    category: ProductionCategory,
    type_id: InternedId,
) -> Option<ReadyFactoryObject> {
    let object = sim
        .production
        .factory_shadow
        .view(owner, category)?
        .object?;
    if object.type_id != type_id {
        return None;
    }
    let entity_id = object
        .entity_id
        .filter(|&id| sim.substrate.entities.contains(id))?;
    Some(ReadyFactoryObject {
        owner,
        category,
        type_id,
        entity_id,
    })
}

/// Fixture-only bridge for tests that deliberately seed a registry kernel.
#[cfg(test)]
pub(in crate::sim) fn construct_active_factory_fixture(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    category: ProductionCategory,
    type_id: InternedId,
) -> Option<u64> {
    start_active_production(sim, rules, owner, category, type_id)
}

/// Revalidate before the charge sweep at its existing frame phase. Dispose all
/// abandoned objects before starting any promoted ones.
pub(in crate::sim) fn revalidate_and_step_factories(sim: &mut Simulation, rules: &RuleSet) {
    let mut registry = std::mem::take(&mut sim.production.factory_shadow);
    // P6: prereq/factory-loss revalidation BEFORE the charge sweep. Builds whose
    // prerequisites or producing factory were lost are abandoned (partial refund)
    // + now-unbuildable queued items dropped, so a freshly-abandoned factory is not
    // charged this tick; a promoted build is started at this frame.
    let reval_plan = registry.plan_revalidation(sim, rules);
    let lifecycle = registry.apply_revalidation(&reval_plan, &mut sim.houses);
    for entity_id in lifecycle.discarded_entity_ids {
        let discarded = sim.discard_constructed_limbo(entity_id);
        debug_assert!(
            discarded,
            "prerequisite AbandonProduction destroys its held limbo object"
        );
    }
    // An abandoned finished building no longer waits for placement.
    for (owner, type_id) in lifecycle.abandoned_finished {
        if let Some(ready_queue) = sim.production.ready_by_owner.get_mut(&owner)
            && let Some(index) = ready_queue.iter().position(|&ready| ready == type_id)
        {
            ready_queue.remove(index);
            if ready_queue.is_empty() {
                sim.production.ready_by_owner.remove(&owner);
            }
        }
    }
    sim.production.factory_shadow = registry;
    for (owner, category, type_id) in lifecycle.promoted {
        start_active_production(sim, rules, owner, category, type_id)
            .expect("validated revalidation promotion must construct one Techno");
    }
    let mut registry = std::mem::take(&mut sim.production.factory_shadow);
    let prepared = registry.prepare_step_inputs(sim, rules);
    registry.step_all(&mut sim.houses, &prepared, sim.session.binary_frame);
    sim.production.factory_shadow = registry;
}

/// A saved relationship that the live factory operations cannot publish.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("factory state for owner {owner}: {reason}")]
pub(crate) struct FactoryRestoreError {
    pub(crate) owner: InternedId,
    pub(crate) reason: &'static str,
}

/// Validate factory relationships before a loaded candidate becomes a match.
///
/// VERA-internal snapshot admission; gamemd equivalent UNCHECKED. These checks
/// follow the synchronous constructor/publication/settlement operations above,
/// not a new queue, refund or gameplay policy. Generic snapshot reference and
/// LogicVector validation precede this operation. Missing Houses and unrelated
/// limbo objects remain supported; manager pointers are not globally reciprocal.
pub(crate) fn validate_restored_factory_state(
    sim: &Simulation,
    rules: &RuleSet,
) -> Result<(), FactoryRestoreError> {
    use crate::map::entities::EntityCategory;
    use std::collections::{BTreeMap, BTreeSet};

    let fail = |owner, reason| FactoryRestoreError { owner, reason };
    let mut ready = BTreeMap::new();
    for (&owner, entries) in &sim.production.ready_by_owner {
        if sim.interner.try_resolve(owner).is_none() {
            return Err(fail(owner, "ready owner is absent from the interner"));
        }
        for &type_id in entries {
            let object = sim
                .interner
                .try_resolve(type_id)
                .and_then(|name| rules.object(name))
                .ok_or_else(|| fail(owner, "ready type is absent from bound rules"))?;
            if object.category != ObjectCategory::Building {
                return Err(fail(owner, "ready entry is not a building"));
            }
            let category = production_category_for_object(object);
            if ready.insert((owner, category), type_id).is_some() {
                return Err(fail(owner, "multiple ready entries claim one factory"));
            }
            let factory = sim
                .production
                .factory_shadow
                .view(owner, category)
                .ok_or_else(|| fail(owner, "ready entry has no factory"))?;
            let held = factory
                .object
                .ok_or_else(|| fail(owner, "ready entry has no active object"))?;
            if held.type_id != type_id {
                return Err(fail(owner, "ready type disagrees with active object"));
            }
            if !factory.ready || !held.completion_accounted {
                return Err(fail(owner, "ready entry precedes completion publication"));
            }
        }
    }

    let mut roots = BTreeSet::new();
    for (&(owner, category), factory) in sim.production.factory_shadow.keyed_factories() {
        if factory.owner != owner || factory.category != category {
            return Err(fail(owner, "registry key disagrees with factory identity"));
        }
        if sim.interner.try_resolve(owner).is_none() {
            return Err(fail(owner, "factory owner is absent from the interner"));
        }
        for queued in &factory.queue {
            let object = sim
                .interner
                .try_resolve(queued.type_id)
                .and_then(|name| rules.object(name))
                .ok_or_else(|| fail(owner, "queued type is absent from bound rules"))?;
            if production_category_for_object(object) != category {
                return Err(fail(owner, "queued type disagrees with factory category"));
            }
        }
        let Some(held) = &factory.object else {
            if !factory.queue.is_empty() {
                return Err(fail(owner, "queued tail has no active object"));
            }
            continue;
        };
        let object = sim
            .interner
            .try_resolve(held.type_id)
            .and_then(|name| rules.object(name))
            .ok_or_else(|| fail(owner, "active type is absent from bound rules"))?;
        if production_category_for_object(object) != category {
            return Err(fail(owner, "active type disagrees with factory category"));
        }
        let entity_id = held
            .entity_id
            .ok_or_else(|| fail(owner, "active object has no constructed identity"))?;
        if !roots.insert(entity_id) {
            return Err(fail(
                owner,
                "constructed identity belongs to multiple factories",
            ));
        }
        let entity = sim.substrate.entities.get(entity_id).ok_or_else(|| {
            fail(
                owner,
                "constructed identity is absent from the entity store",
            )
        })?;
        if entity.owner() != owner || entity.type_ref() != held.type_id {
            return Err(fail(
                owner,
                "constructed identity disagrees with factory owner or type",
            ));
        }
        let expected_category = match object.category {
            ObjectCategory::Infantry => EntityCategory::Infantry,
            ObjectCategory::Vehicle => EntityCategory::Unit,
            ObjectCategory::Aircraft => EntityCategory::Aircraft,
            ObjectCategory::Building => EntityCategory::Structure,
        };
        if entity.category != expected_category {
            return Err(fail(
                owner,
                "constructed identity has the wrong concrete category",
            ));
        }
        if entity.spawn_owner_id.is_some() || entity.slave.owner().is_some() {
            return Err(fail(owner, "factory root is itself a manager child"));
        }
        // Factory admission validates retained identity and membership, without
        // inferring health or death flags. Terminal-state admission separately
        // validates the object's lifecycle handoff.
        if !entity.lifecycle.in_limbo || entity.lifecycle.cell_marked || entity.in_logic_vector {
            return Err(fail(
                owner,
                "factory object is already admitted to the world",
            ));
        }
        if held.completion_accounted && factory.progress < super::PRODUCTION_STEPS {
            return Err(fail(
                owner,
                "completion accounting precedes completed progress",
            ));
        }
        if held.completion_accounted
            && object.category == ObjectCategory::Building
            && ready.get(&(owner, category)) != Some(&held.type_id)
        {
            return Err(fail(owner, "accounted building has no ready entry"));
        }
    }

    // Restrict only factory roots: native manager pointers elsewhere can alias
    // without implying reciprocal ownership, and ordinary limbo is not a root.
    for (&(owner, _), factory) in sim.production.factory_shadow.keyed_factories() {
        let Some(parent) = factory.object.as_ref().and_then(|object| object.entity_id) else {
            continue;
        };
        let entity = sim
            .substrate
            .entities
            .get(parent)
            .expect("validated factory root");
        let spawn_alias = entity.spawn_manager.as_ref().is_some_and(|manager| {
            manager
                .slots
                .iter()
                .filter_map(|slot| slot.spawn)
                .any(|id| roots.contains(&id))
        });
        let slave_alias = entity
            .slave_manager
            .as_ref()
            .is_some_and(|manager| manager.slaves().any(|id| roots.contains(&id)));
        if spawn_alias || slave_alias {
            return Err(fail(owner, "factory root aliases a held constructor child"));
        }
    }
    Ok(())
}
