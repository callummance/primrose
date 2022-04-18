use std::{
    fs::{self, File},
    io::{BufReader, Write},
};
use std::{io::BufRead, os::unix::fs::FileTypeExt};

use anyhow::Result;
use log::{error, info};
use nix::{sys::stat::Mode, unistd::mkfifo};

//Create pipes readable and writable to owner and group
fn pipe_mode() -> Mode {
    Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IWGRP
}

pub fn get_or_create_pipe(path: &str) -> Result<File> {
    match mkfifo(path, pipe_mode()) {
        Ok(_) => {
            info!("Created named pipe at {}", path);
            Ok(File::open(path)?)
        }
        Err(nix::errno::Errno::EEXIST) => {
            //A file already exists, so just check if it is a named pipe
            let meta = fs::metadata(path)?;
            if meta.file_type().is_fifo() {
                //It is a named pipe, so we can just open it
                info!("Using existing fifo at {}", path);
                Ok(File::open(path)?)
            } else {
                //A non-pipe file already exists, so throw an error
                error!(
                    "Couldn't create new fifo at {} as a file already exists",
                    path
                );
                Err(anyhow::anyhow!("File already exists"))
            }
        }
        Err(e) => Err(e)?,
    }
}

pub struct SextetStreamReader {
    path: String,
    reader: BufReader<File>,
}

impl SextetStreamReader {
    pub fn open(path: &str) -> Result<SextetStreamReader> {
        let file = get_or_create_pipe(&path)?;
        let reader = BufReader::new(file);

        Ok(SextetStreamReader {
            path: path.to_string(),
            reader,
        })
    }

    pub fn read_line(&mut self, buf: &mut Vec<u8>) -> Result<()> {
        self.reader.read_until(0x0A, buf)?;
        Ok(())
    }
}

pub struct SextetStreamWriter {
    path: String,
    writer: File,
}

impl SextetStreamWriter {
    pub fn open(path: &str) -> Result<SextetStreamWriter> {
        let writer = get_or_create_pipe(&path)?;

        Ok(SextetStreamWriter {
            path: path.to_string(),
            writer,
        })
    }

    pub fn write_line(&mut self, data: &[u8]) -> Result<()> {
        let newline = [0x0Au8; 1];
        self.writer.write_all(data)?;
        self.writer.write(&newline)?;
        self.writer.flush()?;
        Ok(())
    }
}
