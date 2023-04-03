use std::io::{ErrorKind, Read, Write};

use crate::{
    adb_termios::ADBTermios,
    models::{AdbCommand, HostFeatures},
    AdbTcpConnexion, Result, RustADBError,
};

impl AdbTcpConnexion {
    /// Pushes
    pub fn sync<S: ToString>(&mut self, serial: Option<S>, filename: S, path: S) -> Result<()> {}
    ) -> Result<()> {
    }
}
