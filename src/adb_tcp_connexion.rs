use byteorder::{ByteOrder, LittleEndian};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
    str,
    str::FromStr,
};

use crate::{
    models::{AdbCommand, AdbRequestStatus, SyncCommand},
    Result, RustADBError,
};

/// Represents an ADB-over-TCP connexion.
#[derive(Debug)]
pub struct AdbTcpConnexion {
    pub(crate) socket_addr: SocketAddrV4,
    pub(crate) tcp_stream: TcpStream,
}

impl AdbTcpConnexion {
    /// Instantiates a new instance of [AdbTcpConnexion]
    pub fn new(address: Ipv4Addr, port: u16) -> Result<Self> {
        let addr = SocketAddrV4::new(address, port);
        Ok(Self {
            socket_addr: addr,
            tcp_stream: TcpStream::connect(addr)?,
        })
    }

    /// Creates a new connection to ADB server.
    ///
    /// Can be used after requests that closes connection.
    pub(crate) fn new_connection(&mut self) -> Result<()> {
        self.tcp_stream = TcpStream::connect(self.socket_addr)?;

        Ok(())
    }

    pub(crate) fn proxy_connexion(
        &mut self,
        adb_command: AdbCommand,
        with_response: bool,
    ) -> Result<Vec<u8>> {
        Self::send_adb_request(&mut self.tcp_stream, adb_command)?;

        if with_response {
            let length = Self::get_body_length(&mut self.tcp_stream)?;
            let mut body = vec![
                0;
                length
                    .try_into()
                    .map_err(|_| RustADBError::ConvertionError)?
            ];
            if length > 0 {
                self.tcp_stream.read_exact(&mut body)?;
            }

            Ok(body)
        } else {
            Ok(vec![])
        }
    }

    /// Sends the given [AdbCommand] to ADB server, and checks that the request has been taken in consideration.
    /// If an error occured, a [RustADBError] is returned with the response error string.
    pub(crate) fn send_adb_request(tcp_stream: &mut TcpStream, command: AdbCommand) -> Result<()> {
        let adb_command_string = command.to_string();
        let adb_request = format!("{:04x}{}", adb_command_string.len(), adb_command_string);

        tcp_stream.write_all(adb_request.as_bytes())?;

        // Reads returned status code from ADB server
        let mut request_status = [0; 4];
        tcp_stream.read_exact(&mut request_status)?;

        match AdbRequestStatus::from_str(str::from_utf8(request_status.as_ref())?)? {
            AdbRequestStatus::Fail => {
                // We can keep reading to get further details
                let length = Self::get_body_length(tcp_stream)?;

                let mut body = vec![
                    0;
                    length
                        .try_into()
                        .map_err(|_| RustADBError::ConvertionError)?
                ];
                if length > 0 {
                    tcp_stream.read_exact(&mut body)?;
                }

                Err(RustADBError::ADBRequestFailed(String::from_utf8(body)?))
            }
            AdbRequestStatus::Okay => Ok(()),
        }
    }

    /// Sends the given [SyncCommand] to ADB server, and checks that the request has been taken in consideration.
    /// If an error occured, something will be returned? TODO
    pub(crate) fn send_sync_request(
        tcp_stream: &mut TcpStream,
        command: SyncCommand,
    ) -> Result<()> {
        let mut len_buf = [0_u8; 4];
        match command {
            SyncCommand::List(x)
            | SyncCommand::Recv(x, _)
            | SyncCommand::Send(x, _)
            | SyncCommand::Stat(x) => LittleEndian::write_u32(&mut len_buf, x.len() as u32),
        };

        // First send 8 byte common header
        tcp_stream.write_all(command.to_string().as_bytes())?;
        tcp_stream.write_all(&len_buf)?;

        // Then send specific content
        match command {
            SyncCommand::List(a) => Self::handle_list_command(a),
            SyncCommand::Recv(a, b) => Self::handle_recv_command(a, b),
            SyncCommand::Send(a, b) => Self::handle_send_command(a, b),
            SyncCommand::Stat(a) => Self::handle_stat_command(a),
        }

        // Reads returned status code from ADB server
        let mut request_status = [0; 4];
        tcp_stream.read_exact(&mut request_status)?;

        match AdbRequestStatus::from_str(str::from_utf8(request_status.as_ref())?)? {
            AdbRequestStatus::Fail => {
                // We can keep reading to get further details
                let length = Self::get_body_length(tcp_stream)?;

                let mut body = vec![
                    0;
                    length
                        .try_into()
                        .map_err(|_| RustADBError::ConvertionError)?
                ];
                if length > 0 {
                    tcp_stream.read_exact(&mut body)?;
                }

                Err(RustADBError::ADBRequestFailed(String::from_utf8(body)?))
            }
            AdbRequestStatus::Okay => Ok(()),
        }
    }

    fn handle_list_command(_: &str) {
        // List sends the string of the directory to list, and then the server sends a list of files
        todo!()
    }
    fn handle_recv_command(_: &str, _: String) {
        todo!()
    }
    fn handle_send_command(_: &str, _: String) {
        todo!()
    }
    fn handle_stat_command(_: &str) {
        todo!()
    }

    pub(crate) fn get_body_length(tcp_stream: &mut TcpStream) -> Result<u32> {
        let mut length = [0; 4];
        tcp_stream.read_exact(&mut length)?;

        Ok(u32::from_str_radix(str::from_utf8(&length)?, 16)?)
    }
}
