use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;

use rbroadlink::Device;

#[tokio::main]
async fn main() {
    let device_ip = Ipv4Addr::from_str("10.0.10.32").unwrap();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(1)).await;
        loop {
            // Construct a device directly
            match Device::from_ip(device_ip, None) {
                Ok(device) => {
                    println!("inside:  {}", device);
                }
                Err(e) => eprintln!("Error {}", e)
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    // Construct a device directly
    let device = Device::from_ip(device_ip, None).expect("Could not connect to device2!");
    println!("outside:  {}", device);

    tokio::time::sleep(Duration::from_secs(30)).await
}
