use strum::FromRepr;

const ITGIO_BITMAP_BITS: usize = 16;

pub fn btn_bitmap_to_sextetstream(btns_bitmap: u32, sextetstream_bytes: &mut [u8]) -> usize {
    //Zero out sextetstream bytes
    sextetstream_bytes.fill(0);

    //Keep track of how many sextets are needed to represent button status
    let mut bytes_cnt = 0;
    for i in 0..ITGIO_BITMAP_BITS {
        if btns_bitmap & (0b1 << i) != 0 {
            let byte_idx = i / 6;
            let bit_idx = i % 6;

            sextetstream_bytes[byte_idx] |= 0b1 << bit_idx;
            bytes_cnt = byte_idx
        }
    }

    //Ensure each of the bytes used is a prinatble character
    for b in sextetstream_bytes[0..bytes_cnt].iter_mut() {
        //Recomended packing function from https://github.com/stepmania/stepmania/blob/5_1-new/src/arch/InputHandler/InputHandler_SextetStream.md
        *b = ((*b + 0x10) & 0x3f) + 0x30;
    }

    bytes_cnt
}

pub fn sextetstream_to_lights(sextetstream_bytes: &[u8]) -> u32 {
    let mut res = 0u32;

    for (byte_idx, byte) in sextetstream_bytes.iter().enumerate() {
        if *byte == 0 {
            continue;
        }

        for bit in 0..6 {
            if byte & (0b1 << bit) != 0 {
                let light = CabinetLight::from_repr(byte_idx * 6 + bit);
                if let Some(l) = light {
                    l.add_to_itgio_bitmap(&mut res);
                }
            }
        }
    }

    res
}

#[derive(FromRepr, Debug, PartialEq, Eq)]
pub enum CabinetLight {
    MarqueeUpperLeft,
    MarqueeUpperRight,
    MarqueeLowerLeft,
    MarqueeLowerRight,
    BassLeft,
    BassRight,

    Player1MenuLeft,
    Player1MenuRight,
    Player1MenuUp,
    Player1MenuDown,
    Player1Start,
    Player1Select,
    Player1Back,
    Player1Coin,
    Player1Operator,
    Player1EffectUp,
    Player1EffectDown,
    Player1Reserved1,

    Player1PadLeft,
    Player1PadRight,
    Player1PadUp,
    Player1PadDown,
    Player1PadUpLeft,
    Player1PadUpRight,
    Player1PadCentre,
    Player1PadDownLeft,
    Player1PadDownRight,
    Player1Light10,
    Player1Light11,
    Player1Light12,
    Player1Light13,
    Player1Light14,
    Player1Light15,
    Player1Light16,
    Player1Light17,
    Player1Light18,
    Player1Light19,
    Player1Reserved2,
    Player1Reserved3,
    Player1Reserved4,
    Player1Reserved5,
    Player1Reserved6,

    Player2MenuLeft,
    Player2MenuRight,
    Player2MenuUp,
    Player2MenuDown,
    Player2Start,
    Player2Select,
    Player2Back,
    Player2Coin,
    Player2Operator,
    Player2EffectUp,
    Player2EffectDown,
    Player2Reserved1,

    Player2PadLeft,
    Player2PadRight,
    Player2PadUp,
    Player2PadDown,
    Player2PadUpLeft,
    Player2PadUpRight,
    Player2PadCentre,
    Player2PadDownLeft,
    Player2PadDownRight,
    Player2Light10,
    Player2Light11,
    Player2Light12,
    Player2Light13,
    Player2Light14,
    Player2Light15,
    Player2Light16,
    Player2Light17,
    Player2Light18,
    Player2Light19,
    Player2Reserved2,
    Player2Reserved3,
    Player2Reserved4,
    Player2Reserved5,
    Player2Reserved6,
}

impl CabinetLight {
    pub fn add_to_itgio_bitmap(&self, bitmap: &mut u32) {
        *bitmap |= 0b1u32
            << match self {
                CabinetLight::MarqueeUpperLeft => 8,
                CabinetLight::MarqueeUpperRight => 10,
                CabinetLight::MarqueeLowerLeft => 9,
                CabinetLight::MarqueeLowerRight => 11,
                CabinetLight::BassLeft => 15,
                CabinetLight::BassRight => 15,
                CabinetLight::Player1Start => 13,
                CabinetLight::Player1PadLeft => 1,
                CabinetLight::Player1PadRight => 0,
                CabinetLight::Player1PadUp => 3,
                CabinetLight::Player1PadDown => 2,
                CabinetLight::Player2Start => 12,
                CabinetLight::Player2PadLeft => 5,
                CabinetLight::Player2PadRight => 4,
                CabinetLight::Player2PadUp => 7,
                CabinetLight::Player2PadDown => 6,
                _ => return,
            }
    }
}
