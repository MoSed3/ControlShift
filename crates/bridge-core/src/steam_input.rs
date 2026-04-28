use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use bridge_protocol::{ControllerId, ControllerState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerSnapshot {
    pub id: ControllerId,
    pub label: String,
    pub state: ControllerState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerEvent {
    Connected { id: ControllerId, label: String },
    State(ControllerSnapshot),
    Disconnected { id: ControllerId },
}

pub trait InputSource: Send + 'static {
    fn poll(&mut self) -> Vec<ControllerSnapshot>;
}

#[derive(Debug)]
pub struct SteamPoller<S> {
    source: S,
    excluded: HashSet<ControllerId>,
    tick_rate: Duration,
}

impl<S> SteamPoller<S> {
    pub fn new(source: S) -> Self {
        Self {
            source,
            excluded: HashSet::new(),
            tick_rate: Duration::from_millis(16),
        }
    }

    pub fn with_tick_rate(mut self, tick_rate: Duration) -> Self {
        self.tick_rate = tick_rate;
        self
    }

    pub fn with_excluded<I>(mut self, excluded: I) -> Self
    where
        I: IntoIterator<Item = ControllerId>,
    {
        self.excluded = excluded.into_iter().collect();
        self
    }
}

impl<S> SteamPoller<S>
where
    S: InputSource,
{
    pub fn spawn(self, tx: Sender<ControllerEvent>) -> JoinHandle<()> {
        thread::Builder::new()
            .name("steam-input-poller".to_string())
            .spawn(move || {
                let mut poller = self;
                poller.run_until_channel_closes(tx);
            })
            .expect("steam input poller thread should spawn")
    }

    pub fn run_for_ticks(&mut self, tx: &Sender<ControllerEvent>, ticks: usize) {
        let mut known = HashMap::new();
        for _ in 0..ticks {
            self.tick(tx, &mut known);
        }
    }

    fn run_until_channel_closes(&mut self, tx: Sender<ControllerEvent>) {
        let mut known = HashMap::new();
        loop {
            let started = Instant::now();
            if !self.tick(&tx, &mut known) {
                break;
            }

            let elapsed = started.elapsed();
            if elapsed < self.tick_rate {
                thread::sleep(self.tick_rate - elapsed);
            }
        }
    }

    fn tick(
        &mut self,
        tx: &Sender<ControllerEvent>,
        known: &mut HashMap<ControllerId, String>,
    ) -> bool {
        let snapshots = self
            .source
            .poll()
            .into_iter()
            .filter(|snapshot| !self.excluded.contains(&snapshot.id))
            .collect::<Vec<_>>();

        let current = snapshots
            .iter()
            .map(|snapshot| (snapshot.id, snapshot.label.clone()))
            .collect::<HashMap<_, _>>();

        for snapshot in &snapshots {
            if !known.contains_key(&snapshot.id)
                && tx
                    .send(ControllerEvent::Connected {
                        id: snapshot.id,
                        label: snapshot.label.clone(),
                    })
                    .is_err()
            {
                return false;
            }

            if tx.send(ControllerEvent::State(snapshot.clone())).is_err() {
                return false;
            }
        }

        let disconnected = known
            .keys()
            .filter(|id| !current.contains_key(id))
            .copied()
            .collect::<Vec<_>>();

        for id in disconnected {
            if tx.send(ControllerEvent::Disconnected { id }).is_err() {
                return false;
            }
        }

        *known = current;
        true
    }
}

#[derive(Debug, Clone)]
pub struct FakeSteamInput {
    frames: Vec<Vec<ControllerSnapshot>>,
    index: usize,
}

impl FakeSteamInput {
    pub fn new(frames: Vec<Vec<ControllerSnapshot>>) -> Self {
        Self { frames, index: 0 }
    }
}

impl InputSource for FakeSteamInput {
    fn poll(&mut self) -> Vec<ControllerSnapshot> {
        if self.frames.is_empty() {
            return Vec::new();
        }

        let frame = self
            .frames
            .get(self.index)
            .cloned()
            .unwrap_or_else(|| self.frames.last().cloned().unwrap_or_default());

        self.index = self.index.saturating_add(1);
        frame
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use super::*;
    use bridge_protocol::Button;

    fn snapshot(id: u64, label: &str, button: Option<Button>) -> ControllerSnapshot {
        let mut state = ControllerState::default();
        if let Some(button) = button {
            state.set_pressed(button, true);
        }

        ControllerSnapshot {
            id: ControllerId(id),
            label: label.to_string(),
            state,
        }
    }

    #[test]
    fn poller_emits_connect_state_and_disconnect_events() {
        let frames = vec![
            vec![snapshot(1, "Stadia", Some(Button::A))],
            vec![snapshot(1, "Stadia", Some(Button::B))],
            vec![],
        ];
        let mut poller = SteamPoller::new(FakeSteamInput::new(frames));
        let (tx, rx) = channel();

        poller.run_for_ticks(&tx, 3);
        drop(tx);

        let events = rx.iter().collect::<Vec<_>>();

        assert_eq!(
            events,
            vec![
                ControllerEvent::Connected {
                    id: ControllerId(1),
                    label: "Stadia".to_string(),
                },
                ControllerEvent::State(snapshot(1, "Stadia", Some(Button::A))),
                ControllerEvent::State(snapshot(1, "Stadia", Some(Button::B))),
                ControllerEvent::Disconnected {
                    id: ControllerId(1),
                },
            ]
        );
    }

    #[test]
    fn poller_skips_excluded_controllers() {
        let frames = vec![vec![
            snapshot(1, "Stadia", Some(Button::A)),
            snapshot(2, "DualSense", Some(Button::B)),
        ]];
        let mut poller =
            SteamPoller::new(FakeSteamInput::new(frames)).with_excluded([ControllerId(2)]);
        let (tx, rx) = channel();

        poller.run_for_ticks(&tx, 1);
        drop(tx);

        let events = rx.iter().collect::<Vec<_>>();

        assert_eq!(
            events,
            vec![
                ControllerEvent::Connected {
                    id: ControllerId(1),
                    label: "Stadia".to_string(),
                },
                ControllerEvent::State(snapshot(1, "Stadia", Some(Button::A))),
            ]
        );
    }
}
