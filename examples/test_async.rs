use std::net::Ipv4Addr;
use std::process::exit;
use std::str::FromStr;
use std::time::Duration;

use rbroadlink::Device;
use rbroadlink::traits::DeviceTrait;

#[tokio::main]
async fn main() {

    let devices = match Device::list_async(None, Duration::from_secs(3)).await {
        Ok(devices) => devices,
        Err(e) => {
            eprintln!("Error: {}", e);
            exit(1)
        }
    };

    let device_ip = devices.first().expect("No device found").clone().get_info().address;
    println!("Device IP {}", device_ip);

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
