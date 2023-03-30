use std::io::{ErrorKind, Read, Write};

use crate::{
    adb_termios::ADBTermios,
    models::{AdbCommand, HostFeatures},
    AdbTcpConnexion, Result, RustADBError,
};


impl AdbTcpConnexion {
    pub fn sync<S: ToString + Clone>(
        &mut self,
        serial: Option<S>,
        command: impl IntoIterator<Item = S>,
    ) -> Result<()> {
    }
