use crate::{eventlog::{self,Log},model::{self,End},time,theme::*,win::*};
use std::{sync::mpsc::{self,Receiver},mem::size_of,cell::RefCell};

const LOADING:u32=0x8001;

#[derive(Clone,Copy)]
enum Action { Range(usize), Refresh, View(usize), Day(i64), Scroll(i32) }
struct App {
    receiver:Option<Receiver<Result<Log,String>>>,log:Option<Log>,error:Option<String>,
    range:usize,view:usize,offset:usize,hits:Vec<(Rect,Action)>,
    rows:usize,page:usize,updated:i64,focus:Option<usize>,
    selected_day:Option<i64>,hover:Option<usize>,tracking_mouse:bool,
    scrollbar:Option<(Rect,i32,i32)>,drag_grab:Option<i32>,
}
impl App {
    fn new()->Self {Self {receiver:None,log:None,error:None,range:30,view:0,offset:0,hits:Vec::new(),rows:0,page:1,updated:0,focus:None,selected_day:None,hover:None,tracking_mouse:false,scrollbar:None,drag_grab:None}}
    fn load(&mut self) {
        if self.receiver.is_some(){return}
        let (tx,rx)=mpsc::channel();self.receiver=Some(rx);self.error=None;
        std::thread::spawn(move||{let result=std::panic::catch_unwind(eventlog::read).unwrap_or_else(|_|Err("로그 분석 중 오류가 발생했습니다. 새로고침으로 다시 시도해 주세요.".into()));let _=tx.send(result);});
    }
    fn count(&self,now:i64)->usize {
        if self.range!=0{return self.range}
        let earliest=self.log.as_ref().and_then(|l|l.events.iter().map(|e|e.at).min()).unwrap_or(now);
        let today=time::from_utc(time::local(time::midnight(now))).unwrap_or(now);
        let first=time::from_utc(time::local(time::midnight(earliest))).unwrap_or(earliest);
        (((today-first).max(0)/time::DAY+1) as usize).clamp(1,36600)
    }
    fn poll(&mut self)->bool {
        let result=self.receiver.as_ref().map(|r|r.try_recv());
        match result {
            Some(Ok(result))=>{self.receiver=None;match result {Ok(l)=>{self.log=Some(l);self.error=None;self.updated=time::now();},Err(e)=>self.error=Some(e)}true}
            Some(Err(mpsc::TryRecvError::Disconnected))=>{self.receiver=None;self.error=Some("로그를 읽는 작업이 중단되었습니다. 새로고침해 주세요.".into());true}
            _=>false,
        }
    }
    fn action(&mut self,hwnd:Hwnd,action:Action) {
        match action {
            Action::Range(r)=>{self.range=r;self.offset=0;self.focus=None;self.selected_day=None;}
            Action::Refresh=>self.load(),
            Action::View(v)=>{self.view=v;self.offset=0;self.focus=None;self.selected_day=None;}
            Action::Day(start)=>{self.selected_day=Some(start);self.view=1;self.offset=0;self.focus=None;}
            Action::Scroll(by)=>{self.offset=(self.offset as i64+by as i64).clamp(0,self.rows.saturating_sub(self.page) as i64) as usize;}
        }
        unsafe {InvalidateRect(hwnd,std::ptr::null(),0);}
    }
}

fn rgb(c:u32)->u32 {(c>>16&255)|(c&0xFF00)|((c&255)<<16)}
fn rect(x:i32,y:i32,w:i32,h:i32)->Rect {Rect {left:x,top:y,right:x+w,bottom:y+h}}
fn contains(r:Rect,x:i32,y:i32)->bool {x>=r.left && x<r.right && y>=r.top && y<r.bottom}
fn dialog(hwnd:Hwnd,title:&str,body:&str) {unsafe{MessageBoxW(hwnd,wide(body).as_ptr(),wide(title).as_ptr(),0x40);}}

struct Canvas {dc:Hdc,fonts:Vec<(i32,i32,Handle)>}
impl Canvas {
    fn text(&mut self,s:&str,x:i32,y:i32,w:i32,h:i32,size:i32,bold:bool,color:u32) {
        self.text_aligned(s,x,y,w,h,size,bold,color,0);
    }
    fn centered(&mut self,s:&str,x:i32,y:i32,w:i32,h:i32,size:i32,bold:bool,color:u32) {
        self.text_aligned(s,x,y,w,h,size,bold,color,1);
    }
    fn right_aligned(&mut self,s:&str,x:i32,y:i32,w:i32,h:i32,size:i32,bold:bool,color:u32) {
        self.text_aligned(s,x,y,w,h,size,bold,color,2);
    }
    fn text_aligned(&mut self,s:&str,x:i32,y:i32,w:i32,h:i32,size:i32,bold:bool,color:u32,align:u32) {
        let weight=if bold{600}else{400};
        let font=if let Some(f)=self.fonts.iter().find(|f|f.0==size && f.1==weight){f.2}else{
            let f=unsafe{CreateFontW(-size,0,0,0,weight,0,0,0,1,0,0,5,0,wide(crate::fonts::face(bold)).as_ptr())};
            self.fonts.push((size,weight,f));f
        };
        unsafe {
            let old=SelectObject(self.dc,font);SetBkMode(self.dc,1);SetTextColor(self.dc,rgb(color));
            DrawTextW(self.dc,wide(s).as_ptr(),-1,&mut rect(x,y,w,h),0x20|0x4|0x800|0x8000|align);
            SelectObject(self.dc,old);
        }
    }
    fn fill(&self,x:i32,y:i32,w:i32,h:i32,color:u32) {unsafe {let b=CreateSolidBrush(rgb(color));FillRect(self.dc,&rect(x,y,w,h),b);DeleteObject(b);}}
    fn panel(&self,x:i32,y:i32,w:i32,h:i32,color:u32,border:u32,r:i32) {unsafe {
        let b=CreateSolidBrush(rgb(color));let p=CreatePen(0,1,rgb(border));
        let ob=SelectObject(self.dc,b);let op=SelectObject(self.dc,p);
        RoundRect(self.dc,x,y,x+w,y+h,r,r);SelectObject(self.dc,op);SelectObject(self.dc,ob);DeleteObject(p);DeleteObject(b);
    }}
}
impl Drop for Canvas {fn drop(&mut self){for (_,_,h) in &self.fonts {unsafe {DeleteObject(*h);}}}}
fn button(c:&mut Canvas,app:&mut App,s:&str,r:Rect,action:Action,selected:bool) {
    let index=app.hits.len();
    let hover=app.hover==Some(index);
    c.panel(r.left,r.top,r.right-r.left,r.bottom-r.top,
        if selected{PRIMARY_SUBTLE}else if hover{0x303030}else{SURFACE},
        if app.focus==Some(index){PRIMARY_TEXT}else if selected{0x325778}else{LINE},6);
    c.centered(s,r.left+6,r.top,r.right-r.left-12,r.bottom-r.top,13,selected,if selected{PRIMARY_TEXT}else{INK});
    app.hits.push((r,action));
}
fn tab(c:&mut Canvas,app:&mut App,label:&str,r:Rect,view:usize) {
    let index=app.hits.len();
    let active=app.view==view;
    if app.hover==Some(index){c.fill(r.left,r.top,r.right-r.left,r.bottom-r.top,0x2C2C2C);}
    if app.focus==Some(index){c.panel(r.left+3,r.top+3,r.right-r.left-6,r.bottom-r.top-6,SURFACE,PRIMARY_TEXT,4);}
    c.centered(label,r.left+8,r.top,r.right-r.left-16,r.bottom-r.top,14,active,if active{INK}else{MUTED});
    if active {c.fill(r.left+12,r.bottom-2,r.right-r.left-24,2,PRIMARY_TEXT);}
    app.hits.push((r,Action::View(view)));
}
fn badge(c:&mut Canvas,label:&str,x:i32,y:i32,width:i32,kind:u8) {
    let (bg,fg)=match kind {1=>(0x223529,SUCCESS),2=>(0x3B301F,WARNING),_=>(0x303030,MUTED)};
    c.panel(x,y,width,23,bg,bg,6);
    c.centered(label,x+4,y,width-8,23,11,false,fg);
}

fn draw(_hwnd:Hwnd,app:&mut App,dc:Hdc,width:i32,height:i32,dpi:u32) {
    unsafe{SetMapMode(dc,8);SetWindowExtEx(dc,96,96,std::ptr::null_mut());SetViewportExtEx(dc,dpi as i32,dpi as i32,std::ptr::null_mut());}
    let w=width*96/dpi as i32;let h=height*96/dpi as i32;
    let mut c=Canvas {dc,fonts:Vec::new()};app.hits.clear();app.scrollbar=None;
    c.fill(0,0,w,h,BG);

    // Horizontal navbar with restrained borders and an active underline.
    c.fill(0,0,w,55,SURFACE);
    c.fill(0,54,w,1,LINE);
    c.text("PC Time",24,0,132,54,20,true,INK);
    tab(&mut c,app,"날짜별",rect(170,0,92,55),0);
    tab(&mut c,app,"세션",rect(274,0,82,55),1);
    let loading=app.receiver.is_some();
    c.right_aligned(&time::date(time::now()),w-184,0,160,54,13,false,MUTED);

    let now=time::now();
    let count=app.count(now);
    let history=app.log.as_ref().map(|l|model::reconstruct(&l.events,now)).unwrap_or_default();
    let daily=model::days(&history,now,count);
    let today=daily.first().map(|d|d.seconds).unwrap_or(0);
    let total=daily.iter().map(|d|d.seconds).sum::<i64>();
    let active=daily.iter().filter(|d|d.seconds>0).count();
    let cutoff=time::shift_date(time::midnight(now),-(count as i32-1));
    let period_sessions=history.iter().filter(|s|s.end.unwrap_or(s.boundary)>cutoff).count();

    let ranges=[(7,"7일",54),(30,"30일",62),(90,"90일",62),(365,"1년",54),(0,"전체",60)];
    let mut x=24;
    for (range,label,bw) in ranges {
        let selected=app.range==range;button(&mut c,app,label,rect(x,73,bw,32),Action::Range(range),selected);x+=bw+4;
    }
    button(&mut c,app,if loading{"불러오는 중"}else{"새로고침"},rect(w-135,73,111,32),Action::Refresh,false);

    // All four summary values share one compact surface.
    c.panel(24,123,w-48,94,SURFACE,LINE,8);
    let sw=(w-48)/4;
    let metrics=[
        ("오늘",model::duration(today)),
        ("기간 합계",model::duration(total)),
        ("일 평균",model::duration(if active>0{total/active as i64}else{0})),
        ("세션",format!("{}회",period_sessions)),
    ];
    for (i,(label,value)) in metrics.iter().enumerate() {
        let x=24+i as i32*sw;
        if i>0{c.fill(x,144,1,52,LINE);}
        c.text(label,x+20,136,sw-40,22,12,false,MUTED);
        c.text(if app.log.is_none(){"—"}else{value},x+20,163,sw-40,36,25,true,INK);
    }

    // A single conventional histogram, with fixed hour grid and no decorative cards.
    let chart_top=235;
    c.panel(24,chart_top,w-48,189,SURFACE,LINE,8);
    let n=count.min(14);
    c.text("일별 사용 시간",42,chart_top+10,240,25,14,true,INK);
    c.right_aligned(&format!("최근 {}일",n),w-154,chart_top+10,112,25,12,false,MUTED);
    let plotx=77;let plotw=w-125;let plot_top=chart_top+48;let baseline=chart_top+148;
    for (label,y) in [("24h",plot_top),("12h",plot_top+50),("0h",baseline)] {
        c.text(label,41,y-9,32,19,10,false,MUTED);
        c.fill(plotx,y,plotw,1,LINE);
    }
    let step=plotw/n as i32;let bw=(step*58/100).clamp(12,46);
    for (i,d) in daily.iter().take(n).rev().enumerate() {
        let bx=plotx+i as i32*step+(step-bw)/2;
        let bh=(d.seconds as f64/86400.0*100.0).round().clamp(0.0,100.0) as i32;
        let selected=app.selected_day==Some(d.start);
        let bar_color=if selected{PRIMARY_TEXT}else if d.start==time::midnight(now){PRIMARY}else{0x596A7B};
        let index=app.hits.len();
        if app.hover==Some(index){c.fill(plotx+i as i32*step,plot_top,step,101,0x2E2E2E);}
        if bh>0 {c.fill(bx,baseline-bh,bw,bh,bar_color);}
        let label=if d.seconds>0{format!("{}:{:02}",d.seconds/3600,d.seconds%3600/60)}else{"—".into()};
        c.centered(&label,plotx+i as i32*step,(baseline-bh-22).max(plot_top-23),step,20,10,false,if selected{PRIMARY_TEXT}else{MUTED});
        let s=time::local(d.start);
        c.centered(&format!("{:02}.{:02}",s.month,s.day),plotx+i as i32*step,baseline+10,step,20,11,false,if d.start==time::midnight(now){INK}else{MUTED});
        app.hits.push((rect(plotx+i as i32*step,plot_top-5,step,138),Action::Day(d.start)));
    }

    // The table is the primary work surface. Day details stay here in dark mode.
    let section_y=440;
    let title=if let Some(day)=app.selected_day{format!("{}  /  세션",time::date(day))}else if app.view==0{"날짜별 기록".into()}else{"전체 세션".into()};
    c.text(&title,24,section_y,450,28,15,true,INK);
    if app.selected_day.is_some(){button(&mut c,app,"전체 보기",rect(w-115,section_y-1,91,30),Action::View(1),false);}
    let top=481;let bottom=h-52;
    c.panel(24,top,w-48,(bottom-top).max(40),SURFACE,LINE,8);
    c.fill(25,top+1,w-50,33,SURFACE_ALT);
    c.fill(25,top+34,w-50,1,LINE);
    let tablew=w-96;
    let col=if app.view==0{[43,43+tablew*25/100,43+tablew*45/100,43+tablew*65/100,43+tablew*82/100]}
        else{[43,43+tablew*27/100,43+tablew*54/100,43+tablew*71/100,43+tablew*83/100]};
    let headers=if app.view==0{["날짜","시작","종료","사용 시간","상태"]}else{["시작 시각","종료 시각","사용 시간","구분","상태"]};
    for i in 0..5 {c.text(headers[i],col[i],top+3,if i<4{col[i+1]-col[i]-8}else{w-col[i]-48},28,12,false,MUTED);}
    let visible=history.iter().rev().filter(|s|{
        let end=s.end.unwrap_or(s.boundary);
        if let Some(day)=app.selected_day{s.start<time::shift_date(day,1) && end>day}else{end>cutoff}
    }).collect::<Vec<_>>();
    app.rows=if app.view==0{daily.len()}else{visible.len()};
    app.page=((bottom-top-35).max(0)/35).max(1) as usize;
    app.offset=app.offset.min(app.rows.saturating_sub(app.page));

    if let Some(error)=&app.error {
        c.text("로그를 불러오지 못했습니다",44,top+55,w-88,28,15,true,WARNING);
        for (i,line) in error.lines().take(3).enumerate(){c.text(line,44,top+89+i as i32*23,w-88,22,12,false,MUTED);}
    } else if app.log.is_none() || app.rows==0 {
        c.text(if loading{"불러오는 중…"}else{"기록 없음"},44,top+57,w-88,35,14,false,MUTED);
    } else {
        for row in 0..app.page {
            let index=app.offset+row;if index>=app.rows {break}
            let y=top+35+row as i32*35;
            if y+35>bottom{break}
            let values:[String;4];let status:String;let kind:u8;
            if app.view==0 {
                let d=&daily[index];
                let wd=["일","월","화","수","목","금","토"][time::local(d.start).weekday as usize];
                let hit=app.hits.len();
                if app.hover==Some(hit){c.fill(25,y,w-50,35,0x303030);}
                app.hits.push((rect(25,y,w-50,35),Action::Day(d.start)));
                status=if d.uncertain>0{format!("누락 {}",d.uncertain)}else if d.live{"사용 중".into()}else if d.estimated{"추정".into()}else if d.count>0{format!("{}개 세션",d.count)}else{"—".into()};
                kind=if d.uncertain>0 || d.estimated{2}else if d.live{1}else{0};
                let end=if d.live{"현재".into()}else{d.last.map(|t|if t>=time::shift_date(d.start,1){"다음 날 00:00".into()}else{time::clock(t)}).unwrap_or_else(||"—".into())};
                values=[format!("{}  {}",time::date(d.start),wd),d.first.map(time::clock).unwrap_or_else(||"—".into()),end,model::compact(d.seconds)];
            } else {
                let s=visible[index];
                values=[time::stamp(s.start),s.end.map(|t|if s.reason==End::Live{"현재".into()}else{time::stamp(t)}).unwrap_or_else(||"—".into()),s.seconds().map(model::compact).unwrap_or_else(||"—".into()),if s.resumed{"복귀".into()}else{"부팅".into()}];
                status=match s.reason{End::Unknown=>"누락",End::Crash=>"비정상",End::Live=>"사용 중",End::Sleep=>"절전",End::Shutdown=>"정상"}.into();
                kind=if s.reason==End::Unknown || s.estimated(){2}else if s.reason==End::Live{1}else{0};
            }
            for i in 0..4 {c.text(&values[i],col[i],y+2,col[i+1]-col[i]-9,31,12,i==if app.view==0{3}else{2},INK);}
            if status!="—"{badge(&mut c,&status,col[4],y+6,if app.view==0{80}else{64},kind);}
            c.fill(25,y+34,w-50,1,LINE);
        }
    }
    // Custom scrollbar, so no light Windows control can enter the dark surface.
    if app.rows>app.page {
        let x=w-34;let track_top=top+39;let track_h=(bottom-track_top-5).max(1);
        c.panel(x,track_top,4,track_h,SURFACE_ALT,SURFACE_ALT,4);
        let thumb_h=(track_h as f64*app.page as f64/app.rows as f64).round().max(20.0).min(track_h as f64) as i32;
        let progress=app.offset as f64/app.rows.saturating_sub(app.page).max(1) as f64;
        let thumb_y=track_top+((track_h-thumb_h) as f64*progress) as i32;
        c.panel(x,thumb_y,4,thumb_h,if app.drag_grab.is_some(){MUTED}else{0x606060},0x606060,4);
        app.scrollbar=Some((rect(x-5,thumb_y,14,thumb_h),track_top,track_h-thumb_h));
        app.hits.push((rect(x-5,track_top,14,(thumb_y-track_top).max(0)),Action::Scroll(-(app.page as i32))));
        app.hits.push((rect(x-5,thumb_y+thumb_h,14,(bottom-thumb_y-thumb_h).max(0)),Action::Scroll(app.page as i32)));
    }
    // Keep counts, update time and pagination in separate aligned status fields.
    c.fill(0,h-35,w,35,SURFACE);
    c.fill(0,h-35,w,1,LINE);
    let status=if let Some(l)=&app.log{
        format!("이벤트 {}건{}",l.count,if l.skipped>0{format!("  ·  해석 불가 {}건",l.skipped)}else{String::new()})
    }else if loading{"불러오는 중".into()}else{"준비".into()};
    c.text(&status,24,h-34,w-430,33,12,false,MUTED);
    if app.updated>0 {
        c.right_aligned(&format!("업데이트  {}",time::clock(app.updated)),w-390,h-34,192,33,12,false,MUTED);
    }
    if app.rows>0 {
        c.fill(w-176,h-24,1,13,LINE);
        c.right_aligned(&format!("{}–{} / {}",app.offset+1,(app.offset+app.page).min(app.rows),app.rows),w-156,h-34,132,33,12,false,MUTED);
    }
}

unsafe extern "system" fn procedure(hwnd:Hwnd,msg:u32,wp:usize,lp:isize)->isize {
    if msg==0x24 { // WM_GETMINMAXINFO
        unsafe{let m=&mut *(lp as *mut MinMax);let dpi=GetDpiForWindow(hwnd).max(96) as i32;m.min_track=Point{x:960*dpi/96,y:690*dpi/96};}return 0;
    }
    let ptr=unsafe{GetWindowLongPtrW(hwnd,-21)} as *mut RefCell<App>;
    if ptr.is_null(){return unsafe{DefWindowProcW(hwnd,msg,wp,lp)}}
    // Native calls may synchronously re-enter this callback (modal dialogs, scrollbars).
    let Ok(mut borrow)=unsafe{&*ptr}.try_borrow_mut() else {return unsafe{DefWindowProcW(hwnd,msg,wp,lp)}};
    let app=&mut *borrow;
    match msg {
        0xF=>{ // WM_PAINT, double buffered
            let mut p:Paint=unsafe{std::mem::zeroed()};let mut r=Rect::default();
            unsafe {
                let dc=BeginPaint(hwnd,&mut p);GetClientRect(hwnd,&mut r);
                if r.right>0 && r.bottom>0 {
                    let mem=CreateCompatibleDC(dc);let bitmap=CreateCompatibleBitmap(dc,r.right,r.bottom);let old=SelectObject(mem,bitmap);
                    draw(hwnd,app,mem,r.right,r.bottom,GetDpiForWindow(hwnd).max(96));
                    // Reset mapping before the physical-pixel copy.
                    SetMapMode(mem,1);BitBlt(dc,0,0,r.right,r.bottom,mem,0,0,0x00CC0020);
                    SelectObject(mem,old);DeleteObject(bitmap);DeleteDC(mem);
                }EndPaint(hwnd,&p);
            }0
        }
        0x14=>1, // WM_ERASEBKGND
        5=>{unsafe{InvalidateRect(hwnd,std::ptr::null(),0);}0}
        0x113=>{ // WM_TIMER
            let changed=if wp==2 {app.load();true}else{app.poll()};
            if changed {unsafe{InvalidateRect(hwnd,std::ptr::null(),0);}}0
        }
        0x200=>{ // WM_MOUSEMOVE
            let dpi=unsafe{GetDpiForWindow(hwnd)}.max(96) as i32;
            let x=(lp as i16 as i32)*96/dpi;let y=((lp>>16) as i16 as i32)*96/dpi;
            if let (Some(grab),Some((_,top,travel)))=(app.drag_grab,app.scrollbar) {
                let progress=(y-top-grab) as f64/travel.max(1) as f64;
                app.offset=(progress.clamp(0.0,1.0)*app.rows.saturating_sub(app.page) as f64).round() as usize;
                unsafe{InvalidateRect(hwnd,std::ptr::null(),0);}
            } else {
                let hover=app.hits.iter().position(|(r,_)|contains(*r,x,y));
                if app.hover!=hover{app.hover=hover;unsafe{InvalidateRect(hwnd,std::ptr::null(),0);}}
            }
            if !app.tracking_mouse {
                let mut track=TrackMouse {size:size_of::<TrackMouse>() as u32,flags:2,hwnd,hover:0};
                unsafe{TrackMouseEvent(&mut track);}app.tracking_mouse=true;
            }0
        }
        0x2A3=>{app.hover=None;app.tracking_mouse=false;unsafe{InvalidateRect(hwnd,std::ptr::null(),0);}0}
        0x201=>{
            let dpi=unsafe{GetDpiForWindow(hwnd)}.max(96) as i32;
            let x=(lp as i16 as i32)*96/dpi;let y=((lp>>16) as i16 as i32)*96/dpi;
            if let Some((r,_,_))=app.scrollbar {if contains(r,x,y){app.drag_grab=Some(y-r.top);unsafe{SetCapture(hwnd);}}}0
        }
        0x202=>{ // WM_LBUTTONUP
            if app.drag_grab.take().is_some(){unsafe{ReleaseCapture();InvalidateRect(hwnd,std::ptr::null(),0);}return 0}
            let dpi=unsafe{GetDpiForWindow(hwnd)}.max(96) as i32;
            let x=(lp as i16 as i32)*96/dpi;let y=((lp>>16) as i16 as i32)*96/dpi;
            if let Some((_,action))=app.hits.iter().find(|(r,_)|contains(*r,x,y)).copied(){app.action(hwnd,action);}0
        }
        0x20=>{ // WM_SETCURSOR: hand over interactive regions
            if lp&0xFFFF==1 {
                let mut p=Point::default();unsafe{GetCursorPos(&mut p);ScreenToClient(hwnd,&mut p);}
                let dpi=unsafe{GetDpiForWindow(hwnd)}.max(96) as i32;
                let hit=app.hits.iter().any(|(r,_)|contains(*r,p.x*96/dpi,p.y*96/dpi));
                unsafe{SetCursor(LoadCursorW(NULL,(if hit{32649}else{32512}) as *const u16));}return 1
            }unsafe{DefWindowProcW(hwnd,msg,wp,lp)}
        }
        0x20A=>{let delta=((wp>>16) as i16) as i32;scroll(hwnd,app,if delta>0{-3}else{3});0}
        0x215=>{app.drag_grab=None;0}
        0x100=>{ // WM_KEYDOWN
            match wp {
                0x74=>app.action(hwnd,Action::Refresh),
                0x1B=>{if app.selected_day.is_some(){app.action(hwnd,Action::View(0));}}
                0x26=>scroll(hwnd,app,-1),0x28=>scroll(hwnd,app,1),0x21=>scroll(hwnd,app,-(app.page as i32)),0x22=>scroll(hwnd,app,app.page as i32),
                0x24=>{app.offset=0;unsafe{InvalidateRect(hwnd,std::ptr::null(),0);}}
                0x23=>{app.offset=app.rows.saturating_sub(app.page);unsafe{InvalidateRect(hwnd,std::ptr::null(),0);}}
                9=>{if !app.hits.is_empty(){let backwards=unsafe{GetKeyState(0x10)}<0;let n=app.hits.len();app.focus=Some(if backwards{app.focus.map_or(n-1,|i|(i+n-1)%n)}else{app.focus.map_or(0,|i|(i+1)%n)});unsafe{InvalidateRect(hwnd,std::ptr::null(),0);}}}
                13|32=>{if let Some(action)=app.focus.and_then(|i|app.hits.get(i).map(|h|h.1)){app.action(hwnd,action);}}
                _=>{}
            }0
        }
        0x2E0=>{ // WM_DPICHANGED: use the OS suggested bounds
            let r=unsafe{&*(lp as *const Rect)};
            unsafe{SetWindowPos(hwnd,NULL,r.left,r.top,r.right-r.left,r.bottom-r.top,0x14);}0
        }
        LOADING=>{app.load();unsafe{InvalidateRect(hwnd,std::ptr::null(),0);}0}
        2=>{unsafe{KillTimer(hwnd,1);KillTimer(hwnd,2);PostQuitMessage(0);}0}
        _=>unsafe{DefWindowProcW(hwnd,msg,wp,lp)},
    }
}
#[link(name="user32")]
unsafe extern "system" {
    fn SetWindowPos(hwnd:Hwnd,after:Hwnd,x:i32,y:i32,w:i32,h:i32,flags:u32)->i32;
    fn TrackMouseEvent(track:*mut TrackMouse)->i32;
    fn SetCapture(hwnd:Hwnd)->Hwnd;
    fn ReleaseCapture()->i32;
    fn GetKeyState(key:i32)->i16;
}
#[repr(C)] struct TrackMouse {size:u32,flags:u32,hwnd:Hwnd,hover:u32}
fn scroll(hwnd:Hwnd,app:&mut App,by:i32) {
    app.offset=(app.offset as i64+by as i64).clamp(0,app.rows.saturating_sub(app.page) as i64) as usize;
    unsafe{InvalidateRect(hwnd,std::ptr::null(),0);}
}

pub fn run() {
    unsafe {
        SetProcessDpiAwarenessContext(-4isize as Handle);
        let _fonts=match crate::fonts::EmbeddedFonts::load(){Ok(fonts)=>fonts,Err(e)=>{dialog(NULL,"PC Time",&e);return}};
        let instance=GetModuleHandleW(std::ptr::null());let class_name=wide("PcTime.PowerHistory.v1");
        let class=WndClass {size:size_of::<WndClass>() as u32,style:3,proc:Some(procedure),cls_extra:0,wnd_extra:0,instance,icon:LoadIconW(NULL,32516usize as *const u16),cursor:LoadCursorW(NULL,32512usize as *const u16),background:NULL,menu:std::ptr::null(),name:class_name.as_ptr(),small_icon:NULL};
        if RegisterClassExW(&class)==0 {dialog(NULL,"PC TIME","창을 등록하지 못했습니다.");return}
        let state=Box::into_raw(Box::new(RefCell::new(App::new())));
        let hwnd=CreateWindowExW(0,class_name.as_ptr(),wide("PC Time").as_ptr(),0x00CF0000,0x80000000u32 as i32,0x80000000u32 as i32,1140,860,NULL,NULL,instance,NULL);
        if hwnd.is_null(){drop(Box::from_raw(state));dialog(NULL,"PC TIME","창을 열지 못했습니다.");return}
        SetWindowLongPtrW(hwnd,-21,state as isize);
        let dark=1i32;DwmSetWindowAttribute(hwnd,20,(&dark as *const i32).cast(),4);
        let title=rgb(SURFACE);DwmSetWindowAttribute(hwnd,35,(&title as *const u32).cast(),4);
        let text=rgb(INK);DwmSetWindowAttribute(hwnd,36,(&text as *const u32).cast(),4);
        SetTimer(hwnd,1,200,NULL);SetTimer(hwnd,2,60_000,NULL);
        ShowWindow(hwnd,5);UpdateWindow(hwnd);PostMessageW(hwnd,LOADING,0,0);
        let mut msg=Message::default();
        while GetMessageW(&mut msg,NULL,0,0)>0 {TranslateMessage(&msg);DispatchMessageW(&msg);}
        drop(Box::from_raw(state));
    }
}
