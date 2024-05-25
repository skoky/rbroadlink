//! Set of utility methods useful when working with network requests.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    slice::ChunksExact,
    time::Duration,
};

use tokio::time::timeout;

/// Computes the checksum of a slice of bytes.
///
/// The checksum is computed by summing all of the bytes with 0xBEAF and masking
/// with 0xFFFF.
pub fn checksum(data: &[u8]) -> u16 {
    // Get the checksum
    let mut sum = 0xBEAFu32;
    for &d in data {
        sum += u32::from(d);
    }

    return sum as u16;
}

/// Computes the generic checksum of a bytes array.
///
/// This iterates all 16-bit array elements, summing
/// the values into a 32-bit variable. This functions
/// paddies with zero an octet at the end (if necessary)
/// to turn into a 16-bit element.
pub fn compute_generic_checksum(buf: &[u8]) -> u16 {
    let mut state: u32 = 0xFFFF;

    let mut chunks_iter: ChunksExact<u8> = buf.chunks_exact(2);
    while let Some(chunk) = chunks_iter.next() {
        state += u16::from_le_bytes([chunk[0], chunk[1]]) as u32;
    }

    if let Some(&b) = chunks_iter.remainder().get(0) {
        state += u16::from_le_bytes([b, 0]) as u32;
    }

    state = (state >> 16) + (state & 0xffff);
    state = !state & 0xffff;

    state as u16
}

/// Returns the first available non-local address or the passed IP, if present.
pub fn local_ip_or(ip: Option<Ipv4Addr>) -> Result<IpAddr, String> {
    Ok(match ip {
        Some(ip) => IpAddr::V4(ip),
        None => get_if_addrs::get_if_addrs()
            .map_err(|e| {
                format!(
                    "Could not automatically determine machine IP address. {}",
                    e
                )
            })?
            .iter()
            .find(|x| x.ip().is_ipv4() && !x.ip().is_loopback())
            .ok_or("Could not find a local IPv4 address!")?
            .ip(),
    })
}

/// Sends a message and returns the received response.
fn send_and_receive_impl(
    msg: &[u8],
    addr: Ipv4Addr,
    port: Option<u16>,
) -> Result<UdpSocket, String> {
    // Set up the socket addresses
    let unspecified_addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port.unwrap_or(0)));
    let destination_addr = SocketAddr::from((addr, 80));

    // Set up the communication socket
    // Note: We need to enable support for broadcast
    let socket = UdpSocket::bind(unspecified_addr)
        .map_err(|e| format!("Could not bind to any port. {}", e))?;
    socket
        .set_broadcast(true)
        .map_err(|e| format!("Could not enable broadcast. {}", e))?;

    // Send the message
    socket
        .set_read_timeout(Some(Duration::new(10, 0)))
        .map_err(|e| format!("Could not set read timeout! {}", e))?;
    socket
        .send_to(&msg, destination_addr)
        .map_err(|e| format!("Could not broadcast message! {}", e))?;

    return Ok(socket);
}


async fn send_and_receive_impl_async(
    msg: &[u8],
    addr: Ipv4Addr,
    port: u16,
) -> Result<tokio::net::UdpSocket, String> {
    // Set up the socket addresses
    let unspecified_addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));
    let destination_addr = SocketAddr::from((addr, 80));

    // Set up the communication socket
    // Note: We need to enable support for broadcast

    // std::net::UdpSocket::set_nonblocking()
    let socket = tokio::net::UdpSocket::bind(unspecified_addr).await
        .map_err(|e| format!("Could not bind to any port. {}", e))?;

    socket
        .set_broadcast(true)
        .map_err(|e| format!("Could not enable broadcast. {}", e))?;

    // Send the message
    // socket.set_read_timeout(Duration::from_secs(3))
    //     .map_err(|e| format!("Could not set read timeout! {}", e))?;
    socket
        .send_to(&msg, destination_addr).await
        .map_err(|e| format!("Could not broadcast message! {}", e))?;

    return Ok(socket);
}

/// Sends a message and returns the as many received responses as possible (within a timeout).
pub fn send_and_receive_many<I, T>(
    msg: &[u8],
    addr: Ipv4Addr,
    port: Option<u16>,
    cb: T,
) -> Result<Vec<I>, String>
    where
        T: Fn(usize, &[u8], SocketAddr) -> Result<I, String>,
{
    // Get the socket
    let socket = send_and_receive_impl(msg, addr, port)
        .map_err(|e| format!("Could not create socket for message sending! {}", e))?;

    // Transform the results
    let mut results: Vec<I> = vec![];
    let mut recv_buffer = [0u8; 8092];
    while let Ok((bytes_received, addr)) = socket.recv_from(&mut recv_buffer) {
        results.push(cb(bytes_received, &recv_buffer[0..bytes_received], addr)?);
    }
    drop(socket);
    return Ok(results);
}

/// Sends a message and returns the as many received responses as possible (within a timeout).
pub async fn send_and_receive_many_async<I, T>(
    msg: &[u8],
    addr: Ipv4Addr,
    port: u16,
    cb: T,
    read_timeout: Duration,
) -> Result<Vec<I>, String>
    where
        T: Fn(usize, &[u8], SocketAddr) -> Result<I, String>,
{
    // Get the socket
    let socket = send_and_receive_impl_async(msg, addr, port).await
        .map_err(|e| format!("Could not create socket for message sending! {}", e))?;

    // Transform the results
    let mut results: Vec<I> = vec![];
    let mut recv_buffer = [0u8; 8092];
    loop {
        match timeout(read_timeout, socket.recv_from(&mut recv_buffer)).await {
            Ok(Ok((len, addr))) => {
                println!("received MSG");
                results.push(cb(len, &recv_buffer[0..len], addr)?)
                // Process the received data
            }
            Ok(Err(e)) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
            Err(_) => {
                println!("Receive operation timed out");
                break;
            }
        };
    }

    // while let Ok((bytes_received, addr)) = socket.recv_from(&mut recv_buffer).await {
    //     results.push(cb(bytes_received, &recv_buffer[0..bytes_received], addr)?);
    // }
    drop(socket);
    return Ok(results);
}

/// Sends a message and returns the first received response.
pub fn send_and_receive_one<I, T>(
    msg: &[u8],
    addr: Ipv4Addr,
    port: Option<u16>,
    cb: T,
) -> Result<I, String>
    where
        T: Fn(usize, &[u8], SocketAddr) -> Result<I, String>,
{
    // Get the socket
    let socket = send_and_receive_impl(msg, addr, port)
        .map_err(|e| format!("Could not create socket for message sending! {}", e))?;

    // Transform the result
    let mut recv_buffer = [0u8; 8092];
    if let Ok((bytes_received, addr)) = socket.recv_from(&mut recv_buffer) {
        drop(socket);
        return Ok(cb(bytes_received, &recv_buffer[0..bytes_received], addr)?);
    }
    drop(socket);
    return Err("No response within timeout!".into());
}

/// Sends a message and returns the first received response.
pub async fn send_and_receive_one_async<I, T>(
    msg: &[u8],
    addr: Ipv4Addr,
    port: u16,
    cb: T,
    response_timeout: Duration,
) -> Result<I, String>
    where
        T: Fn(usize, &[u8], SocketAddr) -> Result<I, String>,
{
    // Get the socket
    let socket = send_and_receive_impl_async(msg, addr, port).await
        .map_err(|e| format!("Could not create socket for message sending! {}", e))?;

    // Transform the result
    let mut recv_buffer = [0u8; 8092];
    let result = match timeout(response_timeout, socket.recv_from(&mut recv_buffer)).await {
        Ok(Ok((len, addr))) => {
            Ok(cb(len, &recv_buffer[0..len], addr)?)
            // Process the received data
        }
        Ok(Err(e)) => {
            eprintln!("Error receiving data: {}", e);
            Err("Error receiving".to_string())
        }
        Err(_) => {
            println!("Receive operation timed out");
            Err("timeout".to_string())
        }
    };

    // if let Ok((bytes_received, addr)) = socket.recv_from(&mut recv_buffer).await {
    //     return Ok(cb(bytes_received, &recv_buffer[0..bytes_received], addr)?);
    // }
    drop(socket);
    // return Err("No response within timeout!".into());
    return result;
}

/// Reverses a MAC address. Used to fix the backwards response from the broadlink device.
pub fn reverse_mac(mac_flipped: [u8; 6]) -> [u8; 6] {
    // Fix the mac address by reversing it.
    let mut mac = [0u8; 6];
    for i in 0..6 {
        mac[i] = mac_flipped[6 - i - 1];
    }

    return mac;
}
