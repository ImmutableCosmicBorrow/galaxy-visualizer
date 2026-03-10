use crossbeam_channel::{Receiver, Sender};
use orchestrator::ui::{OrchestratorToUiUpdate, UiToOrchestratorCommand};

/// Wraps the two-way channel between the UI and the orchestrator thread.
pub struct OrchestratorComms {
    pub cmd_sender: Sender<UiToOrchestratorCommand>,
    pub update_receiver: Receiver<OrchestratorToUiUpdate>,
}

impl OrchestratorComms {
    pub fn new(
        cmd_sender: Sender<UiToOrchestratorCommand>,
        update_receiver: Receiver<OrchestratorToUiUpdate>,
    ) -> Self {
        Self {
            cmd_sender,
            update_receiver,
        }
    }

    /// Convenience: send a command, ignoring errors.
    pub fn send(&self, cmd: UiToOrchestratorCommand) {
        let _ = self.cmd_sender.send(cmd);
    }

    /// Convenience: send a command, panicking on error.
    pub fn send_expect(&self, cmd: UiToOrchestratorCommand, msg: &str) {
        self.cmd_sender.send(cmd).expect(msg);
    }
}
