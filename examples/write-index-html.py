#!/usr/bin/env python3
"""Write full-featured index.html to data repo with all requested features."""

import os

DATA = "G:/i2c/PROJECTS/MiniPlatform/MiniDi/Data/minidi-vn-data"

# The HTML is built in segments to manage complexity
CSS = """
:root{--bg:#070b17;--bg2:#0f172a;--bg3:#1e293b;--a1:#6366f1;--a2:#06b6d4;--a3:#a78bfa;--a1d:rgba(99,102,241,.15);--text:#f1f5f9;--text2:#94a3b8;--text3:#64748b;--ok:#22c55e;--warn:#eab308;--sm:6px;--md:10px;--lg:16px;--xl:24px;--tr:.3s cubic-bezier(.4,0,.2,1);--hdr:56px}
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:'Inter',sans-serif;background:var(--bg);color:var(--text);min-height:100vh;overflow-x:hidden}
::-webkit-scrollbar{width:5px}::-webkit-scrollbar-track{background:transparent}::-webkit-scrollbar-thumb{background:var(--text3);border-radius:3px}
a{color:var(--a1);text-decoration:none}
/* Splash */
#sp{z-index:999;position:fixed;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;background:var(--bg);transition:opacity .8s,transform .8s}
#sp.hide{opacity:0;transform:scale(1.05);pointer-events:none}
.sp-ring{width:80px;height:80px;border-radius:50%;border:3px solid var(--bg3);border-top-color:var(--a1);border-right-color:var(--a2);animation:spin 1s linear infinite;margin-bottom:24px}
@keyframes spin{to{transform:rotate(360deg)}}
.sp-logo{font-size:2.5rem;font-weight:800;background:linear-gradient(135deg,var(--a1),var(--a2),var(--a3));-webkit-background-clip:text;-webkit-text-fill-color:transparent;margin-bottom:8px}
.sp-sub{color:var(--text3);font-size:.9rem;margin-bottom:20px}
.track{width:240px;height:4px;background:var(--bg3);border-radius:2px;overflow:hidden}
.bar{height:100%;width:0%;background:linear-gradient(90deg,var(--a1),var(--a2));border-radius:2px;transition:width .3s}
.pct{color:var(--text3);font-size:.75rem;margin-top:8px}
/* Header */
.hdr{display:flex;align-items:center;gap:12px;padding:0 16px;height:var(--hdr);background:rgba(7,11,23,.88);backdrop-filter:blur(16px);-webkit-backdrop-filter:blur(16px);border-bottom:1px solid rgba(255,255,255,.07);position:sticky;top:0;z-index:50}
.logo{display:flex;align-items:center;gap:8px;font-weight:700;font-size:1rem;flex-shrink:0;cursor:pointer}
.logo-i{width:28px;height:28px;background:linear-gradient(135deg,var(--a1),var(--a2));border-radius:6px;display:inline-flex;align-items:center;justify-content:center;font-size:1rem}
.srch{flex:1;max-width:400px;position:relative}
.srch input{width:100%;padding:7px 12px 7px 32px;background:var(--bg3);border:1px solid rgba(255,255,255,.07);border-radius:var(--md);color:var(--text);font-size:.85rem;outline:none;transition:var(--tr)}
.srch input:focus{border-color:var(--a1);box-shadow:0 0 16px var(--a1d)}
.srch input::placeholder{color:var(--text3)}
.srch-i{position:absolute;left:10px;top:50%;transform:translateY(-50%);color:var(--text3);font-size:.8rem}
/* View tabs */
.tabs{display:flex;gap:2px;background:var(--bg3);border-radius:var(--sm);padding:2px;flex-shrink:0}
.tab{padding:5px 10px;border:none;border-radius:4px;background:transparent;color:var(--text3);cursor:pointer;font-size:.78rem;transition:var(--tr);white-space:nowrap}
.tab:hover{color:var(--text2)}
.tab.act{background:var(--a1);color:#fff}
.tab .badge{font-size:.6rem;background:rgba(255,255,255,.15);padding:0 5px;border-radius:8px;margin-left:3px}
.lang-w{display:flex;background:var(--bg3);border-radius:var(--sm);overflow:hidden;flex-shrink:0}
.lang-btn{padding:4px 9px;border:none;background:transparent;color:var(--text3);cursor:pointer;font-size:.75rem;transition:var(--tr)}
.lang-btn.act{background:var(--a1);color:#fff}
/* Hero */
.hero{position:relative;text-align:center;padding:48px 20px 32px;overflow:hidden;min-height:360px;display:flex;flex-direction:column;align-items:center;justify-content:center}
.hero-bg{position:absolute;inset:0;z-index:0;overflow:hidden}
.h-slide{position:absolute;inset:0;background-size:cover;background-position:center;opacity:0;transition:opacity 1.5s;filter:brightness(.32) saturate(.8)}
.h-slide.act{opacity:1}
.h-over{position:absolute;inset:0;background:linear-gradient(180deg,transparent 30%,var(--bg) 85%)}
.h-cnt{position:relative;z-index:1;max-width:680px}
.h-cnt h1{font-size:2.5rem;font-weight:800;margin-bottom:10px}
.grad{background:linear-gradient(135deg,var(--a1),var(--a2),var(--a3));-webkit-background-clip:text;-webkit-text-fill-color:transparent}
.h-cnt p{color:var(--text2);line-height:1.5;font-size:.9rem;margin-bottom:6px}
.stats{display:flex;justify-content:center;gap:28px;flex-wrap:wrap;margin-top:16px}
.stat{text-align:center}
.stat .num{font-size:1.5rem;font-weight:700;background:linear-gradient(135deg,var(--a1),var(--a2));-webkit-background-clip:text;-webkit-text-fill-color:transparent}
.stat .lbl{color:var(--text3);font-size:.75rem;margin-top:1px}
.sl-dots{display:flex;gap:5px;margin-top:12px;position:relative;z-index:1}
.sl-dot{width:7px;height:7px;border-radius:50%;background:rgba(255,255,255,.3);cursor:pointer;transition:var(--tr)}
.sl-dot.act{background:#fff;transform:scale(1.2)}
/* Container */
.cont{max-width:860px;margin:0 auto;padding:0 16px}
/* Views */
.view{display:none}
.view.act{display:block}
/* Search */
.fbar{display:flex;gap:5px;flex-wrap:wrap;justify-content:center;margin:8px 0}
.fbtn{padding:3px 12px;border:1px solid rgba(255,255,255,.07);border-radius:20px;background:transparent;color:var(--text2);cursor:pointer;font-size:.74rem;transition:var(--tr)}
.fbtn:hover{border-color:var(--a1);color:var(--a1)}
.fbtn.act{background:var(--a1d);border-color:var(--a1);color:var(--a1)}
.ebar{display:flex;gap:4px;overflow-x:auto;padding:5px 0;margin:6px 0}
.ebtn{flex-shrink:0;padding:3px 10px;border:1px solid rgba(255,255,255,.07);border-radius:20px;background:transparent;color:var(--text3);cursor:pointer;font-size:.68rem;white-space:nowrap;transition:var(--tr)}
.ebtn:hover{border-color:var(--a2);color:var(--a2)}
.ebtn.act{background:var(--a2);border-color:var(--a2);color:#fff}
/* Alphabet bar */
.alpha{display:flex;gap:2px;flex-wrap:wrap;justify-content:center;margin:6px 0;padding:4px;background:var(--bg3);border-radius:var(--sm)}
.alpha-btn{width:28px;height:28px;border:none;border-radius:4px;background:transparent;color:var(--text3);cursor:pointer;font-size:.72rem;font-weight:500;transition:var(--tr)}
.alpha-btn:hover{background:var(--a1d);color:var(--a1)}
.alpha-btn.act{background:var(--a1);color:#fff}
/* Cards */
.card{background:rgba(255,255,255,.03);border:1px solid rgba(255,255,255,.07);border-radius:var(--md);padding:12px 16px;margin-bottom:6px;cursor:pointer;transition:var(--tr)}
.card:hover{background:rgba(255,255,255,.06);border-color:rgba(255,255,255,.12);transform:translateY(-1px)}
.ch{display:flex;align-items:center;gap:7px;flex-wrap:wrap;margin-bottom:2px}
.ct{font-size:.92rem;font-weight:600;color:var(--a1)}
.ct-vi{font-size:.75rem;color:var(--text3);font-style:italic}
.badge{font-size:.6rem;padding:1px 8px;border-radius:20px;font-weight:500}
.badge-place{background:rgba(34,197,94,.15);color:var(--ok)}
.badge-person{background:rgba(99,102,241,.15);color:var(--a1)}
.badge-event{background:rgba(234,179,8,.15);color:var(--warn)}
.badge-other{background:var(--bg3);color:var(--text3)}
.cd{color:var(--text2);font-size:.8rem;line-height:1.5;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}
.cn{font-size:.65rem;color:var(--text3);margin-top:2px}
/* Map */
#map{width:100%;height:500px;border-radius:var(--md);margin:10px 0;z-index:1}
.map-leg{display:flex;gap:12px;justify-content:center;flex-wrap:wrap;font-size:.75rem;color:var(--text2);margin:6px 0}
.map-leg span{display:flex;align-items:center;gap:4px}
.mdot{width:10px;height:10px;border-radius:50%;display:inline-block}
/* Timeline */
.tl-wrap{overflow-x:auto;padding:16px 0;position:relative;margin:10px 0}
.tl-c{position:relative;height:200px;min-width:300%}
.tl-ax{position:absolute;top:50%;left:0;right:0;height:2px;background:var(--bg3)}
.tl-ms{position:absolute;top:50%;transform:translateY(-50%);font-size:.6rem;color:var(--text3);white-space:nowrap}
.tl-ms::before{content:'';position:absolute;top:-8px;left:50%;width:1px;height:16px;background:var(--bg3)}
.tl-evt{position:absolute;cursor:pointer;transition:var(--tr)}
.tl-evt:hover{transform:scale(1.3);z-index:10}
.tl-dot{width:10px;height:10px;border-radius:50%;position:absolute;top:50%;left:50%;transform:translate(-50%,-50%)}
.tl-lb{position:absolute;top:-18px;left:50%;transform:translateX(-50%);font-size:.6rem;color:var(--text3);white-space:nowrap;display:none}
.tl-evt:hover .tl-lb{display:block}
.tl-zm{display:flex;gap:6px;justify-content:center;margin:6px 0}
.tl-zm button{padding:4px 12px;border:1px solid rgba(255,255,255,.07);border-radius:var(--sm);background:var(--bg3);color:var(--text2);cursor:pointer;font-size:.75rem;transition:var(--tr)}
.tl-zm button:hover{border-color:var(--a1)}
/* Tree */
.tree-wrap{overflow-x:auto;padding:16px 0;margin:10px 0}
.tree-c{position:relative;display:flex;gap:40px;justify-content:center;min-height:400px;padding-top:40px}
.tree-n{display:flex;flex-direction:column;align-items:center;gap:4px;position:relative;cursor:pointer;transition:var(--tr)}
.tree-n:hover{transform:scale(1.05)}
.tree-n .tn-av{width:48px;height:48px;border-radius:50%;background:var(--bg3);border:2px solid rgba(255,255,255,.1);display:flex;align-items:center;justify-content:center;font-size:1.3rem;overflow:hidden}
.tree-n .tn-lb{font-size:.7rem;color:var(--text2);text-align:center;max-width:100px}
.tree-n .tn-sub{font-size:.6rem;color:var(--text3)}
.tree-ln{position:absolute;width:2px;background:var(--bg3)}
.tree-ch{display:flex;gap:30px;margin-top:60px;position:relative}
.tree-ch::before{content:'';position:absolute;top:-30px;left:50%;width:2px;height:30px;background:var(--bg3)}
/* Chat */
.chat-w{position:fixed;bottom:20px;right:20px;z-index:90;display:flex;flex-direction:column;align-items:flex-end;gap:8px}
.chat-b{width:50px;height:50px;border-radius:50%;border:none;background:linear-gradient(135deg,var(--a1),var(--a2));color:#fff;font-size:1.3rem;cursor:pointer;box-shadow:0 4px 20px rgba(99,102,241,.4);transition:var(--tr)}
.chat-b:hover{transform:scale(1.1)}
.chat-p{width:340px;max-height:460px;background:var(--bg2);border:1px solid rgba(255,255,255,.07);border-radius:var(--lg);overflow:hidden;display:none;flex-direction:column;box-shadow:0 8px 40px rgba(0,0,0,.5)}
.chat-p.open{display:flex}
.chat-h{padding:12px 16px;border-bottom:1px solid rgba(255,255,255,.07);display:flex;justify-content:space-between;align-items:center}
.chat-h span{font-weight:600;font-size:.85rem}
.chat-h button{background:none;border:none;color:var(--text3);cursor:pointer;font-size:1.1rem}
.chat-msgs{flex:1;overflow-y:auto;padding:12px 16px;min-height:200px;max-height:300px}
.cm{font-size:.8rem;line-height:1.5;margin-bottom:8px;padding:8px 12px;border-radius:var(--md);max-width:85%}
.cm.bot{background:var(--bg3);color:var(--text2);align-self:flex-start}
.cm.user{background:var(--a1d);color:var(--text);margin-left:auto}
.chat-inp{display:flex;padding:8px 12px;gap:6px;border-top:1px solid rgba(255,255,255,.07)}
.chat-inp input{flex:1;padding:7px 10px;background:var(--bg3);border:1px solid rgba(255,255,255,.07);border-radius:var(--sm);color:var(--text);font-size:.8rem;outline:none}
.chat-inp input:focus{border-color:var(--a1)}
.chat-inp button{padding:7px 14px;border:none;border-radius:var(--sm);background:var(--a1);color:#fff;cursor:pointer;font-size:.8rem}
.chat-sug{display:flex;gap:4px;flex-wrap:wrap;padding:8px 12px}
.chat-sug button{font-size:.68rem;padding:3px 10px;border:1px solid rgba(255,255,255,.07);border-radius:20px;background:transparent;color:var(--text3);cursor:pointer}
.chat-sug button:hover{border-color:var(--a1);color:var(--a1)}
/* Detail overlay */
.overlay{position:fixed;inset:0;background:rgba(0,0,0,.7);backdrop-filter:blur(8px);-webkit-backdrop-filter:blur(8px);z-index:100;display:none;align-items:center;justify-content:center}
.overlay.open{display:flex}
.panel{background:var(--bg2);border:1px solid rgba(255,255,255,.07);border-radius:var(--xl);padding:28px;max-width:640px;width:92%;max-height:88vh;overflow-y:auto;position:relative}
.p-close{position:absolute;top:12px;right:16px;font-size:1.4rem;cursor:pointer;color:var(--text3);background:none;border:none;z-index:2}
.p-close:hover{color:var(--text)}
.p-img{width:80px;height:80px;border-radius:50%;background:var(--bg3);display:flex;align-items:center;justify-content:center;font-size:2rem;margin-bottom:10px;overflow:hidden;float:right}
.p-img img{width:100%;height:100%;object-fit:cover}
.p-title{font-size:1.3rem;font-weight:700}
.p-title-vi{font-size:.9rem;color:var(--text3);margin-bottom:4px}
.p-desc{color:var(--text2);line-height:1.6;margin:10px 0;font-size:.88rem;clear:both}
.p-meta{display:grid;grid-template-columns:1fr 1fr;gap:4px;font-size:.78rem;margin:10px 0}
.p-meta dt{color:var(--text3)}.p-meta dd{color:var(--text);word-break:break-all}
.p-live{border:1px dashed var(--a2);border-radius:var(--md);padding:10px;margin:8px 0;background:var(--a2);font-size:.78rem}
.p-live h4{color:var(--a2);margin-bottom:4px;font-size:.82rem}
.p-live .pld{color:var(--text2);line-height:1.5}
.p-sec{margin-top:10px}
.p-sec h3{font-size:.82rem;color:var(--a2);margin-bottom:5px}
.p-rel{display:flex;align-items:center;gap:4px;padding:2px 0;font-size:.76rem;color:var(--text);cursor:pointer;transition:var(--tr)}
.p-rel:hover{color:var(--a1)}
.rarr{color:var(--text3);margin:0 3px}
/* Footer */
.ftr{text-align:center;padding:24px 16px;color:var(--text3);font-size:.76rem;border-top:1px solid rgba(255,255,255,.07);margin-top:32px;line-height:1.8}
.ftr a{color:var(--a1)}
.ftr .emoji{font-size:1.1rem}
.ftr-l{display:flex;gap:16px;justify-content:center;flex-wrap:wrap;margin-bottom:8px}
.ftr-b{display:inline-block;padding:4px 12px;background:var(--bg3);border-radius:var(--sm);margin:2px}
@media(max-width:768px){
  .h-cnt h1{font-size:1.5rem}
  .hero{min-height:260px;padding:32px 16px 20px}
  .stats{gap:12px}
  .stat .num{font-size:1.1rem}
  .panel{padding:20px}
  .p-meta{grid-template-columns:1fr}
  .tabs{overflow-x:auto}
  .tab{font-size:.7rem;padding:4px 7px}
  .srch{max-width:160px}
  .chat-p{width:300px;right:10px}
  #map{height:300px}
}
"""

HTML_HEAD = (
    r"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>MiniDi — Vietnam Knowledge Graph</title>
<meta name="description" content="Kham pha du lieu WikiData ve Viet Nam: lich su, dia ly, nhan vat, van hoa.">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&display=swap" rel="stylesheet">
<link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
<script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
<style>"""
    + CSS
    + r"""</style>
</head>
<body>
"""
)

SPLASH = r"""<div id=sp>
  <div class=sp-logo>&#x1F1FB;&#x1F1F3; MiniDi</div>
  <div class=sp-ring></div>
  <div class=sp-sub id=sp-msg>Loading Vietnam knowledge graph...</div>
  <div class=track><div class=bar id=bar></div></div>
  <div class=pct id=pct>0%</div>
</div>"""

HEADER = r"""<div class=hdr>
  <div class=logo onclick="showView('search')"><span class=logo-i>&#x1F577;</span><span>MiniDi</span></div>
  <div class=srch><span class=srch-i>&#x1F50D;</span><input type=text id=si placeholder="SEARCH" oninput=onSearch()></div>
  <div class=tabs id=tabs>
    <button class="tab act" onclick="showView('search')" title=Search>&#x1F50D;</button>
    <button class=tab onclick="showView('map')" title=Map>&#x1F5FA;</button>
    <button class=tab onclick="showView('timeline')" title=Timeline>&#x1F4C5;</button>
    <button class=tab onclick="showView('tree')" title="Dynasty Tree">&#x1F333;</button>
  </div>
  <div class=lang-w>
    <button class="lang-btn act" id=le onclick="setL('en')">EN</button>
    <button class=lang-btn id=lv onclick="setL('vi')">VI</button>
  </div>
  <a href=https://github.com/minidivn/minidi-vn-data target=_blank style=color:var(--text3);font-size:1.2rem title="GitHub">&#x1F5D2;</a>
</div>"""

HERO = r"""<div class=hero id=hero>
  <div class=hero-bg id=hero-bg></div>
  <div class=h-over></div>
  <div class=h-cnt>
    <h1><span class=grad>&#x1F1FB;&#x1F1F3; Vietnam Knowledge Graph</span></h1>
    <p id=hs>Exploring WikiData entities: history, geography, people & culture</p>
    <div class=stats>
      <div class=stat><div class=num id=st-t>--</div><div class=lbl data-en=Entities data-vi="Thuc the">Entities</div></div>
      <div class=stat><div class=num id=st-e>--</div><div class=lbl data-en=Events data-vi="Su kien">Events</div></div>
      <div class=stat><div class=num id=st-p>--</div><div class=lbl data-en=People data-vi="Nhan vat">People</div></div>
      <div class=stat><div class=num id=st-pl>--</div><div class=lbl data-en=Places data-vi="Dia danh">Places</div></div>
    </div>
    <div class=sl-dots id=sl-dots></div>
  </div>
</div>"""

VIEWS = r"""<div class=cont>
<div id=v-search class="view act">
  <div class=fbar>
    <button class="fbtn act" onclick="setF('all')" data-en=All data-vi="Tat ca">All</button>
    <button class=fbtn onclick="setF('place')" data-en=Places data-vi="Dia danh">&#x1F4CD; Places</button>
    <button class=fbtn onclick="setF('person')" data-en=People data-vi="Nhan vat">&#x1F464; People</button>
    <button class=fbtn onclick="setF('event')" data-en=Events data-vi="Su kien">&#x1F4C5; Events</button>
  </div>
  <div class=alpha id=alpha></div>
  <div class=ebar>
    <button class=ebtn onclick="setE('hong_bang')">&#x1F3EE; Hong Bang</button>
    <button class=ebtn onclick="setE('chinese_domination')">&#x1F4DC; Chinese Dom.</button>
    <button class=ebtn onclick="setE('dynastic_vn')">&#x1F3EF; Dynastic</button>
    <button class=ebtn onclick="setE('colonial')">&#x1F3DB; Colonial</button>
    <button class=ebtn onclick="setE('vietnam_war')">&#x2694; Vietnam War</button>
    <button class=ebtn onclick="setE('modern')">&#x1F3D9; Modern</button>
  </div>
  <div class=st id=st></div>
  <div id=r></div>
  <div id=pg></div>
</div>

<div id=v-map class=view>
  <div class=map-leg>
    <span><span class=mdot style=background:var(--ok)></span> Places</span>
    <span><span class=mdot style=background:var(--a1)></span> People</span>
    <span><span class=mdot style=background:var(--warn)></span> Events</span>
  </div>
  <div id=map></div>
  <div style=text-align:center;font-size:.75rem;color:var(--text3)>&#x1F5FA; Click markers to view entity details</div>
</div>

<div id=v-timeline class=view>
  <div class=tl-zm>
    <button onclick="tlZoom(-1)">&#x2795; Zoom In</button>
    <button onclick="tlZoom(1)">&#x2796; Zoom Out</button>
    <button onclick="tlReset()">&#x1F504; Reset</button>
  </div>
  <div class=tl-wrap id=tl-wrap>
    <div class=tl-c id=tl-c></div>
  </div>
  <div style=text-align:center;font-size:.75rem;color:var(--text3) id=tl-info>Scroll to explore centuries</div>
</div>

<div id=v-tree class=view>
  <div class=tree-wrap id=tw>
    <div class=tree-c id=tc></div>
  </div>
  <div style=text-align:center;font-size:.75rem;color:var(--text3);margin-top:8px>&#x1F333; Click on a ruler to see details</div>
</div>
</div>"""

DETAIL_OVERLAY = r"""<div class=overlay id=ol onclick="closeD(event)">
  <div class=panel><button class=p-close onclick="closeD()">&times;</button><div id=dc></div></div>
</div>"""

CHAT = r"""<div class=chat-w>
  <div class=chat-p id=cp>
    <div class=chat-h><span>&#x1F916; MiniDi Assistant</span><button onclick="toggleChat()">&times;</button></div>
    <div class=chat-msgs id=cm></div>
    <div class=chat-sug id=cs>
      <button onclick="chatSend('What happened in 1789?')">&#x1F4D6; 1789 events</button>
      <button onclick="chatSend('List Nguyen dynasty kings')">&#x1F451; Nguyen kings</button>
      <button onclick="chatSend('Show me battles')">&#x2694; Battles</button>
      <button onclick="chatSend('How many entities?')">&#x1F4CA; Stats</button>
    </div>
    <div class=chat-inp>
      <input type=text id=ci placeholder="Ask about Vietnam..." onkeydown="if(event.key==='Enter')chatSend()">
      <button onclick="chatSend()">&#x27A1;</button>
    </div>
  </div>
  <button class=chat-b id=cb onclick="toggleChat()">&#x1F4AC;</button>
</div>"""

FOOTER = r"""<div class=ftr>
  <div class=ftr-l>
    <span class=ftr-b>&#x1F4E6; <span data-en="Entities:" data-vi="Thuc the:">Entities:</span> <strong id=ft-e>--</strong></span>
    <span class=ftr-b>&#x1F517; <span data-en="Edges:" data-vi="Quan he:">Edges:</span> <strong id=ft-ed>--</strong></span>
    <span class=ftr-b>&#x1F4C5; <span data-en="Sources:" data-vi="Nguon:">Sources:</span> WikiData</span>
    <span class=ftr-b>&#x1F310; <span data-en="Version:" data-vi="Phien ban:">Version:</span> v1</span>
  </div>
  <div>
    &#x1F1FB;&#x1F1F3; <span data-en="Built with" data-vi="Xay dung voi">Built with</span>
    <a href=https://github.com/minidivn/minidi-spider>&#x1F577; minidi-spider</a> &middot;
    <a href=https://github.com/minidivn/minidi-vn-data>&#x1F5D2; source</a> &middot;
    <a href=https://query.wikidata.org/sparql>&#x1F310; SPARQL</a> &middot;
    <a href=https://minidi-graphql.i2cvnco.workers.dev/>&#x26A1; GraphQL</a>
  </div>
  <div style=margin-top:6px;color:var(--text3)>
    <span class=emoji>&#x00A9;</span> <span data-en="Data: WikiData contributors (CC0)" data-vi="Du lieu: WikiData (CC0)">Data: WikiData contributors (CC0)</span>
  </div>
</div>"""

SCRIPT = r"""<script>
// ── Slides ──
var SLIDES = [
  'https://upload.wikimedia.org/wikipedia/commons/thumb/0/0a/Ha_Long_Bay_%282%29.jpg/1200px-Ha_Long_Bay_%282%29.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/d/d4/2018_Vietnam_Rice_Terrace_Fields.jpg/1200px-2018_Vietnam_Rice_Terrace_Fields.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/5/50/Saigon_at_night.jpg/1200px-Saigon_at_night.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/0/02/Hue_Citadel.jpg/1200px-Hue_Citadel.jpg'
];
var si=0,sti=null;
function initSlides(){var bg=document.getElementById('hero-bg');SLIDES.forEach(function(u,i){var img=new Image();img.onload=function(){var d=document.createElement('div');d.className='h-slide'+(i===0?' act':'');d.style.backgroundImage='url('+u+')';bg.appendChild(d);var dot=document.createElement('div');dot.className='sl-dot'+(i===0?' act':'');dot.onclick=function(){goSl(i)};document.getElementById('sl-dots').appendChild(dot)};img.src=u})}
function goSl(i){si=i;document.querySelectorAll('.h-slide').forEach(function(s,idx){s.classList.toggle('act',idx===i)});document.querySelectorAll('.sl-dot').forEach(function(d,idx){d.classList.toggle('act',idx===i)})}
function nextSl(){goSl((si+1)%SLIDES.length)}

// ── State ──
var IX=null,_F='all',_E=null,_L='en',_P=0,PS=40,bm=null,MA=null,TLZ=1;

// ── Load ──
function loadData(){
  var pr=document.getElementById('bar'),pp=document.getElementById('pct'),pm=document.getElementById('sp-msg');
  var xhr=new XMLHttpRequest();xhr.open('GET','index.json',true);xhr.responseType='arraybuffer';
  xhr.onprogress=function(e){if(e.lengthComputable){var p=Math.round(e.loaded/e.total*100);pr.style.width=p+'%';pp.textContent=p+'% ('+(e.loaded/1048576).toFixed(1)+'/'+(e.total/1048576).toFixed(1)+' MB)';pm.textContent=p<30?'Downloading Vietnam knowledge graph...':'Parsing entity data...'}};
  xhr.onload=function(){try{IX=JSON.parse(new TextDecoder('utf-8').decode(xhr.response));pr.style.width='100%';pp.textContent='100%';pm.textContent=IX.meta.entity_count+' entities loaded!';setTimeout(function(){document.getElementById('sp').classList.add('hide');initSlides();sti=setInterval(nextSl,5000);updS();showP(0);updateFooter();buildMap();buildTimeline();buildTree();initAlphabet();docReady()},400)}catch(e){pm.textContent='Error: '+e.message}};
  xhr.onerror=function(){pm.textContent='Failed to load index.json - check file exists'};
  xhr.send()
}
function docReady(){document.getElementById('st').textContent=IX.meta.entity_count.toLocaleString()+' entities'}
function updS(){if(!IX)return;var t={};IX.nodes.forEach(function(n){var k=n.t||'other';t[k]=(t[k]||0)+1});document.getElementById('st-t').textContent=IX.meta.entity_count.toLocaleString();document.getElementById('st-e').textContent=(t.event||0).toLocaleString();document.getElementById('st-p').textContent=(t.person||0).toLocaleString();document.getElementById('st-pl').textContent=(t.place||0).toLocaleString()}
function updateFooter(){if(!IX)return;document.getElementById('ft-e').textContent=IX.meta.entity_count.toLocaleString();document.getElementById('ft-ed').textContent=IX.meta.edge_count.toLocaleString()}

// ── Views ──
function showView(v){document.querySelectorAll('.view').forEach(function(x){x.classList.remove('act')});var el=document.getElementById('v-'+v);if(el)el.classList.add('act');document.querySelectorAll('.tab').forEach(function(t){t.classList.toggle('act',t.getAttribute('onclick').indexOf(v)>=0)})}

// ── BM25 ──
function buildBM(ns){var k1=1.5,b=0.75,N=ns.length,ad=0,ln=[],tf=[],df={};for(var i=0;i<N;i++){var txt=((ns[i].l||'')+' '+(ns[i].lv||'')+' '+(ns[i].d||'')).toLowerCase(),tr=txt.split(/\s+/).filter(function(t){return t.length>1});ln.push(tr.length);ad+=tr.length;var ft={};tr.forEach(function(t){ft[t]=(ft[t]||0)+1});tf.push(ft);for(var k in ft)df[k]=(df[k]||0)+1}ad/=N;return function(q){var qt=q.toLowerCase().split(/\s+/).filter(function(t){return t.length>1}),sc=[];for(var i=0;i<N;i++){var s=0,l=ln[i],ft=tf[i];for(var j=0;j<qt.length;j++){var f=ft[qt[j]]||0;if(!f)continue;var idf=Math.log((N-(df[qt[j]]||0)+0.5)/((df[qt[j]]||0)+0.5)+1);s+=idf*((f*(k1+1))/(f+k1*(1-b+b*(l/ad))))}sc.push({i:i,s:s})}sc.sort(function(a,b){return b.s-a.s});return sc.filter(function(x){return x.s>0})}}

function onSearch(){clearTimeout(window._st);window._st=setTimeout(doSearch,300)}
function doSearch(){if(!IX)return;var q=document.getElementById('si').value.trim();if(!q){showP(0);return}document.getElementById('st').textContent='Searching...';var ns=IX.nodes;if(_F!=='all')ns=ns.filter(function(n){return n.t===_F});if(_E)ns=ns.filter(function(n){return mE(n,_E)});if(!bm)bm=buildBM(IX.nodes);var sc=bm(q),rs=sc.slice(0,50).map(function(x){return{node:ns[x.i],score:x.s}});ren(rs)}
function ren(rs){var d=document.getElementById('r'),s=document.getElementById('st');if(!rs.length){s.textContent='No results';d.innerHTML='<div style=text-align:center;padding:40px;color:var(--text3)>&#x1F914; No matching entities</div>';return}s.textContent=rs.length+' results';d.innerHTML=rs.map(function(r){var n=r.node,tc='badge-'+(n.t||'other');return'<div class=card onclick="showD(\''+n.id+'\')"><div class=ch><span class=ct>'+esc(n.l)+'</span>'+(n.lv?' <span class=ct-vi>'+esc(n.lv)+'</span>':'')+' <span class="badge '+tc+'">'+(n.t||'other')+'</span></div><div class=cd>'+esc(n.d||'')+'</div><div class=cn>&#x1F50D; BM25: '+r.score.toFixed(4)+'</div></div>'}).join('')}
function esc(s){return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;')}

function setF(t){_F=t;_P=0;document.querySelectorAll('.fbar .fbtn').forEach(function(b){b.classList.toggle('act',b.getAttribute('onclick').indexOf(t)>=0)});var q=document.getElementById('si').value.trim();q?doSearch():showP(0)}
function setE(e){_E=_E===e?null:e;document.querySelectorAll('.ebtn').forEach(function(b){b.classList.toggle('act',b.getAttribute('onclick').indexOf(e)>=0)});var q=document.getElementById('si').value.trim();q?doSearch():showP(0)}
function mE(n,e){if(!n.m)return false;var era={hong_bang:[-2879,-258],chinese_domination:[-111,939],dynastic_vn:[939,1858],colonial:[1858,1954],vietnam_war:[1955,1975],modern:[1975,9999]},r=era[e];if(!r)return false;var f=['point_in_time','start_time','birth_date','death_date'];for(var i=0;i<f.length;i++){var v=n.m[f[i]];if(v){var m=String(v).match(/(-?\d{4})/);if(m){var y=parseInt(m[1],10);if(y>=r[0]&&y<=r[1])return true}}}return false}

// ── Pagination ──
function showP(p){if(!IX)return;_P=p;var ns=IX.nodes;if(_F!=='all')ns=ns.filter(function(n){return n.t===_F});if(_E)ns=ns.filter(function(n){return mE(n,_E)});var t=ns.length,pg=Math.ceil(t/PS),st=p*PS,en=Math.min(st+PS,t);document.getElementById('st').textContent=t.toLocaleString()+' total - page '+(p+1)+'/'+pg;document.getElementById('r').innerHTML=ns.slice(st,en).map(function(n){var tc='badge-'+(n.t||'other');return'<div class=card onclick="showD(\''+n.id+'\')"><div class=ch><span class=ct>'+esc(n.l)+'</span>'+(n.lv?' <span class=ct-vi>'+esc(n.lv)+'</span>':'')+' <span class="badge '+tc+'">'+(n.t||'other')+'</span></div><div class=cd>'+esc(n.d||'')+'</div></div>'}).join('');document.getElementById('pg').innerHTML='<div style=text-align:center;display:flex;gap:8px;justify-content:center;margin-top:10px>'+(p>0?'<button class=fbtn onclick="showP('+(p-1)+')">&larr; Prev</button>':'')+'<span style=color:var(--text3);padding:6px;font-size:.8rem>'+(p+1)+' / '+pg+'</span>'+(en<t?'<button class=fbtn onclick="showP('+(p+1)+')">Next &rarr;</button>':'')+'</div>'}

// ── Alphabet Index ──
function initAlphabet(){if(!IX)return;var letters='ABCDEFGHIJKLMNOPQRSTUVWXYZ#',html='';for(var i=0;i<letters.length;i++){var l=letters[i];html+='<button class=alpha-btn onclick="filterAlpha(\''+l+'\')">'+l+'</button>'}document.getElementById('alpha').innerHTML=html}
function filterAlpha(l){_P=0;var q=document.getElementById('si').value.trim();if(q){doSearch();return}var ns=IX.nodes;if(_F!=='all')ns=ns.filter(function(n){return n.t===_F});if(_E)ns=ns.filter(function(n){return mE(n,_E)});if(l!=='#'){var first=l.toLowerCase();ns=ns.filter(function(n){return n.l&&n.l[0].toLowerCase()===first})}else{ns=ns.filter(function(n){return!n.l||!n.l.match(/[a-zA-Z]/)})}if(!ns.length){document.getElementById('r').innerHTML='<div style=text-align:center;padding:40px;color:var(--text3)>&#x1F50D; No entities starting with '+l+'</div>';return}document.getElementById('st').textContent=ns.length+' starting with '+l;showPageFrom(ns,0)}
function showPageFrom(ns,p){var PS2=40,t=ns.length,pg=Math.ceil(t/PS2),st=p*PS2,en=Math.min(st+PS2,t);document.getElementById('r').innerHTML=ns.slice(st,en).map(function(n){var tc='badge-'+(n.t||'other');return'<div class=card onclick="showD(\''+n.id+'\')"><div class=ch><span class=ct>'+esc(n.l)+'</span>'+(n.lv?' <span class=ct-vi>'+esc(n.lv)+'</span>':'')+' <span class="badge '+tc+'">'+(n.t||'other')+'</span></div><div class=cd>'+esc(n.d||'')+'</div></div>'}).join('');document.getElementById('pg').innerHTML='<div style=text-align:center;display:flex;gap:8px;justify-content:center;margin-top:10px>'+(p>0?'<button class=fbtn onclick="showPageFrom(ns,'+(p-1)+')">&larr; Prev</button>':'')+'<span style=color:var(--text3);padding:6px;font-size:.8rem>'+(p+1)+' / '+pg+'</span>'+(en<t?'<button class=fbtn onclick="showPageFrom(ns,'+(p+1)+')">Next &rarr;</button>':'')+'</div>'}

// ── Detail Overlay ──
function showD(id){var n=IX.nodes.find(function(x){return x.id===id});if(!n)return;var rel=IX.edges.filter(function(e){return e.s===id}).slice(0,30),meta='';if(n.m){var ks=Object.keys(n.m);if(ks.length)meta='<dl class=p-meta>'+ks.map(function(k){return'<dt>'+esc(k)+'</dt><dd>'+esc(n.m[k])+'</dd>'}).join('')+'</dl>'}
// Live fetch from WikiData API
var liveHtml='<div class=p-live><h4>&#x1F310; Loading live data from WikiData...</h4><div class=pld>Fetching...</div></div>';
fetch('https://www.wikidata.org/wiki/Special:EntityData/'+id+'.json').then(function(r){return r.json()}).then(function(d){var e=d.entities[id];if(!e)return;var html='<h4>&#x1F310; WikiData Live</h4><div class=pld>';
if(e.labels&&e.labels.en)html+='<b>Label:</b> '+esc(e.labels.en.value)+'<br>';
if(e.labels&&e.labels.vi)html+='<b>Tieng Viet:</b> '+esc(e.labels.vi.value)+'<br>';
if(e.descriptions&&e.descriptions.en)html+='<b>Desc:</b> '+esc(e.descriptions.en.value)+'<br>';
if(e.sitelinks&&e.sitelinks.enwiki)html+='<b>Wikipedia:</b> <a href=https://en.wikipedia.org/wiki/'+esc(e.sitelinks.enwiki.title.replace(/ /g,'_'))+' target=_blank>'+esc(e.sitelinks.enwiki.title)+'</a><br>';
// Try to find image
if(e.claims){for(var p in e.claims){var claim=e.claims[p];if(p==='P18'&&claim[0]&&claim[0].mainsnak&&claim[0].mainsnak.datavalue){html+='<b>Image:</b> '+esc(claim[0].mainsnak.datavalue.value)+'<br>'}
if(p==='P569'&&claim[0]&&claim[0].mainsnak&&claim[0].mainsnak.datavalue)html+='<b>Born:</b> '+esc(claim[0].mainsnak.datavalue.value.time||'')+'<br>'
if(p==='P570'&&claim[0]&&claim[0].mainsnak&&claim[0].mainsnak.datavalue)html+='<b>Died:</b> '+esc(claim[0].mainsnak.datavalue.value.time||'')+'<br>'}}
html+='</div>';document.querySelector('.p-live').innerHTML=html}).catch(function(){document.querySelector('.p-live').innerHTML='<h4>&#x1F310; WikiData</h4><div class=pld><a href='+esc(n.u)+' target=_blank>View on WikiData</a></div>'});

document.getElementById('dc').innerHTML='<div class=p-title>'+esc(n.l)+'</div>'+(n.lv?'<div class=p-title-vi>'+esc(n.lv)+'</div>':'')+'<span class="badge badge-'+(n.t||'other')+'">'+(n.t||'other')+'</span><div class=p-desc>'+esc(n.d||'')+'</div>'+liveHtml+meta+'<div class=p-sec><h3>&#x1F517; Relations ('+rel.length+')</h3>'+(rel.length?rel.map(function(e){return'<div class=p-rel onclick="showD(\''+e.t+'\')"><span>'+esc(e.r)+'</span><span class=rarr>&rarr;</span><strong>'+esc(e.tl||e.t)+'</strong></div>'}).join(''):'<span style=color:var(--text3);font-size:.8rem>No relations</span>')+'</div><div style=margin-top:12px><a href='+esc(n.u)+' target=_blank style=color:var(--a1);font-size:.82rem>&#x1F517; Open in WikiData &rarr;</a></div>';
document.getElementById('ol').classList.add('open')}
function closeD(e){if(e&&e.target!==document.getElementById('ol'))return;document.getElementById('ol').classList.remove('open')}

// ── Map (Leaflet) ──
var MAP=null;
function buildMap(){if(!IX)return;var hasCoords=IX.nodes.filter(function(n){return n.m&&(n.m.coordinates||n.m.coord)});if(!hasCoords.length)return;
setTimeout(function(){
MAP=L.map('map').setView([16.0,107.5],6);
L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png',{maxZoom:18,attribution:'© OpenStreetMap'}).addTo(MAP);
var colors={place:'#22c55e',person:'#6366f1',event:'#eab308',other:'#64748b'};
hasCoords.forEach(function(n){var c=n.m.coordinates||n.m.coord;if(!c)return;var m=c.match(/(-?\d+\.?\d*)\s*[,\s]\s*(-?\d+\.?\d*)/);if(!m)return;var lat=parseFloat(m[1]),lng=parseFloat(m[2]);if(isNaN(lat)||isNaN(lng))return;var col=colors[n.t]||'#64748b',emoji=n.t==='place'?'&#x1F4CD;':n.t==='person'?'&#x1F464;':n.t==='event'?'&#x1F4C5;':'&#x1F539;';
L.circleMarker([lat,lng],{radius:7,color:col,fillColor:col,fillOpacity:.6,weight:2}).addTo(MAP).bindPopup('<b>'+esc(n.l)+'</b>'+(n.lv?'<br>'+esc(n.lv):'')+'<br><span style=color:var(--text2)>'+esc(n.d||'')+'</span><br><a href=# onclick="closeD();showD(\\''+n.id+'\\')" style=color:var(--a1)>Details &rarr;</a>')});
},500)}

// ── Timeline ──
var TL_DATA=null;
function buildTimeline(){if(!IX)return;TL_DATA=IX.nodes.filter(function(n){return n.t==='event'&&n.m&&(n.m.point_in_time||n.m.start_time)});renderTL()}
function renderTL(){if(!TL_DATA||!TL_DATA.length)return;var c=document.getElementById('tl-c'),w=300*TLZ;c.style.minWidth=w+'%';c.innerHTML='';var years=TL_DATA.map(function(n){var v=n.m.point_in_time||n.m.start_time;if(!v)return null;var m=String(v).match(/(-?\d{4,6})/);return m?parseInt(m[1],10):null}).filter(function(y){return y!==null});if(!years.length)return;var minY=Math.min.apply(null,years),maxY=Math.max.apply(null,years),range=Math.max(maxY-minY,1);c.style.position='relative';c.style.height='200px';var ax=document.createElement('div');ax.className='tl-ax';c.appendChild(ax);
// Century markers
for(var cY=Math.floor(minY/100)*100;cY<=maxY;cY+=100*TLZ){var pct=((cY-minY)/range)*100;var m=document.createElement('div');m.className='tl-ms';m.style.left=pct+'%';var label=cY<0?Math.abs(cY)+' BCE':cY+' CE';m.textContent=label;c.appendChild(m)}
// Event dots
TL_DATA.forEach(function(n){var v=n.m.point_in_time||n.m.start_time;if(!v)return;var m=String(v).match(/(-?\d{4,6})/);if(!m)return;var y=parseInt(m[1],10),pct=((y-minY)/range)*100;var e=document.createElement('div');e.className='tl-evt';e.style.left=pct+'%';e.style.top=(60+Math.random()*80)+'px';e.onclick=function(){showD(n.id)};e.innerHTML='<div class=tl-dot style=background:var(--warn)></div><div class=tl-lb>'+esc(n.l)+(n.lv?' ('+esc(n.lv)+')':'')+'</div>';c.appendChild(e)})}
function tlZoom(d){TLZ=Math.max(0.2,Math.min(5,TLZ+(d*0.5)));renderTL()}
function tlReset(){TLZ=1;renderTL()}

// ── Dynasty Tree ──
function buildTree(){if(!IX)return;var dynasties=IX.nodes.filter(function(n){return n.t==='event'&&n.l&&(n.l.toLowerCase().indexOf('dynasty')>=0||n.l.toLowerCase().indexOf('king')>=0||n.l.toLowerCase().indexOf('emperor')>=0)}).slice(0,30);if(!dynasties.length)return;var c=document.getElementById('tc');c.innerHTML='';dynasties.forEach(function(n,i){var t=document.createElement('div');t.className='tree-n';t.onclick=function(){showD(n.id)};t.style.top='0';t.style.left=(i*130)+'px';t.innerHTML='<div class=tn-av>'+(n.t==='person'?'&#x1F464;':'&#x1F451;')+'</div><div class=tn-lb>'+esc(n.l)+'</div><div class=tn-sub>'+(n.lv?'<i>'+esc(n.lv)+'</i>':'')+'</div>';c.appendChild(t)})}

// ── Chat ──
var CHAT_OPEN=false,CM_HISTORY=[];
function toggleChat(){CHAT_OPEN=!CHAT_OPEN;document.getElementById('cp').classList.toggle('open',CHAT_OPEN);if(CHAT_OPEN&&!CM_HISTORY.length){addChatMsg('bot','&#x1F44B; Hello! I am the MiniDi assistant. Ask me anything about the Vietnam knowledge graph!');CM_HISTORY.push('init')}}
function addChatMsg(role,text){var d=document.getElementById('cm'),m=document.createElement('div');m.className='cm '+role;m.innerHTML=text;d.appendChild(m);d.scrollTop=d.scrollHeight}
function chatSend(txt){var inp=document.getElementById('ci');if(txt){inp.value=txt}else{txt=inp.value.trim();inp.value=''}if(!txt)return;addChatMsg('user',esc(txt));getBotResponse(txt)}
function getBotResponse(q){var lq=q.toLowerCase();var resp='';
var total=IX?IX.meta.entity_count.toLocaleString():'--';var edges=IX?IX.meta.edge_count.toLocaleString():'--';
if(lq.indexOf('count')>=0||lq.indexOf('entity')>=0||lq.indexOf('how many')>=0||lq.indexOf('stat')>=0){resp='&#x1F4CA; The graph has <b>'+total+'</b> entities and <b>'+edges+'</b> relations.'}
else if(lq.indexOf('dynasty')>=0||lq.indexOf('king')>=0||lq.indexOf('emperor')>=0||lq.indexOf('nguyen')>=0){var kings=IX?IX.nodes.filter(function(n){return n.l&&n.l.toLowerCase().indexOf('nguy')>=0&&(n.t==='person'||n.t==='event')}):[];if(kings.length){resp='&#x1F451; Found <b>'+kings.length+'</b> Nguyen-related entities:<br>'+kings.slice(0,10).map(function(n){return'<a href=# onclick="closeD();showD(\\''+n.id+'\\')" style=color:var(--a1)>'+esc(n.l)+'</a>'}).join(', ')}else{resp='&#x1F451; Try searching for "Nguyen" in the search bar!'}}
else if(lq.indexOf('battle')>=0||lq.indexOf('war')>=0){var battles=IX?IX.nodes.filter(function(n){return n.l&&(n.l.toLowerCase().indexOf('battle')>=0||n.l.toLowerCase().indexOf('war')>=0)}):[];resp='&#x2694; Found <b>'+(battles?battles.length:0)+'</b> battles/wars. Try the timeline view!'}
else if(lq.indexOf('1789')>=0||lq.indexOf('year')>=0){resp='&#x1F4C5; Try searching "1789" in the search bar, or check the Timeline view!'}
else if(lq.indexOf('hello')>=0||lq.indexOf('hi')>=0){resp='&#x1F44B; Hello! Ask me about Vietnam history, people, or统计数据!'}
else{resp='&#x1F50D; I found <b>'+total+'</b> entities in the database. Try asking about dynasties, battles, or specific years! You can also use the search bar above.'}
setTimeout(function(){addChatMsg('bot',resp)},300)}
function escChat(s){return String(s).replace(/</g,'&lt;').replace(/>/g,'&gt;')}

// ── Lang ──
function setL(l){_L=l;document.getElementById('le').className='lang-btn'+(l==='en'?' act':'');document.getElementById('lv').className='lang-btn'+(l==='vi'?' act':'');document.querySelectorAll('[data-en]').forEach(function(el){el.textContent=l==='vi'?(el.getAttribute('data-vi')||el.textContent):(el.getAttribute('data-en')||el.textContent)})}

// ── Init ──
document.addEventListener('DOMContentLoaded',loadData);
</script>"""

full_html = (
    HTML_HEAD
    + SPLASH
    + HEADER
    + HERO
    + VIEWS
    + DETAIL_OVERLAY
    + CHAT
    + FOOTER
    + SCRIPT
    + "</body></html>"
)

path = os.path.join(DATA, "docs", "index.html")
with open(path, "w", encoding="utf-8") as f:
    f.write(full_html)
print(f"Written: {path} ({len(full_html)} bytes)")
