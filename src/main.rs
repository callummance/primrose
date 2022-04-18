use log::{error, warn};

use crate::device::ItgioTranslator;

mod device;
mod itgio;
mod mapping;
mod sextetstream;

const SEXTET_BASE_PATH: &'static str = "/opt/itgio";

fn main() {
    let ctx = rusb::Context::new().expect("Failed to initialize libusb context");
    let found_devices =
        itgio::ItgioDevice::find_devs(ctx).expect("Failed to enumerate ITGIO devices");
    match found_devices.len() {
        0 => panic!("No matching devices found"),
        1 => (),
        _ => warn!("Multiple matching devices found, this may not exit properly"),
    }

    let device_idx = 0;
    let mut device_handles: Vec<ItgioTranslator> = Vec::new();
    //let device_handles = found_devices
    //    .into_iter()
    //    .map(|device| device::ItgioTranslator::init(SEXTET_BASE_PATH.to_string(), device))
    //    .scan(initial_state, f)
    //    .collect::<Vec<Result<ItgioTranslator>>>();

    for device in found_devices {
        //Initialize all devices
        let translator = match device::ItgioTranslator::init(
            format!("{}_{}", SEXTET_BASE_PATH, device_idx),
            device,
        ) {
            Ok(res) => res,
            Err(e) => {
                error!(
                    "Failed to initialize translator for device due to error {}",
                    e
                );
                continue;
            }
        };
        //Start listening
        if let Err(e) = translator.start_translation() {
            error!(
                "Failed to initialize translator for device due to error {}",
                e
            );
            continue;
        }

        device_handles.push(translator);
    }

    if let Some(first_dev) = device_handles.first() {
        first_dev.wait_exit();
    }
}
