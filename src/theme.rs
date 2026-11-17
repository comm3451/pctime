//! Native dark theme: neutral grays, blue accents, 14px type, 4/8/16/24px spacing.
//! See THIRD-PARTY-NOTICES.txt; the notice is also embedded in the executable.
pub const BG:u32=0x171717;             // $body-bg-dark: $gray-900
pub const SURFACE:u32=0x262626;        // $body-secondary-bg-dark: $gray-800
pub const SURFACE_ALT:u32=0x202020;    // gray-800 / gray-900 mix
pub const INK:u32=0xE5E5E5;            // $body-color-dark: $gray-200
pub const MUTED:u32=0x999FA4;          // $text-secondary-dark
pub const LINE:u32=0x373737;           // translucent white border on dark surface
pub const PRIMARY:u32=0x066FD1;        // $blue
pub const PRIMARY_TEXT:u32=0x6AA9E3;   // tint-color($primary, 40%)
pub const PRIMARY_SUBTLE:u32=0x1A2E43;
pub const SUCCESS:u32=0x81D18F;        // $green mixed with white, 40%
pub const WARNING:u32=0xF9C566;        // $yellow mixed with white, 40%
pub const NOTICE:&str=include_str!("../THIRD-PARTY-NOTICES.txt");
