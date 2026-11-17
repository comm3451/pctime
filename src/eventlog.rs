use crate::{model::{Event,Kind},time,win::*};

pub struct Log { pub events:Vec<Event>, pub count:usize, pub skipped:usize }
struct Owned(Handle);
impl Drop for Owned {fn drop(&mut self){unsafe {EvtClose(self.0);}}}

pub fn read()->Result<Log,String> {
    // Filter by BOTH provider and event ID. IDs alone are not unique across providers.
    let query="*[System[(Provider[@Name='Microsoft-Windows-Kernel-General'] and (EventID=12 or EventID=13)) or (Provider[@Name='Microsoft-Windows-Kernel-Boot'] and EventID=27) or (Provider[@Name='Microsoft-Windows-Kernel-Power'] and (EventID=41 or EventID=42 or EventID=107 or EventID=109 or EventID=506 or EventID=507)) or (Provider[@Name='Microsoft-Windows-Power-Troubleshooter'] and EventID=1) or (Provider[@Name='EventLog'] and EventID=6008)]]";
    let h=unsafe {EvtQuery(NULL,wide("System").as_ptr(),wide(query).as_ptr(),1|0x100)};
    if h.is_null() {return Err(error("System 로그를 열 수 없습니다",unsafe {GetLastError()}));}
    let query=Owned(h);
    let mut log=Log {events:Vec::new(),count:0,skipped:0};
    loop {
        let mut events=[NULL;64];let mut returned=0;
        if unsafe {EvtNext(query.0,64,events.as_mut_ptr(),5000,0,&mut returned)}==0 {
            let code=unsafe {GetLastError()};
            if code==259 {break}return Err(error("이벤트를 읽지 못했습니다",code));
        }
        // Own the whole batch before rendering, so every handle closes on an early error.
        let owned=events[..returned as usize].iter().map(|h|Owned(*h)).collect::<Vec<_>>();
        for event in owned {
            log.count+=1;
            let mut used=0;let mut properties=0;
            unsafe {EvtRender(NULL,event.0,1,0,NULL,&mut used,&mut properties);}
            let code=unsafe {GetLastError()};
            if code!=122 || used==0 {return Err(error("이벤트 XML 크기를 읽지 못했습니다",code));}
            let mut buf=vec![0u16;(used as usize).div_ceil(2)];
            if unsafe {EvtRender(NULL,event.0,1,(buf.len()*2) as u32,buf.as_mut_ptr().cast(),&mut used,&mut properties)}==0 {return Err(error("이벤트 XML을 읽지 못했습니다",unsafe {GetLastError()}));}
            let end=buf.iter().position(|c|*c==0).unwrap_or(buf.len());
            let xml=String::from_utf16_lossy(&buf[..end]);
            match parse(&xml) {Some(mut e)=>log.events.append(&mut e),None=>log.skipped+=1}
        }
    }
    normalize(&mut log.events);
    Ok(log)
}
fn error(action:&str,code:u32)->String {
    if code==5 {format!("{action} (Windows 오류 5: 접근 거부).\nPC TIME을 마우스 오른쪽 버튼으로 눌러 '관리자 권한으로 실행'해 주세요.")}
    else {format!("{action} (Windows 오류 {code}).\n이벤트 로그 서비스와 System 로그의 읽기 권한을 확인해 주세요.")}
}

fn element<'a>(xml:&'a str,name:&str)->Option<&'a str> {
    let needle=format!("<{name}");let mut from=0;
    while let Some(p)=xml[from..].find(&needle) {
        let at=from+p;let next=*xml.as_bytes().get(at+needle.len())?;
        if next==b'>' || next.is_ascii_whitespace() {
            let begin=xml[at..].find('>')?+at+1;
            let end=xml[begin..].find(&format!("</{name}>"))?+begin;
            return Some(&xml[begin..end]);
        }
        from=at+needle.len();
    }None
}
fn attribute<'a>(xml:&'a str,tag:&str,attr:&str)->Option<&'a str> {
    let begin=xml.find(&format!("<{tag}"))?;
    let end=xml[begin..].find('>')?+begin;
    attr_value(&xml[begin..end],attr)
}
fn attr_value<'a>(tag:&'a str,attr:&str)->Option<&'a str> {
    let p=tag.find(&format!("{attr}="))?+attr.len()+1;
    let quote=*tag.as_bytes().get(p)? as char;
    if quote!='\'' && quote!='"' {return None}
    let s=&tag[p+1..];Some(&s[..s.find(quote)?])
}
fn data<'a>(xml:&'a str,name:&str)->Option<&'a str> {
    let mut remaining=xml;
    while let Some(at)=remaining.find("<Data ") {
        remaining=&remaining[at..];let end=remaining.find('>')?;
        if attr_value(&remaining[..end],"Name")==Some(name) {
            return Some(&remaining[end+1..][..remaining[end+1..].find("</Data>")?]);
        }remaining=&remaining[end+1..];
    }None
}
fn parse(xml:&str)->Option<Vec<Event>> {
    let id=element(xml,"EventID")?.parse::<u32>().ok()?;
    let record=element(xml,"EventRecordID")?.parse().ok()?;
    let provider=attribute(xml,"Provider","Name")?;
    let at=time::parse(attribute(xml,"TimeCreated","SystemTime")?)?;
    let mut e=Event {at,kind:Kind::Boot,id,record};
    match (provider,id) {
        ("Microsoft-Windows-Kernel-General",12)=>{e.at=data(xml,"StartTime").and_then(time::parse).unwrap_or(at);}
        ("Microsoft-Windows-Kernel-General",13)=>{e.kind=Kind::Stop;e.at=data(xml,"StopTime").and_then(time::parse).unwrap_or(at);}
        ("Microsoft-Windows-Kernel-Boot",27)=>{
            // BootType 0 is already represented by Kernel-General 12; 2 is a hibernate resume.
            match data(xml,"BootType")? {"1"|"0x1"=>{},"2"|"0x2"=>e.kind=Kind::Wake,_=>return Some(Vec::new())}
        }
        ("Microsoft-Windows-Kernel-Power",109)=>e.kind=Kind::Stop,
        ("Microsoft-Windows-Kernel-Power",42|506)=>e.kind=Kind::Sleep,
        ("Microsoft-Windows-Kernel-Power",107|507)=>e.kind=Kind::Wake,
        ("Microsoft-Windows-Kernel-Power",41)=>e.kind=Kind::UnexpectedBoot,
        ("Microsoft-Windows-Power-Troubleshooter",1)=>{
            e.kind=Kind::Wake;e.at=data(xml,"WakeTime").and_then(time::parse).unwrap_or(at);
            if let Some(sleep)=data(xml,"SleepTime").and_then(time::parse).filter(|t|*t<e.at) {
                return Some(vec![Event {at:sleep,kind:Kind::Sleep,id,record},e]);
            }
        }
        ("EventLog",6008)=>{
            // Binary: local SYSTEMTIME, then UTC SYSTEMTIME of the previous unexpected stop.
            // Never use TimeCreated: the event itself is emitted at the NEXT boot.
            e.kind=Kind::CrashStop;e.at=crash_time(element(xml,"Binary")?)?;
            if e.at>at || at-e.at>366*time::DAY {return None}
        }
        _=>return Some(Vec::new()),
    }
    Some(vec![e])
}
fn crash_time(hex:&str)->Option<i64> {
    let h=hex.trim();if h.len()<64 || !h.is_ascii() {return None}
    let bytes=(32..64).step_by(2).map(|n|u8::from_str_radix(&h[n..n+2],16).ok()).collect::<Option<Vec<_>>>()?;
    let word=|n:usize|u16::from_le_bytes([bytes[n*2],bytes[n*2+1]]);
    time::from_utc(SystemTime {year:word(0),month:word(1),weekday:word(2),day:word(3),hour:word(4),minute:word(5),second:word(6),millis:word(7)})
}
fn normalize(events:&mut Vec<Event>) {
    events.sort_by_key(|e|(e.at,e.record));
    let exact=events.iter().filter(|e|e.id==13).map(|e|e.at).collect::<Vec<_>>();
    let boots=events.iter().filter(|e|e.id==12).map(|e|e.at).collect::<Vec<_>>();
    events.retain(|e|{
        let near=|list:&[i64]|{let p=list.partition_point(|t|*t<e.at-2*time::MINUTE);list.get(p).is_some_and(|t|(*t-e.at).abs()<=2*time::MINUTE)};
        !(e.id==109 && near(&exact) || e.id==27 && e.kind==Kind::Boot && near(&boots))
    });
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn event_id_attributes(){assert_eq!(element("<EventID Qualifiers='32768'>6008</EventID>","EventID"),Some("6008"));}
    #[test] fn both_quote_styles(){assert_eq!(attribute("<Provider Name=\"EventLog\"/>","Provider","Name"),Some("EventLog"));assert_eq!(data("<Data Name='BootType'>1</Data>","BootType"),Some("1"));}
    #[test] fn crash_uses_previous_utc_time(){let t=crash_time("E7070C00060002000D0038003200A301E7070C00060002000C0038003200A301600900003C000000").unwrap();assert_eq!(t,time::parse("2023-12-02T12:56:50.419Z").unwrap());}
    #[test] fn invalid_crash_is_not_next_boot(){assert!(crash_time("garbage").is_none());}
    #[test] fn shutdown_fallback_removed_near_exact(){let mut e=vec![Event {at:100*time::SECOND,kind:Kind::Stop,id:109,record:1},Event {at:101*time::SECOND,kind:Kind::Stop,id:13,record:2}];normalize(&mut e);assert_eq!(e.len(),1);assert_eq!(e[0].id,13);}
}
