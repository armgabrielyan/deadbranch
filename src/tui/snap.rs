//! Snap animation orchestration: Flash → Dissolve → Settle → Done.

use std::time::{Duration, Instant};

use ratatui::style::Color;

use super::particles::ParticleSystem;

/// Animation phase constants.
const FLASH_DURATION: Duration = Duration::from_millis(200);
const SETTLE_DURATION: Duration = Duration::from_millis(500);
/// How long a single row takes to fully dissolve.
const ROW_DISSOLVE_SECS: f32 = 1.0;
/// Number of new cells that begin dissolving per tick (per row).
const CELLS_PER_TICK: usize = 3;
/// Chance (0.0–1.0) that a scattering cell spawns a free-floating particle.
const SPAWN_CHANCE: f32 = 0.4;

/// Current phase of the snap animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapPhase {
    Flash,
    Dissolve,
    Settle,
    Done,
}

/// State of a single character cell during dissolution.
#[derive(Debug, Clone)]
pub enum CellState {
    Normal {
        ch: char,
        color: Color,
    },
    Flickering {
        original: char,
        color: Color,
        frames_left: u8,
    },
    Scattering {
        density: u8,
        color: Color,
    },
    Empty,
}

impl CellState {
    pub fn is_empty(&self) -> bool {
        matches!(self, CellState::Empty)
    }

    pub fn render(&self) -> Option<(char, Color)> {
        match self {
            CellState::Normal { ch, color } => Some((*ch, *color)),
            CellState::Flickering {
                original,
                color,
                frames_left,
            } => {
                if frames_left % 2 == 0 {
                    let particles = ['░', '▒', '▓'];
                    let ch = particles[fastrand::usize(0..particles.len())];
                    Some((ch, Color::DarkGray))
                } else {
                    Some((*original, *color))
                }
            }
            CellState::Scattering { density, color } => {
                let chars = ['·', '░', '▒', '▓', '█'];
                if (*density as usize) < chars.len() {
                    Some((chars[*density as usize], *color))
                } else {
                    None
                }
            }
            CellState::Empty => None,
        }
    }

    pub fn tick(&mut self) -> bool {
        match self {
            CellState::Normal { .. } => false,
            CellState::Flickering {
                original: _,
                color,
                frames_left,
            } => {
                if *frames_left == 0 {
                    let color = *color;
                    *self = CellState::Scattering { density: 3, color };
                    true
                } else {
                    *frames_left -= 1;
                    false
                }
            }
            CellState::Scattering { density, .. } => {
                if *density == 0 {
                    *self = CellState::Empty;
                } else {
                    *density -= 1;
                }
                false
            }
            CellState::Empty => false,
        }
    }
}

pub struct RowDissolve {
    pub branch_index: usize,
    pub start_delay: Duration,
    pub cell_states: Vec<CellState>,
    dissolve_order: Vec<usize>,
    next_dissolve: usize,
    started: bool,
    /// Actual screen Y coordinate (populated during first render)
    pub screen_y: u16,
    /// Actual screen X coordinate per cell (populated during first render)
    pub x_positions: Vec<u16>,
}

impl RowDissolve {
    pub fn new(branch_index: usize, start_delay: Duration, cells: Vec<(char, Color)>) -> Self {
        let cell_states: Vec<CellState> = cells
            .into_iter()
            .map(|(ch, color)| CellState::Normal { ch, color })
            .collect();

        let len = cell_states.len();
        let mut order: Vec<usize> = (0..len).collect();
        order.sort_by_cached_key(|&i| {
            let bias = (i as f32 / len.max(1) as f32 * 50.0) as i32;
            (fastrand::i32(0..100)) - bias
        });

        Self {
            branch_index,
            start_delay,
            cell_states,
            dissolve_order: order,
            next_dissolve: 0,
            started: false,
            screen_y: 0,
            x_positions: Vec::new(),
        }
    }

    pub fn is_fully_dissolved(&self) -> bool {
        self.cell_states.iter().all(|c| c.is_empty())
    }

    pub fn tick(&mut self, elapsed: Duration) -> Vec<(usize, Color)> {
        if elapsed < self.start_delay {
            return Vec::new();
        }

        self.started = true;
        let mut spawn_positions = Vec::new();

        let remaining = self.dissolve_order.len() - self.next_dissolve;
        let to_start = CELLS_PER_TICK.min(remaining);
        for _ in 0..to_start {
            let cell_idx = self.dissolve_order[self.next_dissolve];
            self.next_dissolve += 1;
            if let CellState::Normal { ch, color } = self.cell_states[cell_idx] {
                let frames = 3 + fastrand::u8(0..3);
                self.cell_states[cell_idx] = CellState::Flickering {
                    original: ch,
                    color,
                    frames_left: frames,
                };
            }
        }

        for (i, cell) in self.cell_states.iter_mut().enumerate() {
            let became_scattering = cell.tick();
            if became_scattering && fastrand::f32() < SPAWN_CHANCE {
                if let CellState::Scattering { color, .. } = cell {
                    spawn_positions.push((i, *color));
                }
            }
        }

        spawn_positions
    }

    /// Reinitialize this row's cells from actual buffer content.
    /// Called once after the first render to capture real screen positions.
    pub fn capture_from_screen(&mut self, screen_y: u16, cells: Vec<(u16, char, Color)>) {
        self.screen_y = screen_y;
        self.x_positions = cells.iter().map(|&(x, _, _)| x).collect();
        self.cell_states = cells
            .iter()
            .map(|&(_, ch, color)| CellState::Normal { ch, color })
            .collect();

        // Rebuild dissolve order with left-to-right bias
        let len = self.cell_states.len();
        let mut order: Vec<usize> = (0..len).collect();
        order.sort_by_cached_key(|&i| {
            let bias = (i as f32 / len.max(1) as f32 * 50.0) as i32;
            (fastrand::i32(0..100)) - bias
        });
        self.dissolve_order = order;
        self.next_dissolve = 0;
    }

    #[cfg(test)]
    pub fn has_started(&self) -> bool {
        self.started
    }
}

pub struct SnapAnimation {
    start: Instant,
    pub phase: SnapPhase,
    pub rows: Vec<RowDissolve>,
    pub particles: ParticleSystem,
    settle_start: Option<Instant>,
    /// Whether screen positions have been captured from the buffer
    pub captured: bool,
}

impl SnapAnimation {
    pub fn new(branch_cells: Vec<(usize, Vec<(char, Color)>)>) -> Self {
        let count = branch_cells.len();
        let total_window = 2.5_f32;
        let stagger = if count > 1 {
            (total_window - ROW_DISSOLVE_SECS) / (count - 1) as f32
        } else {
            0.0
        };

        let rows = branch_cells
            .into_iter()
            .enumerate()
            .map(|(i, (branch_index, cells))| {
                let delay = Duration::from_secs_f32(stagger * i as f32);
                RowDissolve::new(branch_index, delay, cells)
            })
            .collect();

        Self {
            start: Instant::now(),
            phase: SnapPhase::Flash,
            rows,
            particles: ParticleSystem::new(),
            settle_start: None,
            captured: false,
        }
    }

    /// Advance the animation by one frame.
    ///
    /// `can_finish` gates the Settle → Done transition: the animation will
    /// stay in the Settle phase (keeping particles alive) until the caller
    /// signals that background work (e.g. branch deletions) is complete.
    pub fn tick(&mut self, screen_width: u16, screen_height: u16, can_finish: bool) {
        let elapsed = self.start.elapsed();

        match self.phase {
            SnapPhase::Flash => {
                if elapsed >= FLASH_DURATION {
                    self.phase = SnapPhase::Dissolve;
                }
            }
            SnapPhase::Dissolve => {
                let dissolve_elapsed = elapsed - FLASH_DURATION;

                for row in self.rows.iter_mut() {
                    let spawn_positions = row.tick(dissolve_elapsed);
                    for (cell_idx, color) in spawn_positions {
                        let x = row.x_positions.get(cell_idx).copied().unwrap_or(0);
                        self.particles.spawn(x as f32, row.screen_y as f32, color);
                    }
                }

                self.particles.tick(screen_width, screen_height);

                if self.rows.iter().all(|r| r.is_fully_dissolved()) {
                    self.phase = SnapPhase::Settle;
                    self.settle_start = Some(Instant::now());
                }
            }
            SnapPhase::Settle => {
                self.particles.tick(screen_width, screen_height);

                let settle_elapsed = self
                    .settle_start
                    .map(|s| s.elapsed())
                    .unwrap_or(Duration::ZERO);

                if can_finish && (self.particles.is_empty() || settle_elapsed >= SETTLE_DURATION) {
                    self.phase = SnapPhase::Done;
                }
            }
            SnapPhase::Done => {}
        }
    }

    pub fn is_done(&self) -> bool {
        self.phase == SnapPhase::Done
    }

    #[cfg(test)]
    pub fn dissolved_branch_indices(&self) -> Vec<usize> {
        self.rows
            .iter()
            .filter(|r| r.is_fully_dissolved())
            .map(|r| r.branch_index)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cells() -> Vec<(char, Color)> {
        "feature/old-branch"
            .chars()
            .map(|ch| (ch, Color::White))
            .collect()
    }

    #[test]
    fn test_cell_state_normal_renders_original() {
        let cs = CellState::Normal {
            ch: 'A',
            color: Color::Green,
        };
        assert_eq!(cs.render(), Some(('A', Color::Green)));
        assert!(!cs.is_empty());
    }

    #[test]
    fn test_cell_state_empty_renders_none() {
        let cs = CellState::Empty;
        assert_eq!(cs.render(), None);
        assert!(cs.is_empty());
    }

    #[test]
    fn test_cell_state_flickering_tick_transitions_to_scattering() {
        let mut cs = CellState::Flickering {
            original: 'X',
            color: Color::Cyan,
            frames_left: 0,
        };
        let became_scattering = cs.tick();
        assert!(became_scattering);
        assert!(matches!(cs, CellState::Scattering { .. }));
    }

    #[test]
    fn test_cell_state_flickering_decrements_frames() {
        let mut cs = CellState::Flickering {
            original: 'X',
            color: Color::Cyan,
            frames_left: 3,
        };
        let became_scattering = cs.tick();
        assert!(!became_scattering);
        if let CellState::Flickering { frames_left, .. } = cs {
            assert_eq!(frames_left, 2);
        } else {
            panic!("expected Flickering");
        }
    }

    #[test]
    fn test_cell_state_scattering_decays_to_empty() {
        let mut cs = CellState::Scattering {
            density: 1,
            color: Color::White,
        };
        cs.tick();
        assert!(matches!(cs, CellState::Scattering { density: 0, .. }));
        cs.tick();
        assert!(cs.is_empty());
    }

    #[test]
    fn test_cell_state_full_lifecycle() {
        let mut cs = CellState::Flickering {
            original: 'Z',
            color: Color::Red,
            frames_left: 1,
        };
        cs.tick(); // frames_left 1 → 0
        cs.tick(); // 0 → Scattering{density:3}
        assert!(matches!(cs, CellState::Scattering { density: 3, .. }));
        cs.tick(); // 3 → 2
        cs.tick(); // 2 → 1
        cs.tick(); // 1 → 0
        cs.tick(); // 0 → Empty
        assert!(cs.is_empty());
    }

    #[test]
    fn test_row_dissolve_new_creates_all_normal() {
        let rd = RowDissolve::new(0, Duration::ZERO, sample_cells());
        assert!(rd
            .cell_states
            .iter()
            .all(|c| matches!(c, CellState::Normal { .. })));
        assert!(!rd.is_fully_dissolved());
        assert!(!rd.has_started());
    }

    #[test]
    fn test_row_dissolve_respects_start_delay() {
        let mut rd = RowDissolve::new(0, Duration::from_millis(500), sample_cells());
        let spawns = rd.tick(Duration::from_millis(100));
        assert!(spawns.is_empty());
        assert!(!rd.has_started());
    }

    #[test]
    fn test_row_dissolve_starts_after_delay() {
        let mut rd = RowDissolve::new(0, Duration::from_millis(100), sample_cells());
        let _spawns = rd.tick(Duration::from_millis(200));
        assert!(rd.has_started());
        let non_normal = rd
            .cell_states
            .iter()
            .filter(|c| !matches!(c, CellState::Normal { .. }))
            .count();
        assert!(non_normal > 0);
    }

    #[test]
    fn test_row_dissolve_fully_dissolves() {
        let cells: Vec<(char, Color)> = "ab".chars().map(|c| (c, Color::White)).collect();
        let mut rd = RowDissolve::new(0, Duration::ZERO, cells);
        for i in 0..200 {
            rd.tick(Duration::from_millis(i * 33));
            if rd.is_fully_dissolved() {
                break;
            }
        }
        assert!(rd.is_fully_dissolved());
    }

    #[test]
    fn test_snap_animation_starts_in_flash() {
        let anim = SnapAnimation::new(vec![(0, sample_cells())]);
        assert_eq!(anim.phase, SnapPhase::Flash);
        assert!(!anim.is_done());
    }

    #[test]
    fn test_snap_animation_phase_progression() {
        let cells: Vec<(char, Color)> = "ab".chars().map(|c| (c, Color::White)).collect();
        let mut anim = SnapAnimation::new(vec![(0, cells)]);

        // Flash phase requires real wall-clock time (200ms)
        std::thread::sleep(Duration::from_millis(250));
        anim.tick(80, 24, true);
        assert_eq!(anim.phase, SnapPhase::Dissolve);

        // Dissolve: tick many times; rows are short ("ab") so dissolve quickly
        for _ in 0..500 {
            anim.tick(80, 24, true);
            if anim.phase != SnapPhase::Dissolve {
                break;
            }
        }
        assert!(
            anim.phase == SnapPhase::Settle || anim.phase == SnapPhase::Done,
            "expected Settle or Done, got {:?}",
            anim.phase
        );

        // Settle requires 500ms or particles to clear
        std::thread::sleep(Duration::from_millis(550));
        for _ in 0..100 {
            anim.tick(80, 24, true);
            if anim.is_done() {
                break;
            }
        }
        assert!(anim.is_done());
    }

    #[test]
    fn test_snap_animation_stagger_ordering() {
        let cells = || sample_cells();
        let anim = SnapAnimation::new(vec![(0, cells()), (1, cells()), (2, cells())]);
        assert!(anim.rows[0].start_delay <= anim.rows[1].start_delay);
        assert!(anim.rows[1].start_delay <= anim.rows[2].start_delay);
    }

    #[test]
    fn test_dissolved_branch_indices() {
        let cells: Vec<(char, Color)> = "a".chars().map(|c| (c, Color::White)).collect();
        let mut anim = SnapAnimation::new(vec![(5, cells.clone()), (9, cells)]);

        // Wait past Flash (200ms) + stagger window (1.5s for 2nd row) + margin
        std::thread::sleep(Duration::from_millis(2000));

        for _ in 0..1000 {
            anim.tick(80, 24, true);
            if anim.is_done() {
                break;
            }
        }

        let dissolved = anim.dissolved_branch_indices();
        assert!(dissolved.contains(&5));
        assert!(dissolved.contains(&9));
    }
}
