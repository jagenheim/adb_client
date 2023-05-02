use byteorder::{ByteOrder, LittleEndian};
use std::{
    fs::File,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
    path::{Path, PathBuf},
    str,
    str::FromStr,
    time::SystemTime,
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
    /// Note: This function does not take a tcp_stream anymore, as it is already stored in the struct.
    pub(crate) fn send_sync_request(&mut self, command: SyncCommand) -> Result<()> {
        // Send specific data depending on command
        match command {
            SyncCommand::List(a) => self.handle_list_command(a)?,
            SyncCommand::Recv(a, b) => Self::handle_recv_command(a, b),
            SyncCommand::Send(a, b) => self.handle_send_command(a, b)?,
            SyncCommand::Stat(a) => Self::handle_stat_command(a),
        }

        Ok(())
    }

    // This command does not seem to work correctly. The devices I test it on just resturn
    // 'DONE' directly without listing anything.
    fn handle_list_command(&mut self, path: &str) -> Result<()> {
        let mut len_buf = [0_u8; 4];
        LittleEndian::write_u32(&mut len_buf, path.len() as u32);

        // First send 8 byte common header
        self.tcp_stream
            .write_all(SyncCommand::List(path).to_string().as_bytes())?;
        self.tcp_stream.write_all(&len_buf)?;

        // List sends the string of the directory to list, and then the server sends a list of files
        self.tcp_stream.write_all(path.to_string().as_bytes())?;

        // Reads returned status code from ADB server
        let mut response = [0_u8; 4];
        loop {
            self.tcp_stream.read_exact(&mut response)?;
            match str::from_utf8(response.as_ref())? {
                "DENT" => {
                    // TODO: Move this to a struct that extract this data
                    let mut file_mod = [0_u8; 4];
                    let mut file_size = [0_u8; 4];
                    let mut mod_time = [0_u8; 4];
                    let mut name_len = [0_u8; 4];
                    self.tcp_stream.read_exact(&mut file_mod)?;
                    self.tcp_stream.read_exact(&mut file_size)?;
                    self.tcp_stream.read_exact(&mut mod_time)?;
                    self.tcp_stream.read_exact(&mut name_len)?;
                    let name_len = LittleEndian::read_u32(&name_len);
                    let mut name_buf = vec![0_u8; name_len as usize];
                    self.tcp_stream.read_exact(&mut name_buf)?;
                }
                "DONE" => {
                    //println!("We are done");
                    return Ok(());
                }
                x => println!("Unknown response {}", x),
            }
        }
    }

    fn handle_recv_command(_: &str, _: String) {
        todo!()
    }

    fn handle_send_command(&mut self, from: &str, to: String) -> Result<()> {
        // Append the filename from from to the path of to
        // FIXME: This should only be done if to doesn't already contain a filename
        // I guess we need to STAT the to file first to check this
        let mut to = PathBuf::from(to);
        to.push(Path::new(from).file_name().unwrap());
        let to = to.display().to_string() + ",0777";

        // First send 8 byte common header
        let mut len_buf = [0_u8; 4];
        LittleEndian::write_u32(&mut len_buf, to.len() as u32);
        self.tcp_stream
            .write_all(SyncCommand::Send(from, to.clone()).to_string().as_bytes())?;
        self.tcp_stream.write_all(&len_buf)?;

        // Send appends the filemode to the string sent
        self.tcp_stream.write_all(to.as_bytes())?;

        // Then we send the byte data in chunks of up to 64k
        // Chunk looks like 'DATA' <length> <data>
        let mut file = File::open(Path::new(from)).unwrap();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            let mut chunk_len_buf = [0_u8; 4];
            LittleEndian::write_u32(&mut chunk_len_buf, bytes_read as u32);
            self.tcp_stream.write_all(b"DATA")?;
            self.tcp_stream.write_all(&chunk_len_buf)?;
            self.tcp_stream.write_all(&buffer[..bytes_read])?;
        }

        // When we are done sending, we send 'DONE' <last modified time>
        // Re-use len_buf to send the last modified time
        let metadata = std::fs::metadata(Path::new(from))?;
        let last_modified = match metadata.modified()?.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(n) => n,
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        };
        LittleEndian::write_u32(&mut len_buf, last_modified.as_secs() as u32);
        self.tcp_stream.write_all(b"DONE")?;
        self.tcp_stream.write_all(&len_buf)?;

        // We expect 'OKAY' response from this

        Ok(())
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
