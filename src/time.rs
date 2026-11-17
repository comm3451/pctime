use crate::win::*;
pub const SECOND:i64 = 10_000_000;
pub const MINUTE:i64 = 60 * SECOND;
pub const DAY:i64 = 86400 * SECOND;
pub fn ticks(ft:FileTime)->i64 { ((ft.high as u64)<<32 | ft.low as u64) as i64 }
pub fn file(t:i64)->FileTime { FileTime { low:t as u32, high:(t as u64>>32) as u32 } }
pub fn now()->i64 { let mut t=FileTime::default(); unsafe { GetSystemTimeAsFileTime(&mut t); } ticks(t) }
pub fn from_utc(st:SystemTime)->Option<i64> { let mut t=FileTime::default(); (unsafe { SystemTimeToFileTime(&st,&mut t) }!=0).then(||ticks(t)) }
pub fn utc(t:i64)->SystemTime { let mut s=SystemTime::default(); unsafe { FileTimeToSystemTime(&file(t),&mut s); } s }
pub fn local(t:i64)->SystemTime { let mut s=SystemTime::default(); unsafe { SystemTimeToTzSpecificLocalTime(NULL,&utc(t),&mut s); } s }
pub fn from_local(s:SystemTime)->i64 { let mut u=SystemTime::default(); unsafe { TzSpecificLocalTimeToSystemTime(NULL,&s,&mut u); } from_utc(u).unwrap_or(0) }
pub fn midnight(t:i64)->i64 { let mut s=local(t); s.hour=0;s.minute=0;s.second=0;s.millis=0;from_local(s) }
// Move calendar dates in UTC first, then convert the new local midnight. This preserves DST boundaries.
pub fn shift_date(mid:i64,days:i32)->i64 { let s=local(mid); let u=from_utc(SystemTime { hour:12,minute:0,second:0,millis:0,..s }).unwrap(); let mut next=utc(u+days as i64*DAY); next.hour=0;from_local(next) }
pub fn date(t:i64)->String { let s=local(t);format!("{:04}.{:02}.{:02}",s.year,s.month,s.day) }
pub fn clock(t:i64)->String { let s=local(t);format!("{:02}:{:02}:{:02}",s.hour,s.minute,s.second) }
pub fn stamp(t:i64)->String { format!("{} {}",date(t),clock(t)) }
pub fn parse(s:&str)->Option<i64> {
    if s.len()<20 || !s.is_ascii() || !s.ends_with('Z') { return None; }
    if &s[4..5]!="-" || &s[7..8]!="-" || &s[10..11]!="T" || &s[13..14]!=":" || &s[16..17]!=":" { return None; }
    let n=|a,b|s.get(a..b)?.parse::<u16>().ok();
    let st=SystemTime { year:n(0,4)?,month:n(5,7)?,day:n(8,10)?,hour:n(11,13)?,minute:n(14,16)?,second:n(17,19)?,..Default::default() };
    let base=from_utc(st)?;
    let fraction=if s.as_bytes()[19]==b'.' { let f=s.get(20..s.len()-1)?;if f.is_empty() || !f.bytes().all(|c|c.is_ascii_digit()){return None} let mut f=f.chars().take(7).collect::<String>();while f.len()<7 {f.push('0')} f.parse::<i64>().ok()? } else if s.len()==20 {0} else {return None};
    Some(base+fraction)
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn timestamp_precision(){ assert_eq!(parse("2026-11-17T00:00:00.1234567Z").unwrap()-parse("2026-11-17T00:00:00Z").unwrap(),1_234_567); }
    #[test] fn invalid_timestamp(){for s in ["bad","2026-13-17T00:00:00Z","2026-11-17T00:00:00+09:00","2026-11-17T00:00:00.xZ"]{assert!(parse(s).is_none());} }
    #[test] fn calendar_midnights(){let t=parse("2026-11-17T12:00:00Z").unwrap();assert_eq!(shift_date(shift_date(midnight(t),-31),31),midnight(t));}
}
