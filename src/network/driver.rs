use crate::protocol::commands::Command;

pub trait NetworkDriver {
    fn send_command(&mut self, cmd: Command);
    fn poll_commands(&mut self) -> Vec<Command>;
}

/// Локальный драйвер для одиночной игры: команды применяются мгновенно без сокетов
pub struct LocalDriver {
    inbox: Vec<Command>,
}

impl LocalDriver {
    pub fn new() -> Self {
        Self { inbox: Vec::new() }
    }
}

impl NetworkDriver for LocalDriver {
    fn send_command(&mut self, cmd: Command) {
        self.inbox.push(cmd);
    }

    fn poll_commands(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.inbox)
    }
}
