pub mod event_system;
pub mod play_system;
pub mod setup_system;
pub mod ui_system;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GameState {
    Playing,
    Terminal,
    Restart,
}
