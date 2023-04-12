use crate::{
    models::{AdbCommand, SyncCommand},
    AdbTcpConnexion, Result,
};

impl AdbTcpConnexion {
    /// Pushes
    pub fn push_command<S: ToString>(
        &mut self,
        serial: Option<S>,
        _filename: S,
        _path: S,
    ) -> Result<()> {
        self.new_connection()?;

        match serial {
            None => Self::send_adb_request(&mut self.tcp_stream, AdbCommand::TransportAny)?,
            Some(serial) => Self::send_adb_request(
                &mut self.tcp_stream,
                AdbCommand::TransportSerial(serial.to_string()),
            )?,
        }

        // Set device in SYNC mode
        Self::send_adb_request(&mut self.tcp_stream, AdbCommand::Sync)?;

        // Send a list command
        Self::send_sync_request(&mut self.tcp_stream, SyncCommand::List("/data/"))?;

        Ok(())
    }
}
