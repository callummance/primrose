use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use crate::{
    itgio::{ItgioDevice, ItgioLights},
    mapping::{btn_bitmap_to_sextetstream, sextetstream_to_lights},
    sextetstream::{SextetStreamReader, SextetStreamWriter},
};

use anyhow::Result;
use crossbeam_utils::sync::Parker;
use log::{debug, error};

const POLL_DELAY: Duration = Duration::from_millis(20);

pub struct ItgioTranslator {
    device: Arc<Mutex<ItgioDevice>>,
    should_close: Arc<AtomicBool>,
    close_notifier: Parker,
    reader_path: String,
    writer_path: String,
}

impl ItgioTranslator {
    pub fn init(stream_base_path: String, mut device: ItgioDevice) -> Result<ItgioTranslator> {
        device.open()?;
        let device = Arc::new(Mutex::new(device));
        let reader_path = stream_base_path.clone() + "-lights";
        let writer_path = stream_base_path.clone() + "-buttons";

        let should_close = Arc::new(AtomicBool::new(false));
        let close_notifier = Parker::new();

        Ok(ItgioTranslator {
            device,
            should_close,
            close_notifier,
            reader_path,
            writer_path,
        })
    }

    pub fn start_translation(&self) -> Result<()> {
        let sextetstream_reader = SextetStreamReader::open(&self.reader_path)?;
        let sextetstream_writer = SextetStreamWriter::open(&self.writer_path)?;

        //Start lights monitor
        let device = Arc::clone(&self.device);
        let unparker = self.close_notifier.unparker().clone();
        let should_close = Arc::clone(&self.should_close);
        thread::spawn(move || {
            if let Err(e) = Self::monitor_lights(sextetstream_reader, device, &should_close) {
                error!(
                    "Lights translator encountered an error {:?} and had to exit",
                    e
                );
                should_close.store(true, Ordering::Relaxed);
                unparker.unpark();
            }
        });

        //Start buttons monitor
        let device = Arc::clone(&self.device);
        let unparker = self.close_notifier.unparker().clone();
        let should_close = Arc::clone(&self.should_close);
        thread::spawn(move || {
            if let Err(e) = Self::monitor_buttons(sextetstream_writer, device, &should_close) {
                error!(
                    "Buttons translator encountered an error {:?} and had to exit",
                    e
                );
                should_close.store(true, Ordering::Relaxed);
                unparker.unpark();
            }
        });

        Ok(())
    }

    pub fn wait_exit(&self) {
        self.close_notifier.park();
    }

    fn monitor_lights(
        mut sextetstream_reader: SextetStreamReader,
        device: Arc<Mutex<ItgioDevice>>,
        should_close: &AtomicBool,
    ) -> Result<()> {
        debug!("Started monitoring lights...");
        let mut ss_buf: Vec<u8> = Vec::with_capacity(12);

        while !should_close.load(Ordering::Relaxed) {
            sextetstream_reader.read_line(&mut ss_buf)?;
            let lights_bitmap = sextetstream_to_lights(&ss_buf);

            let device = device.lock().unwrap();
            device.write_lights(ItgioLights(lights_bitmap))?;
        }

        unreachable!()
    }

    fn monitor_buttons(
        mut sextetstream_writer: SextetStreamWriter,
        device: Arc<Mutex<ItgioDevice>>,
        should_close: &AtomicBool,
    ) -> Result<()> {
        debug!("Started monitoring buttons...");
        let mut ss_buf: Vec<u8> = Vec::with_capacity(6);

        let mut last_update_time = Instant::now();
        let mut last_update_state = 0u32;

        while !should_close.load(Ordering::Relaxed) {
            let device = device.lock().unwrap();
            let state = device.read_buttons()?.0;
            if state != last_update_state {
                debug!("ITGIO button state changed to {:#034b}", state);
                last_update_state = state;

                let state_ss_len = btn_bitmap_to_sextetstream(state, &mut ss_buf);
                let ss_slice = &ss_buf[0..state_ss_len];
                sextetstream_writer.write_line(ss_slice)?;
            }

            //sleep until the update delay has passed
            let time_since_update_start = Instant::now().duration_since(last_update_time);
            debug!("Device polling took {:#?}", time_since_update_start);
            let time_to_wait = POLL_DELAY - time_since_update_start;
            thread::sleep(time_to_wait);
            last_update_time = Instant::now()
        }

        unreachable!()
    }
}
