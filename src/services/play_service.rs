use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};

use crate::error::Result;

use super::player::{decode::decode, Command, PlayState};

pub struct PlayService {
    cmd_tx: Sender<Command>,
    state_rx: Receiver<PlayState>,
    stopped: bool,
}

impl Drop for PlayService {
    fn drop(&mut self) {
        log::info!("PlayService Dropped");
    }
}

unsafe impl Send for PlayService {}
unsafe impl Sync for PlayService {}

impl PlayService {
    pub fn create(filename: String) -> Result<Self> {
        let (cmd_tx, cmd_rx) = bounded::<Command>(2);
        let (state_tx, state_rx) = bounded::<PlayState>(2);
        decode(filename, cmd_rx, state_tx)?;
        let this = Self {
            cmd_tx,
            state_rx,
            stopped: false,
        };
        Ok(this)
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
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
                self.stopped = true;
                None
            }
        }
    }
}
