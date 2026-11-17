use crate::win::{Handle,NULL};

const REGULAR:&[u8]=include_bytes!("../assets/fonts/Pretendard-Regular.otf");
const SEMIBOLD:&[u8]=include_bytes!("../assets/fonts/Pretendard-SemiBold.otf");
pub const NOTICE:&str=include_str!("../assets/fonts/LICENSE.txt");

#[link(name="gdi32")]
unsafe extern "system" {
    fn AddFontMemResourceEx(data:*const u8,length:u32,reserved:Handle,count:*mut u32)->Handle;
    fn RemoveFontMemResourceEx(font:Handle)->i32;
}

/// Fonts stay private to this process; no system installation or extracted files.
pub struct EmbeddedFonts(Vec<Handle>);
impl EmbeddedFonts {
    pub fn load()->Result<Self,String> {
        let mut fonts=Self(Vec::new());
        for data in [REGULAR,SEMIBOLD] {
            let mut count=0;
            let font=unsafe{AddFontMemResourceEx(data.as_ptr(),data.len() as u32,NULL,&mut count)};
            if font.is_null(){return Err("Pretendard 폰트를 불러오지 못했습니다.".into())}
            fonts.0.push(font);
        }
        Ok(fonts)
    }
}
impl Drop for EmbeddedFonts {
    fn drop(&mut self){for &font in &self.0 {unsafe{RemoveFontMemResourceEx(font);}}}
}

// GDI uses the legacy family names of the static OTFs.
pub fn face(semibold:bool)->&'static str {if semibold{"Pretendard SemiBold"}else{"Pretendard"}}
