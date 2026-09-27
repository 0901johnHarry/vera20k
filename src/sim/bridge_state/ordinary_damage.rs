//! Concrete ground-bridge damage57CCF0/57CF60/57D530 and sibling57E7A0/57ED00.
//! Cell writes, Recalc and live occupants are synchronous. Neither first
//! damage nor final collapse calls structural BlowUpBridge47DD70.
//! Native executable corpus: tools/spatial_oracle/bridge_ordinary_damage.
use super::Axis;
use super::ordinary::{OrdinaryBridgeHost, centered, member, offset};
use super::publication::CellCoord;
use super::ramp_repair::Family;

pub(crate) trait OrdinaryDamageHost: OrdinaryBridgeHost {
    fn notify_span(&mut self, first: CellCoord, end: CellCoord) -> Result<(), Self::Error>;
}

pub(crate) fn damage<H: OrdinaryDamageHost>(
    host: &mut H,
    input: CellCoord,
) -> Result<bool, H::Error> {
    let Some((point, ns)) = centered(host, input, Family::High) else {
        return Ok(false);
    };
    root(host, point, ns)
}

fn group<H: OrdinaryBridgeHost>(host: &mut H, point: CellCoord, ns: bool) -> [H::Cell; 3] {
    let across = if ns { (0, 1) } else { (1, 0) };
    [
        host.lookup(point),
        host.lookup(offset(point, -across.0, -across.1)),
        host.lookup(offset(point, across.0, across.1)),
    ]
}

fn recalc_and_occupants<H: OrdinaryBridgeHost>(
    host: &mut H,
    cells: [H::Cell; 3],
    mode: u8,
) -> Result<(), H::Error> {
    for cell in cells {
        host.recalc(cell)?;
    }
    for cell in cells {
        host.occupants(cell, mode)?;
    }
    Ok(())
}

fn root<H: OrdinaryDamageHost>(host: &mut H, point: CellCoord, ns: bool) -> Result<bool, H::Error> {
    let [center, negative, positive] = group(host, point, ns);
    let prior = host.overlay(center);
    let threshold = if ns { 211 } else { 220 };
    let end = if ns { 223 } else { 227 };
    let destroyed = if ns { 231 } else { 232 };
    let (next, before, after, collapsed) = if prior == end {
        (end + 1, ns, !ns, false)
    } else if prior == end + 2 {
        (end + 3, !ns, ns, false)
    } else if prior < threshold {
        (threshold, true, true, false)
    } else if prior < threshold + 3 {
        (destroyed, true, true, true)
    } else {
        return Ok(false);
    };
    // Root57D71B/57CF60 stores both sides before center. The retained
    // identities survive propagation callbacks and their recursive damage.
    for cell in [negative, positive, center] {
        host.write_overlay(cell, next as u8);
    }
    if collapsed {
        for cell in [negative, positive, center] {
            host.radar(host.coord(cell));
        }
    }
    let along = if ns { (1, 0) } else { (0, 1) };
    if before {
        propagate(host, offset(point, -along.0, -along.1), ns)?;
    }
    if after {
        propagate(host, offset(point, along.0, along.1), ns)?;
    }
    if collapsed {
        //57DC20/57DAF0 scan after sibling callbacks. Endpoint membership is
        // live, and the untagged575EE0 traversal still performs cell lookups.
        let first = endpoint(host, point, -along.0, -along.1);
        let last = endpoint(host, point, along.0, along.1);
        host.notify_span(first, last)?;
    }
    host.redraw(center);
    // Concrete NS uses its final-collapse byte; EW always passes1, including
    // first damage. Executed corpus proves this intentional asymmetry.
    recalc_and_occupants(
        host,
        [center, negative, positive],
        u8::from(!ns || collapsed),
    )?;
    if collapsed {
        host.connectivity()?;
        host.rebuild_rectangle([i32::from(point.0) - 1, i32::from(point.1) - 1, 3, 3])?;
    }
    Ok(collapsed)
}

fn endpoint<H: OrdinaryBridgeHost>(host: &mut H, start: CellCoord, dx: i16, dy: i16) -> CellCoord {
    let mut point = start;
    loop {
        let next = offset(point, dx, dy);
        let cell = host.lookup(next);
        if !member(host.overlay(cell), Family::High) {
            return point;
        }
        point = next;
    }
}

fn propagate<H: OrdinaryBridgeHost>(
    host: &mut H,
    point: CellCoord,
    ns: bool,
) -> Result<(), H::Error> {
    let selected = host.lookup(point);
    if !member(host.overlay(selected), Family::High) {
        return Ok(());
    }
    let [center, negative, positive] = group(host, point, ns);
    //57CAB0 reads east then west;57CBE0 reads north then south. Keep lookup
    // order even for a shared dummy. The center is reread after both probes.
    let step = if ns { (1, 0) } else { (0, -1) };
    let first = host.lookup(offset(point, step.0, step.1));
    let first = host.overlay(first);
    let second = host.lookup(offset(point, -step.0, -step.1));
    let second = host.overlay(second);
    let base = if ns { 205 } else { 214 };
    let end = if ns { 223 } else { 227 };
    let destroyed = if ns { 231 } else { 232 };
    let mut index = 0;
    if [base + 4, base + 6, base + 8, end + 1].contains(&first) {
        index |= 1;
    } else if [base + 7, destroyed].contains(&first) {
        index |= 2;
    }
    if [base + 5, base + 6, base + 7, end + 3].contains(&second) {
        index |= 4;
    } else if [base + 8, destroyed].contains(&second) {
        index |= 8;
    }
    if index == 0 {
        return Ok(());
    }
    let prior = host.overlay(center);
    let next = if prior < end {
        let Some(next) = crate::sim::bridge_specs::pick_destruction_overlay(
            index,
            if ns { Axis::NS } else { Axis::EW },
            true,
        ) else {
            return Ok(());
        };
        if i32::from(next) == prior {
            return Ok(());
        }
        next
    } else if prior == end || prior == end + 2 {
        (prior + 1) as u8
    } else {
        return Ok(());
    };
    for cell in [center, negative, positive] {
        host.write_overlay(cell, next);
    }
    host.redraw(center);
    let collapsed = i32::from(next) == destroyed;
    if collapsed {
        let across = if ns { (0, 1) } else { (1, 0) };
        for coord in [
            point,
            offset(point, across.0, across.1),
            offset(point, -across.0, -across.1),
        ] {
            host.radar(coord);
        }
    }
    recalc_and_occupants(host, [center, negative, positive], u8::from(collapsed))
}

#[cfg(test)]
#[path = "ordinary_damage_tests.rs"]
mod tests;
