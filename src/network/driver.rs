use crate::protocol::commands::Command;
use crate::ecs::world::World;
use crate::network::snapshot::WorldSnapshot;

pub trait NetworkDriver {
    fn send_command(&mut self, cmd: Command);
    fn poll_commands(&mut self) -> Vec<Command>;
    fn is_network_client(&self) -> bool { false }
    fn is_network_host(&self) -> bool { false }
    fn is_networked(&self) -> bool { false }
    fn is_network_connected(&self) -> bool { false }
    fn assigned_player_id(&self) -> Option<u8> { None }
    fn take_snapshot(&mut self) -> Option<WorldSnapshot> { None }
    fn publish_snapshot(&mut self, _world: &World) {}
    fn rejoin_commands(&self) -> Vec<Command> { Vec::new() }
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
