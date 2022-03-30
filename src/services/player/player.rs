use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};

use crate::error::Result;

use super::{play::play, Command, PlayState};

pub struct Player {
    cmd_tx: Sender<Command>,
    state_rx: Receiver<PlayState>,
    abort_request: Arc<AtomicBool>,
}

impl Drop for Player {
    fn drop(&mut self) {
        log::info!("Player Dropped");
    }
}

unsafe impl Send for Player {}
unsafe impl Sync for Player {}

impl Default for Player {
    fn default() -> Self {
        let (cmd_tx, _cmd_rx) = bounded::<Command>(2);
        let (_state_tx, state_rx) = bounded::<PlayState>(1);
        let abort_request = Arc::new(AtomicBool::new(false));
        Self {
            cmd_tx,
            state_rx,
            abort_request,
        }
    }
}
impl Player {
    pub fn play(&mut self, file: impl Into<String>) -> Result<()> {
        let (cmd_tx, cmd_rx) = bounded::<Command>(2);
        let (state_tx, state_rx) = bounded::<PlayState>(1);

        self.cmd_tx = cmd_tx;
        self.state_rx = state_rx;
        self.abort_request = Arc::new(AtomicBool::new(false));

        play(file.into(), cmd_rx, state_tx, self.abort_request.clone())?;
        Ok(())
    }

    pub fn play_finished(&self) -> bool {
        self.abort_request.load(Ordering::Relaxed)
    }

    pub fn set_play_finished(&self) {
        self.abort_request.store(true, Ordering::Relaxed);
    }

    pub fn set_pause(&self, pause: bool) {
        log::info!("play service pause");
        if let Err(e) = self.cmd_tx.try_send(Command::Pause(pause)) {
            log::error!("发送 Command::Pause 失败, E: {}", e.to_string());
        }
    }

    pub fn set_mute(&self, mute: bool) {
        log::info!("play service pause");
        if let Err(e) = self.cmd_tx.try_send(Command::Mute(mute)) {
            log::error!("发送 Command::Pause 失败, E: {}", e.to_string());
        }
    }

    pub fn set_volume(&self, volume: f32) {
        log::info!("play service set volume: {volume}");
        if let Err(e) = self.cmd_tx.try_send(Command::Volume(volume)) {
            log::error!(
                "try_send cmd Volume({}) failed, E: {}",
                volume,
                e.to_string()
            );
        }
    }

    pub fn try_recv_state(&mut self) -> Option<PlayState> {
        match self.state_rx.try_recv() {
            Ok(state) => Some(state),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                log::info!("player state Disconnected");
                // self.set_abort_request(true);
                None
            }
        }
    }
}
