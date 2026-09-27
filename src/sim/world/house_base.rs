//! The House's building lists and the cost factors its FactoryPlants set.
//!
//! House+68 (buildings) and House+140 (FactoryPlants) change membership at
//! Unlimbo, pointer expiry and ChangeOwner, which differs from EntityStore's
//! owner index; constructor/destructor tracking (`0x004FF700`/`0x004FF550`)
//! registers the rule inputs separately.
use super::*;
use crate::map::resolved_terrain::{NativeCellQuery, ResolvedTerrainGrid};
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::rules::ruleset::HouseCostFactors;
use crate::sim::find_nearby_cell::{
    NearbyAnchorGate, NearbyFootprint, NearbyQuery, PassabilityArgs, find_nearby_passable_cell,
    map_owned_radius_cap,
};
use crate::sim::pathfinding::zone_map::ZoneId;
use crate::util::native_x87::NativeF32Bits;

type CostFactors = [NativeF32Bits; 5];
const UNIT_FACTORS: CostFactors = [NativeF32Bits::ONE; 5];

/// Immutable rule projections retained beside the House registration. This
/// lets Unlimbo, ChangeOwner and pointer expiry use the same inputs without a
/// borrowed RuleSet or per-tick string lookup. Registration order is not list
/// order.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
struct RegisteredBuilding {
    id: u64,
    tracks_base: bool,
    /// The type's cost bonuses when it is a `FactoryPlant=` type.
    plant: Option<CostFactors>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct HouseBaseState {
    registrations: Vec<RegisteredBuilding>,
    /// Native House+68 (DynamicVectorClass, Items `+0x6C`, Count `+0x78`):
    /// Unlimbo appends (`0x00441553..0x00441594`), pointer expiry and
    /// ChangeOwner stable-remove (`0x004FBC1C`, `0x00448A78..0x00448AB0`) and
    /// ChangeOwner appends to the new House's tail (`0x00449197..0x004491D2`).
    /// The Prism walk reads it in this order ([`Self::buildings`]).
    buildings: Vec<u64>,
    /// Native House+140, whose order determines each f32 multiplication/store.
    plants: Vec<u64>,
    tracked_count: i32,
    factors: CostFactors,
    pub(crate) radius: i32,
}

impl Default for HouseBaseState {
    fn default() -> Self {
        Self {
            registrations: Vec::new(),
            buildings: Vec::new(),
            plants: Vec::new(),
            tracked_count: 0,
            factors: UNIT_FACTORS,
            radius: 0,
        }
    }
}

impl RegisteredBuilding {
    fn from_rules(id: u64, object: &ObjectType, rules: &RuleSet) -> Self {
        // Building+80=457620 ->465D40, then4FF76F tests the UNDEPLOY
        // TARGET's ResourceGatherer. This is not base-reservation eligibility.
        let undeploys_to_gatherer = object
            .undeploys_into
            .as_deref()
            .and_then(|name| rules.object(name))
            .is_some_and(|target| target.resource_gatherer);
        Self {
            id,
            tracks_base: !object.insignificant
                && !object.dont_score
                && !object.is_1x1_with_undeploy()
                && !undeploys_to_gatherer,
            plant: object.factory_plant.then_some(object.cost_bonuses),
        }
    }
}

impl HouseBaseState {
    /// The FactoryPlant products (House `+0x5390..+0x53A0`, in
    /// [`ObjectType::factor_slot`] order) `CalculateCostMultipliers @
    /// 0x0050BF60` last left; `HouseClass::GetAccumulatedBonus @ 0x0050BEB0`
    /// returns one slot.
    pub(crate) fn factory_plant_factors(&self) -> CostFactors {
        self.factors
    }

    /// House+68 in vector order.
    pub(crate) fn buildings(&self) -> &[u64] {
        &self.buildings
    }

    /// An oracle row's House+68, in its order; an id no object holds stands
    /// for a null entry.
    #[cfg(test)]
    pub(crate) fn replace_buildings_for_test(&mut self, buildings: Vec<u64>) {
        self.buildings = buildings;
    }

    fn registration(&self, id: u64) -> Option<&RegisteredBuilding> {
        self.registrations
            .binary_search_by_key(&id, |entry| entry.id)
            .ok()
            .map(|index| &self.registrations[index])
    }

    fn register(&mut self, entry: RegisteredBuilding) {
        match self
            .registrations
            .binary_search_by_key(&entry.id, |entry| entry.id)
        {
            Ok(_) => {}
            Err(index) => {
                self.tracked_count = self
                    .tracked_count
                    .wrapping_add(i32::from(entry.tracks_base));
                self.registrations.insert(index, entry);
            }
        }
    }

    fn unregister(&mut self, id: u64) -> Option<RegisteredBuilding> {
        let index = self
            .registrations
            .binary_search_by_key(&id, |entry| entry.id)
            .ok()?;
        let entry = self.registrations.remove(index);
        self.tracked_count = self
            .tracked_count
            .wrapping_sub(i32::from(entry.tracks_base));
        Some(entry)
    }

    fn remove_membership(&mut self, id: u64) {
        //4FB9B0 stable-removes one matching pointer, then50BF60 recomputes.
        remove_first(&mut self.plants, id);
        remove_first(&mut self.buildings, id);
        self.refresh_factors();
    }

    /// The appends Unlimbo (`0x004414F1..0x00441594`) and ChangeOwner's new
    /// House (`0x00449139..0x004491D2`) make: a `FactoryPlant=` type joins
    /// House+140 and the factors are recomputed (`0x0050BF60`), then the
    /// building joins the House+68 tail. The two vectors have independent
    /// lives.
    fn append_membership(&mut self, id: u64) {
        if self
            .registration(id)
            .is_some_and(|entry| entry.plant.is_some())
        {
            self.plants.push(id);
            self.refresh_factors();
        }
        self.buildings.push(id);
    }

    fn refresh_factors(&mut self) {
        let mut factors = UNIT_FACTORS;
        for &id in &self.plants {
            if let Some(bonuses) = self.registration(id).and_then(|entry| entry.plant) {
                for (factor, bonus) in factors.iter_mut().zip(bonuses) {
                    *factor = multiply_factor(*factor, bonus);
                }
            }
        }
        self.factors = factors;
    }
}

fn remove_first(ids: &mut Vec<u64>, id: u64) {
    if let Some(index) = ids.iter().position(|&candidate| candidate == id) {
        ids.remove(index);
    }
}

/// `0x0050BF60` multiplies two floats in x87 and stores the product with
/// exceptions masked under the chop control word: gradual underflow, signed
/// zero and saturation at the largest finite float.
/// Native comparison: tools/spatial_oracle/factory_plant_factors.
fn multiply_factor(lhs: NativeF32Bits, rhs: NativeF32Bits) -> NativeF32Bits {
    use crate::util::native_x87::MaskedX87Chop53 as X87;
    X87::store_f32_masked_chop(X87::mul(X87::load_f32(lhs), X87::load_f32(rhs)))
}

// Consumer pending: House 0x4FD150 base centre / nonhuman failed-path
// relocation 0x500200 (AI-deferred).

/// Map586E50: correct the diagonal first, sample its Cell once, then walk
/// along the other diagonal until578460(mode1) admits the packed coordinate.
/// The loop's predicate owns subsequent Cell/Dummy lookups independently.
#[allow(dead_code)]
fn clamp_house_cell(
    input: (i16, i16),
    cells: &NativeCellQuery<'_>,
    bounds: crate::sim::cell_rect::PlayfieldBounds,
) -> Result<(i16, i16), String> {
    let (mut x, mut y) = (i32::from(input.0), i32::from(input.1));
    let right = bounds
        .off_fc
        .wrapping_add(bounds.off_104)
        .wrapping_mul(2)
        .wrapping_sub(bounds.base);
    let left = bounds.base.wrapping_sub(bounds.off_fc.wrapping_mul(2));
    if x.wrapping_sub(y) >= right {
        let delta = x.wrapping_sub(y).wrapping_sub(right).wrapping_add(2) / 2;
        x = x.wrapping_sub(delta);
        y = y.wrapping_add(delta);
    } else if y.wrapping_sub(x) >= left {
        let delta = y.wrapping_sub(x).wrapping_sub(left).wrapping_add(2) / 2;
        y = y.wrapping_sub(delta);
        x = x.wrapping_add(delta);
    }
    let cell = cells.lookup((x as i16, y as i16));
    let (raw_level, slope) = cells.ground_fields(cell);
    let mut level = i32::from(raw_level as i8);
    let top = bounds.base.wrapping_add(bounds.off_100.wrapping_mul(2));
    let sum = x.wrapping_add(y);
    if slope != 0 && sum < top.wrapping_add(4).wrapping_add(level) {
        level = level.wrapping_add(1);
    }
    let bottom = bounds
        .base
        .wrapping_add(bounds.off_100.wrapping_add(bounds.off_108).wrapping_mul(2))
        .wrapping_add(2)
        .wrapping_add(level);
    let step = if sum <= top.wrapping_add(level) {
        1
    } else if sum > bottom {
        -1
    } else {
        return Ok((x as i16, y as i16));
    };
    // Packed words repeat after65536 steps. A complete cycle with no admitted
    // Cell is malformed map authority; never invent a substitute destination.
    for _ in 0..=u16::MAX {
        x = x.wrapping_add(step);
        y = y.wrapping_add(step);
        if crate::sim::cell_rect::cell_is_in_playfield_height_aware_in_query(
            (x, y),
            Some(bounds),
            Some(cells.terrain()),
            Some(cells),
        ) {
            return Ok((x as i16, y as i16));
        }
    }
    Err("House map clamp has no admitted packed cell on its diagonal".into())
}

impl Simulation {
    /// Shared56DC20 argument shape used by4FD2C0 and5002E5. Their speed
    /// and required zone differ; neither requests occupancy or height filtering.
    #[allow(dead_code)]
    fn house_nearby_cell(
        &self,
        seed: (i32, i32),
        speed_type: SpeedType,
        required_zone_id: Option<ZoneId>,
        terrain: &ResolvedTerrainGrid,
    ) -> Option<(u16, u16)> {
        let size = self
            .playfield_bounds
            .zip(self.playfield_size_height)
            .map(|(bounds, height)| (bounds.base, height))
            .or_else(|| self.bridge_state.as_ref()?.native_zone_source_size())?;
        let cells = NativeCellQuery::canonical(terrain);
        let grid = self.path_grid_snapshot();
        find_nearby_passable_cell(
            seed,
            &NearbyQuery {
                native_cells: Some(&cells),
                raw_occupation: Some(&self.substrate.raw_cell_occupation),
                passability: PassabilityArgs {
                    speed_type,
                    required_zone_id,
                    movement_zone: MovementZone::Normal,
                    bridge_aware_zone: false,
                },
                footprint: NearbyFootprint::SINGLE,
                anchor_gate: NearbyAnchorGate::NativeHeightAware,
                allow_bridge_cells: true,
                check_height: false,
                check_occupancy: false,
                radius_cap: map_owned_radius_cap(size.0, size.1),
                target_cell: None,
                path_grid: grid.as_deref(),
                resolved_terrain: Some(terrain),
                overlay_grid: self.overlay_grid.as_ref(),
                occupancy: Some(&self.substrate.occupancy),
                entities: Some(&self.substrate.entities),
                zone_grid: self.zone_grid.as_ref(),
                playfield_bounds: self.playfield_bounds,
            },
            self.session.binary_frame,
        )
        .filter(|cell| *cell != (0, 0))
    }

    /// The House factors TechnoType `Cost_Of` (`0x00711F00`) reads for
    /// `owner`: its country's `Cost*Mult=` and its FactoryPlant products.
    /// `None` without a House, which takes Cost_Of's null-House arm
    /// (`0x00711F4E`, the raw cost).
    pub(crate) fn house_cost_factors(
        &self,
        owner: InternedId,
        rules: &RuleSet,
    ) -> Option<HouseCostFactors> {
        let house = self.houses.get(&owner)?;
        Some(HouseCostFactors {
            country: rules.country_cost_mults(self.interner.resolve(house.house_type_id())),
            factory_plant: house.base_projection.factory_plant_factors(),
        })
    }

    /// TechnoType virtual `+0x84` for `owner`'s House ([`RuleSet::cost_of`]).
    pub(crate) fn cost_of(&self, owner: InternedId, object: &ObjectType, rules: &RuleSet) -> i32 {
        rules.cost_of(object, self.house_cost_factors(owner, rules).as_ref())
    }

    pub(super) fn register_house_base_building(&mut self, id: u64, rules: &RuleSet) {
        let Some(entity) = self.substrate.entities.get(id) else {
            return;
        };
        if entity.category != EntityCategory::Structure {
            return;
        }
        let Some(object) = self.object_type(entity.type_ref(), rules) else {
            return;
        };
        let entry = RegisteredBuilding::from_rules(id, object, rules);
        let owner = entity.owner();
        if let Some(house) = self.houses.get_mut(&owner) {
            house.base_projection.register(entry);
        }
    }

    /// `BuildingClass::Unlimbo`'s House+68 append (`0x00441553..0x00441594`),
    /// reached on the same path as its BuildConst append
    /// ([`Simulation::append_live_build_const`]).
    pub(super) fn append_house_base_building(&mut self, id: u64) {
        let Some(owner) = self
            .substrate
            .entities
            .get(id)
            .filter(|entity| entity.category == EntityCategory::Structure)
            .map(|entity| entity.owner())
        else {
            return;
        };
        if let Some(house) = self.houses.get_mut(&owner) {
            house.base_projection.append_membership(id);
        }
    }

    /// `BuildingClass::ChangeOwner`'s moves between the two Houses' lists:
    /// before the Techno owner swap it stable-removes the building from the
    /// old House's FactoryPlant list, recomputes and removes it from House+68
    /// (`0x00448A21..0x00448AB0`); after it, it appends to the new House's
    /// lists (`0x00449155..0x004491D2`).
    pub(super) fn leave_house_base_lists(&mut self, id: u64, owner: InternedId) {
        if let Some(house) = self.houses.get_mut(&owner) {
            house.base_projection.remove_membership(id);
        }
    }

    pub(super) fn join_house_base_lists(&mut self, id: u64, owner: InternedId) {
        if let Some(house) = self.houses.get_mut(&owner) {
            house.base_projection.append_membership(id);
        }
    }

    /// `TechnoClass::ChangeOwner`'s Remove_Tracking on the old House and
    /// Add_Tracking on the new (`0x007015DE`, `0x007015E6`) move the
    /// building's registration with it.
    pub(super) fn move_house_base_tracking(
        &mut self,
        id: u64,
        old_owner: InternedId,
        new_owner: InternedId,
    ) {
        let Some(entry) = self
            .houses
            .get_mut(&old_owner)
            .and_then(|house| house.base_projection.unregister(id))
        else {
            return;
        };
        if let Some(house) = self.houses.get_mut(&new_owner) {
            house.base_projection.register(entry);
        }
    }

    pub(super) fn remove_house_base_membership(&mut self, id: u64) {
        if !self
            .substrate
            .entities
            .get(id)
            .is_some_and(|entity| entity.category == EntityCategory::Structure)
        {
            return;
        }
        // Broadcast visits every House, including historical non-owner lists.
        for house in self.houses.values_mut() {
            house.base_projection.remove_membership(id);
        }
    }

    pub(super) fn release_house_base_tracking(&mut self, id: u64) {
        if let Some(owner) = self.substrate.entities.get(id).map(|entity| entity.owner())
            && let Some(house) = self.houses.get_mut(&owner)
        {
            //43BF34 changes tracking AFTER the earlier expiry/Limbo. No recalc.
            house.base_projection.unregister(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_factory_plant_f32_fold_including_gradual_underflow() {
        let rows: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tools/spatial_oracle/factory_plant_factors.json"
        ))
        .unwrap();
        for row in rows.as_array().unwrap() {
            let count = row["count"].as_u64().unwrap();
            let mut state = HouseBaseState::default();
            let mut bonus = UNIT_FACTORS;
            bonus[1] = NativeF32Bits::from_bits(0.75_f32.to_bits());
            if let Some(bits) = row["bonus_bits"].as_array() {
                for (factor, bits) in bonus.iter_mut().zip(bits) {
                    *factor = NativeF32Bits::from_bits(bits.as_u64().unwrap() as u32);
                }
            }
            for id in 0..=count {
                state.register(RegisteredBuilding {
                    id,
                    tracks_base: true,
                    plant: Some(bonus),
                });
                state.append_membership(id);
            }
            // Exercise the membership receiver used by production Limbo/expiry:
            // removing one plant recomputes the remaining ordered native fold.
            state.remove_membership(count);
            let expected: Vec<u32> = row["factor_bits"]
                .as_array()
                .unwrap()
                .iter()
                .map(|bits| bits.as_u64().unwrap() as u32)
                .collect();
            assert_eq!(
                state.factors.map(NativeF32Bits::bits).as_slice(),
                expected,
                "native FactoryPlant count{count}"
            );
        }
    }

    /// An Industrial Plant discounts its House from Unlimbo on, the discount
    /// moves with a capture and ends at the plant's pointer expiry; the
    /// country's `CostUnitsMult=` multiplies in throughout.
    #[test]
    fn factory_plant_discount_follows_unlimbo_capture_and_expiry() {
        use crate::map::resolved_terrain::test_flat_ground_grid;
        use crate::rules::ini_parser::IniFile;
        use crate::sim::house_state::HouseState;

        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[Countries]\n0=Americans\n1=Russians\n\
             [Russians]\nCostUnitsMult=.5\n\
             [BuildingTypes]\n0=NAINDP\n[VehicleTypes]\n0=HTNK\n\
             [NAINDP]\nStrength=1000\nFoundation=1x1\nFactoryPlant=yes\n\
             UnitsCostBonus=0.75\n\
             [HTNK]\nCost=900\n",
        ))
        .unwrap();
        let mut sim = Simulation::new();
        sim.install_resolved_terrain_for_new_map(test_flat_ground_grid(32));
        let [allies, soviets] = ["Americans", "Russians"].map(|name| sim.interner.intern(name));
        for house in [allies, soviets] {
            sim.houses
                .insert(house, HouseState::new(house, 0, None, false, 0, 10));
        }
        let tank = rules.object("HTNK").unwrap();
        let costs = |sim: &Simulation| {
            (
                sim.cost_of(allies, tank, &rules),
                sim.cost_of(soviets, tank, &rules),
            )
        };
        assert_eq!(costs(&sim), (900, 450));
        let plant = sim
            .spawn_object("NAINDP", "Russians", 5, 5, 0, &rules, &BTreeMap::new())
            .unwrap();
        // ftol(900 * 0.75 * 0.5) = ftol(337.5)
        assert_eq!(costs(&sim), (900, 337));
        sim.change_owner_with_rules(plant, allies, &rules);
        assert_eq!(costs(&sim), (675, 450));
        sim.uninit_with_rules(plant, &rules);
        assert_eq!(costs(&sim), (900, 450));
    }
}
