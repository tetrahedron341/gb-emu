use bitflags::bitflags;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct OamEntry {
    pub ypos: u8,
    pub xpos: u8,
    pub tile: u8,
    pub flags: OamEntryFlags,
}

impl std::fmt::Debug for OamEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OamEntry")
            .field("ypos", &self.ypos)
            .field("xpos", &self.xpos)
            .field("tile", &format_args!("{:#X}", &self.tile))
            .field("flags", &self.flags)
            .finish()
    }
}

bitflags! {
    #[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
    pub struct OamEntryFlags: u8 {
        const BG_PRIORITY = 0x80;
        const Y_FLIP = 0x40;
        const X_FLIP = 0x20;
        const PALETTE_OBP1 = 0x10;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LCDC: u8 {
        const LCD_ENABLE = 0x80;
        const WINDOW_TILEMAP_AREA = 0x40;
        const WINDOW_ENABLE = 0x20;
        const BG_TILE_DATA_AREA = 0x10;
        const BG_TILEMAP_AREA = 0x08;
        const OBJ_SIZE = 0x04;
        const OBJ_ENABLE = 0x02;
        const BG_ENABLE = 0x01;
        const BG_PRIORITY = 0x01;
    }
}

impl Default for LCDC {
    fn default() -> Self {
        LCDC::LCD_ENABLE | LCDC::BG_TILE_DATA_AREA | LCDC::BG_ENABLE
    }
}

bitflags! {
    #[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
    pub struct STAT: u8 {
        const LYC_INTERRUPT_ENABLE = 0x40;
        const OAM_INTERRUPT_ENABLE = 0x20;
        const VBLANK_INTERRUPT_ENABLE = 0x10;
        const HBLANK_INTERRUPT_ENABLE = 0x08;
        const LYC_EQUALS_LY = 0x04;

        const MODE_0 = 0;
        const MODE_1 = 1;
        const MODE_2 = 2;
        const MODE_3 = 3;
    }
}

impl STAT {
    const MODE_BITMASK: STAT = STAT::from_bits_truncate(0xFC);

    #[inline]
    pub fn set_mode(&mut self, mode: Self) {
        use std::assert_matches::assert_matches;
        assert_matches!(
            mode,
            STAT::MODE_0 | STAT::MODE_1 | STAT::MODE_2 | STAT::MODE_3
        );
        *self &= Self::MODE_BITMASK;
        *self |= mode;
    }

    /// Masks out all bits except for the mode bits in order to make matching easier
    #[inline]
    pub fn mode(&self) -> Self {
        *self & !Self::MODE_BITMASK
    }
}
