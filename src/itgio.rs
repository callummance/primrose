use std::time::Duration;

use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use rusb::{
    request_type, Context, Device, DeviceHandle, Direction, Recipient, RequestType, UsbContext,
};

const ITGIO_IFACE: u8 = 0;
const USB_IDS: [DeviceIdentifier; 3] = [
    DeviceIdentifier {
        vid: 0x07C0,
        pid: 0x1502,
    },
    DeviceIdentifier {
        vid: 0x07C0,
        pid: 0x1582,
    },
    DeviceIdentifier {
        vid: 0x07C0,
        pid: 0x1584,
    },
];

//USB control data
const HID_GET_REPORT_BREQUEST: u8 = 0x01;
const HID_SET_REPORT_BREQUEST: u8 = 0x09;

const HID_IFACE_IN_WVALUE: u16 = 256;
const HID_IFACE_OUT_WVALUE: u16 = 512;

const REQ_TIMEOUT: Duration = Duration::new(0, 500);

const EXPECTED_CONTROL_LEN: usize = 4;

pub struct ItgioDevice {
    device: Device<Context>,
    handle: Option<DeviceHandle<Context>>,
    pub ident: DeviceIdentifier,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct DeviceIdentifier {
    vid: u16,
    pid: u16,
}

impl<'a> ItgioDevice {
    pub fn find_devs(ctx: Context) -> Result<Vec<ItgioDevice>> {
        let devices = ctx.devices()?;
        let matches: Vec<ItgioDevice> = devices
            .iter()
            .map(|libusb_dev| {
                let ident = libusb_dev
                    .device_descriptor()
                    .map(|desc| DeviceIdentifier {
                        vid: desc.vendor_id(),
                        pid: desc.product_id(),
                    })
                    .unwrap_or(DeviceIdentifier { vid: 0, pid: 0 });
                ItgioDevice {
                    device: libusb_dev,
                    handle: None,
                    ident,
                }
            })
            .filter(|dev| USB_IDS.iter().find(|id| **id == dev.ident).is_some())
            .inspect(|dev| {
                debug!(
                    "Found matching device with vid {}, pid {}",
                    dev.ident.vid, dev.ident.pid
                )
            })
            .collect();

        info!("Found a total of {} itgio devices", matches.len());
        Ok(matches)
    }

    fn get_handle(&'a mut self) -> Result<&'a mut DeviceHandle<Context>> {
        Ok(self.handle.get_or_insert(self.device.open()?))
    }

    pub fn open(&mut self) -> Result<()> {
        if let Err(e) = self.try_open() {
            self.close()?;
            Err(e)
        } else {
            Ok(())
        }
    }

    fn try_open(&mut self) -> Result<()> {
        //Close existing handle
        self.close()?;

        //Get new handle
        let ident = self.ident;
        info!("Opening itgio device {:?}", ident);
        let handle = self.get_handle()?;

        //Detach any kernel drivers if possible
        handle
            .device()
            .active_config_descriptor()?
            .interfaces()
            .try_for_each(|iface| {
                let iface_idx = iface.number();

                match handle.detach_kernel_driver(iface_idx) {
                    Ok(_) => Ok(()),
                    Err(rusb::Error::NotFound) => Ok(()),
                    Err(rusb::Error::InvalidParam) => Ok(()),
                    Err(rusb::Error::NoDevice) => Ok(()),
                    Err(rusb::Error::NotSupported) => {
                        //Can't unload the driver on this platform
                        error!("A kernel driver is already attached to device {:?} but driver unloading is not supported on this platform", ident);
                        Err(rusb::Error::NotSupported)
                    }
                    Err(e) => {
                        //Some other error occurred
                        error!(
                            "Failed to detach kernel driver for device {:?} due to error {:?}",
                            ident, e
                        );
                        Err(e)
                    }
                }
            })?;

        //Try to claim all interfaces
        handle
            .device()
            .active_config_descriptor()?
            .interfaces()
            .try_for_each(|iface| {
                let iface_idx = iface.number();

                handle.claim_interface(iface_idx)
            })?;

        Ok(())
    }

    pub fn read_buttons(&self) -> Result<ItgioBtnStatus> {
        let mut buf: [u8; 4] = [0; 4];

        let bm_request_type: u8 =
            request_type(Direction::In, RequestType::Class, Recipient::Interface);
        let handle = self.handle.as_ref().ok_or(anyhow!(
            "Tried to read button status from device without opening it first."
        ))?;
        let isize = handle.read_control(
            bm_request_type,
            HID_GET_REPORT_BREQUEST,
            HID_IFACE_IN_WVALUE,
            0,
            &mut buf,
            REQ_TIMEOUT,
        )?;

        if isize != EXPECTED_CONTROL_LEN {
            return Err(anyhow!(
                "Got response of length {} instead of the expected length 4: {:?}",
                isize,
                buf
            ));
        }

        let res = u32::from_le_bytes(buf);
        //Use bitwise NOT to convert from low active to high active
        Ok(ItgioBtnStatus(!res))
    }

    pub fn write_lights(&self, data: ItgioLights) -> Result<()> {
        let mut buf: [u8; 4] = data.0.to_le_bytes();

        let bm_request_type: u8 =
            request_type(Direction::Out, RequestType::Class, Recipient::Interface);
        let handle = self.handle.as_ref().ok_or(anyhow!(
            "Tried to read button status from device without opening it first."
        ))?;
        let isize = handle.write_control(
            bm_request_type,
            HID_SET_REPORT_BREQUEST,
            HID_IFACE_OUT_WVALUE,
            0,
            &mut buf,
            REQ_TIMEOUT,
        )?;

        if isize != EXPECTED_CONTROL_LEN {
            return Err(anyhow!(
                "Only managed to send {} bytes of control message instead of the expected 4: {:?}",
                isize,
                buf
            ));
        }

        Ok(())
    }

    pub fn close(&mut self) -> Result<()> {
        self.handle
            .take()
            .into_iter()
            .for_each(|mut handle: DeviceHandle<Context>| {
                info!("Closing itgio device {:?}", self.ident);
                if let Err(e) = handle.release_interface(ITGIO_IFACE) {
                    warn!(
                        "Failed to release iface {} on itgio device due to error {:?}",
                        ITGIO_IFACE, e
                    )
                }

                if let Err(e) = handle.reset() {
                    warn!("Failed to reset itgio device due to error {:?}", e);
                }
                drop(handle);
            });
        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct ItgioBtnStatus(pub u32);

impl ItgioBtnStatus {}

#[derive(Default, Debug)]
pub struct ItgioLights(pub u32);
