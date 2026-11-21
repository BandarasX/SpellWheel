use std::f32::consts::{PI, TAU};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wheel {
    Spells,
    QuickItems,
}

impl Wheel {
    pub const ALL: [Self; 2] = [Self::Spells, Self::QuickItems];

    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    Interrupted,
    Gameplay,
    Zoom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Rearm,
    Ready,
    Pending(Instant),
    Held,
    Zoom,
    ZoomReleased(Instant),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Close {
    Released,
    Cancelled,
}

struct Press {
    phase: Phase,
    taps: u8,
}

impl Press {
    const fn new() -> Self {
        Self {
            phase: Phase::Rearm,
            taps: 0,
        }
    }

    fn observe(&mut self, down: bool, now: Instant, delay: Duration, context: Context) {
        self.phase = match (context, self.phase, down) {
            (Context::Interrupted, _, _) => Phase::Rearm,
            (Context::Zoom, _, down) => {
                self.taps = 0;
                if down {
                    Phase::Zoom
                } else {
                    Phase::ZoomReleased(now)
                }
            }
            (Context::Gameplay, Phase::Rearm | Phase::Zoom, true) => self.phase,
            (Context::Gameplay, Phase::Zoom, false) => Phase::ZoomReleased(now),
            (Context::Gameplay, Phase::ZoomReleased(at), false) if at == now => self.phase,
            (Context::Gameplay, Phase::Ready | Phase::ZoomReleased(_), true) if delay.is_zero() => {
                Phase::Held
            }
            (Context::Gameplay, Phase::Ready | Phase::ZoomReleased(_), true) => Phase::Pending(now),
            (Context::Gameplay, Phase::Pending(start), true) if now - start >= delay => Phase::Held,
            (Context::Gameplay, Phase::Pending(start), false) => {
                if now - start < delay {
                    self.taps = self.taps.saturating_add(1);
                }
                Phase::Ready
            }
            (Context::Gameplay, _, false) => Phase::Ready,
            (Context::Gameplay, _, true) => self.phase,
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub index: i32,
    pub param_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisibleWheel {
    pub wheel: Wheel,
    pub generation: u64,
}

struct Session {
    visible: VisibleWheel,
    close: Option<Close>,
    selection: Selection,
}

struct Selection {
    initial: Option<Slot>,
    highlighted: Option<Slot>,
    applied: Option<(Slot, bool)>,
}

impl Selection {
    fn update(
        &mut self,
        close: Option<Close>,
        instant: bool,
        mut slot_access: impl FnMut(Option<Slot>) -> Option<Slot>,
    ) -> bool {
        if close == Some(Close::Cancelled) {
            return false;
        }
        let Some(slot) = self.highlighted else {
            return false;
        };
        if (instant || close == Some(Close::Released))
            && self.applied.map(|(slot, _)| slot) != Some(slot)
        {
            self.applied = Some((slot, slot_access(Some(slot)) == Some(slot)));
        }
        close == Some(Close::Released)
            && self.applied == Some((slot, true))
            && self.initial != Some(slot)
            && slot_access(None) == Some(slot)
    }
}

pub struct ControllerState {
    presses: [Press; 2],
    bindings: [u32; 2],
    session: Option<Session>,
    generation: u64,
}

impl ControllerState {
    pub const fn new() -> Self {
        Self {
            presses: [const { Press::new() }; 2],
            bindings: [0; 2],
            session: None,
            generation: 0,
        }
    }

    pub fn set_bindings(&mut self, bindings: [u32; 2]) {
        for wheel in Wheel::ALL {
            if bindings[wheel.index()] != self.bindings[wheel.index()] {
                self.interrupt(wheel);
            }
        }
        self.bindings = bindings;
    }

    pub fn observe_buttons(
        &mut self,
        sample: Option<(u32, Instant)>,
        delay: Duration,
        context: Context,
    ) {
        for wheel in Wheel::ALL {
            let binding = self.bindings[wheel.index()];
            if let Some((buttons, at)) = sample
                && binding != 0
                && context != Context::Interrupted
            {
                self.observe(wheel, buttons & binding != 0, at, delay, context);
            } else {
                self.interrupt(wheel);
            }
        }
    }

    fn interrupt(&mut self, wheel: Wheel) {
        self.presses[wheel.index()] = Press::new();
        if let Some(session) = &mut self.session
            && session.visible.wheel == wheel
        {
            session.close = Some(Close::Cancelled);
        }
    }

    fn observe(
        &mut self,
        wheel: Wheel,
        down: bool,
        now: Instant,
        delay: Duration,
        context: Context,
    ) {
        if let Some(session) = &mut self.session
            && session.visible.wheel == wheel
        {
            if context != Context::Gameplay {
                session.close = Some(Close::Cancelled);
            } else if !down {
                session.close.get_or_insert(Close::Released);
            }
        }
        self.presses[wheel.index()].observe(down, now, delay, context);
    }

    pub fn zoom_owns(&self, wheel: Wheel) -> bool {
        matches!(
            self.presses[wheel.index()].phase,
            Phase::Zoom | Phase::ZoomReleased(_)
        )
    }

    pub fn take_tap(&mut self, wheel: Wheel) -> bool {
        let taps = &mut self.presses[wheel.index()].taps;
        let pending = *taps != 0;
        *taps = taps.saturating_sub(1);
        pending
    }

    pub fn open(
        &mut self,
        wheel: Wheel,
        active: impl FnOnce() -> Option<Slot>,
    ) -> Option<VisibleWheel> {
        if self.session.is_some() || self.presses[wheel.index()].phase != Phase::Held {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        let visible = VisibleWheel {
            wheel,
            generation: self.generation,
        };
        self.session = Some(Session {
            visible,
            close: None,
            selection: Selection {
                initial: active(),
                highlighted: None,
                applied: None,
            },
        });
        Some(visible)
    }

    pub fn visible(&self) -> Option<VisibleWheel> {
        self.session
            .as_ref()
            .filter(|session| session.close.is_none())
            .map(|session| session.visible)
    }

    pub fn highlight(&mut self, visible: VisibleWheel, slot: Slot) {
        if let Some(session) = &mut self.session
            && session.visible == visible
            && session.close.is_none()
        {
            session.selection.highlighted = Some(slot);
        }
    }

    pub fn update_selection(
        &mut self,
        instant: bool,
        mut slot_access: impl FnMut(Wheel, Option<Slot>) -> Option<Slot>,
    ) -> Option<Wheel> {
        let session = self.session.as_mut()?;
        let wheel = session.visible.wheel;
        let sound = session
            .selection
            .update(session.close, instant, |slot| slot_access(wheel, slot));
        if session.close.is_some() {
            self.session = None;
        }
        sound.then_some(wheel)
    }
}

// A slot keeps the highlight for an extra 8% of its angular width on each side.
pub fn retains_highlight(angle: f32, center: f32, slot_count: usize) -> bool {
    let distance = ((angle - center + PI).rem_euclid(TAU) - PI).abs();
    distance <= (0.5 + 0.08) * TAU / slot_count as f32
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TapReplay([u8; 2]);

impl TapReplay {
    pub fn new([spell, item]: [bool; 2], spell_hud_visible: bool) -> Self {
        // The spell HUD can consult its request twice in one update, for HUD
        // visibility and cycling. Item cycling uses a single check.
        Self([
            u8::from(spell) * (1 + u8::from(spell_hud_visible)),
            u8::from(item),
        ])
    }

    pub fn request(&mut self, wheel: Wheel) -> bool {
        let remaining = &mut self.0[wheel.index()];
        let requested = *remaining != 0;
        *remaining = remaining.saturating_sub(1);
        requested
    }
}

#[cfg(test)]
#[path = "controller_state/tests.rs"]
mod tests;
