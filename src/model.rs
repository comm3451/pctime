use crate::time;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Boot, Stop, Sleep, Wake, CrashStop, UnexpectedBoot }
#[derive(Clone, Debug)]
pub struct Event { pub at:i64, pub kind:Kind, pub id:u32, pub record:u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End { Shutdown, Sleep, Crash, Live, Unknown }
#[derive(Clone, Debug)]
pub struct Session { pub start:i64, pub end:Option<i64>, pub boundary:i64, pub reason:End, pub resumed:bool }
impl Session {
    pub fn seconds(&self)->Option<i64> { self.end.map(|e|(e-self.start).max(0)/time::SECOND) }
    pub fn label(&self)->&'static str { match self.reason { End::Shutdown=>"정상 종료",End::Sleep=>"절전 전환",End::Crash=>"비정상 종료 · 추정",End::Live=>"현재 켜짐",End::Unknown=>"종료 기록 없음 · 합계 제외" } }
    pub fn estimated(&self)->bool { self.reason==End::Crash }
}

pub fn reconstruct(events:&[Event],now:i64)->Vec<Session> {
    let mut events=events.iter().filter(|e|e.at<=now).collect::<Vec<_>>();
    events.sort_by_key(|e|(e.at,e.record));
    let mut out=Vec::new();
    let mut open:Option<(i64,bool)>=None;
    for e in events {
        match e.kind {
            Kind::Boot=>{
                if let Some((start,resumed))=open.take() { out.push(Session { start,end:None,boundary:e.at,reason:End::Unknown,resumed }); }
                open=Some((e.at,false));
            }
            Kind::Wake=>{
                if open.is_none() {open=Some((e.at,true));}
            }
            Kind::Sleep=>{
                if let Some((start,resumed))=open.take() {out.push(Session {start,end:Some(e.at),boundary:e.at,reason:End::Sleep,resumed});}
            }
            Kind::Stop|Kind::CrashStop=>{
                if let Some((start,resumed))=open.take() {
                    out.push(Session {start,end:Some(e.at),boundary:e.at,reason:if e.kind==Kind::CrashStop {End::Crash}else{End::Shutdown},resumed});
                }
            }
            Kind::UnexpectedBoot=>{
                // ID 41 belongs to the new boot, not the time of the previous crash.
                // A nearby Boot has already closed the unpaired previous session.
                if let Some((start,resumed))=open {
                    if e.at-start>2*time::MINUTE {
                        out.push(Session {start,end:None,boundary:e.at,reason:End::Unknown,resumed});
                        open=None;
                    }
                }
            }
        }
    }
    if let Some((start,resumed))=open {
        out.push(Session {start,end:Some(now),boundary:now,reason:End::Live,resumed});
    }
    out
}

#[derive(Clone,Debug)]
pub struct Day { pub start:i64, pub seconds:i64, pub count:usize, pub uncertain:usize, pub estimated:bool, pub live:bool, pub first:Option<i64>, pub last:Option<i64> }
pub fn days(sessions:&[Session],now:i64,count:usize)->Vec<Day> {
    let today=time::midnight(now);
    (0..count).map(|n|{
        let start=time::shift_date(today,-(n as i32));
        let end=time::shift_date(start,1);
        let mut d=Day {start,seconds:0,count:0,uncertain:0,estimated:false,live:false,first:None,last:None};
        let mut ticks=0;
        for s in sessions {
            let finish=s.end.unwrap_or(s.boundary);
            if s.start>=end || finish<=start {continue}
            d.first=Some(d.first.map_or(s.start.max(start),|v|v.min(s.start.max(start))));
            if let Some(stop)=s.end {
                ticks+=(stop.min(end)-s.start.max(start)).max(0);
                d.count+=1;
                d.estimated|=s.estimated();
                d.live|=s.reason==End::Live && stop>=start && stop<end;
                d.last=Some(d.last.map_or(stop.min(end),|v|v.max(stop.min(end))));
            } else {d.uncertain+=1;}
        }
        d.seconds=ticks/time::SECOND;d
    }).collect()
}
pub fn duration(secs:i64)->String {format!("{}시간 {:02}분",secs/3600,secs%3600/60)}
pub fn compact(secs:i64)->String {format!("{}h {:02}m",secs/3600,secs%3600/60)}

#[cfg(test)] mod tests {
    use super::*;
    fn ev(at:i64,kind:Kind)->Event {Event {at:at*time::SECOND,kind,id:0,record:0}}
    #[test] fn shutdown_and_current_boot(){let s=reconstruct(&[ev(10,Kind::Boot),ev(30,Kind::Stop),ev(40,Kind::Boot)],60*time::SECOND);assert_eq!(s.iter().map(|s|s.seconds().unwrap()).sum::<i64>(),40);assert_eq!(s[1].reason,End::Live);}
    #[test] fn sleep_is_always_excluded(){let e=[ev(10,Kind::Boot),ev(20,Kind::Sleep),ev(40,Kind::Wake),ev(50,Kind::Stop)];let s=reconstruct(&e,60*time::SECOND);assert_eq!(s.iter().map(|s|s.seconds().unwrap()).sum::<i64>(),20);assert_eq!(s.len(),2);assert_eq!(s[0].reason,End::Sleep);}
    #[test] fn missing_shutdown_never_counts_offline_gap(){let s=reconstruct(&[ev(10,Kind::Boot),ev(90,Kind::Boot)],100*time::SECOND);assert_eq!(s[0].seconds(),None);assert_eq!(s[1].seconds(),Some(10));}
    #[test] fn crash_timestamp_precedes_report(){let s=reconstruct(&[ev(10,Kind::Boot),ev(40,Kind::CrashStop),ev(80,Kind::Boot),ev(81,Kind::UnexpectedBoot)],100*time::SECOND);assert_eq!(s[0].seconds(),Some(30));assert_eq!(s[0].reason,End::Crash);assert_eq!(s[1].reason,End::Live);}
    #[test] fn duplicate_wake_does_not_reset_start(){let s=reconstruct(&[ev(10,Kind::Boot),ev(20,Kind::Sleep),ev(30,Kind::Wake),ev(31,Kind::Wake)],40*time::SECOND);assert_eq!(s[1].seconds(),Some(10));}
    #[test] fn no_boot_is_not_invented(){assert!(reconstruct(&[ev(20,Kind::Stop)],30*time::SECOND).is_empty());}
    #[test] fn midnight_is_split_and_clipped(){let t=time::midnight(time::now());let s=Session {start:t-3600*time::SECOND,end:Some(t+7200*time::SECOND),boundary:0,reason:End::Shutdown,resumed:false};let d=days(&[s],t+7200*time::SECOND,2);assert_eq!(d[0].seconds,7200);assert_eq!(d[1].seconds,3600);}
    #[test] fn future_events_are_ignored(){let s=reconstruct(&[ev(10,Kind::Boot),ev(90,Kind::Stop)],30*time::SECOND);assert_eq!(s[0].seconds(),Some(20));}
}
