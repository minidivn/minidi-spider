#!/usr/bin/env python3
"""Write index.html to the data repo docs folder."""
import os

DATA_DOCS = "G:/i2c/PROJECTS/MiniPlatform/MiniDi/Data/minidi-vn-data/docs"
os.makedirs(DATA_DOCS, exist_ok=True)

html = r"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>MiniDi — Vietnam Knowledge Graph</title>
<meta name="description" content="Kham pha du lieu WikiData ve Viet Nam: lich su, dia ly, nhan vat, van hoa.">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&display=swap" rel="stylesheet">
<style>
:root{--bg-primary:#070b17;--bg-secondary:#0f172a;--bg-tertiary:#1e293b;--bg-glass:rgba(255,255,255,.03);--bg-glass-hover:rgba(255,255,255,.06);--border-glass:rgba(255,255,255,.07);--border-glass-hover:rgba(255,255,255,.12);--accent-1:#6366f1;--accent-2:#06b6d4;--accent-3:#a78bfa;--accent-1-dim:rgba(99,102,241,.15);--accent-2-dim:rgba(6,182,212,.15);--text-primary:#f1f5f9;--text-secondary:#94a3b8;--text-tertiary:#64748b;--success:#22c55e;--warning:#eab308;--radius-sm:6px;--radius-md:10px;--radius-lg:16px;--shadow-glow:0 0 40px rgba(99,102,241,.08);--transition:.3s cubic-bezier(.4,0,.2,1)}
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:'Inter',-apple-system,BlinkMacSystemFont,sans-serif;background:var(--bg-primary);color:var(--text-primary);min-height:100vh;overflow-x:hidden}
#bg-canvas{position:fixed;top:0;left:0;width:100vw;height:100vh;z-index:0;pointer-events:none;overflow:hidden}
#bg-canvas .orb{position:absolute;border-radius:50%;filter:blur(80px);opacity:.12;animation:orbFloat 20s ease-in-out infinite}
#bg-canvas .orb:nth-child(1){width:600px;height:600px;background:radial-gradient(circle,#6366f1,transparent);top:-10%;left:-5%}
#bg-canvas .orb:nth-child(2){width:500px;height:500px;background:radial-gradient(circle,#06b6d4,transparent);bottom:-15%;right:-5%;animation-delay:-7s}
#bg-canvas .orb:nth-child(3){width:400px;height:400px;background:radial-gradient(circle,#8b5cf6,transparent);top:40%;left:50%;animation-delay:-14s}
@keyframes orbFloat{0%,100%{transform:translate(0,0)scale(1)}25%{transform:translate(60px,-80px)scale(1.05)}50%{transform:translate(-40px,60px)scale(.95)}75%{transform:translate(80px,40px)scale(1.02)}}
::-webkit-scrollbar{width:6px}::-webkit-scrollbar-track{background:transparent}::-webkit-scrollbar-thumb{background:var(--text-tertiary);border-radius:3px}
.header{display:flex;align-items:center;gap:16px;padding:16px 24px;background:rgba(7,11,23,.8);backdrop-filter:blur(12px);border-bottom:1px solid var(--border-glass);position:sticky;top:0;z-index:50}
.header-logo{display:flex;align-items:center;gap:10px;font-weight:700;font-size:1.1rem}
.logo-icon{width:32px;height:32px;background:linear-gradient(135deg,var(--accent-1),var(--accent-2));border-radius:8px;display:inline-flex;align-items:center;justify-content:center}
.search-box{flex:1;max-width:500px;position:relative}
.search-box input{width:100%;padding:10px 16px 10px 40px;background:var(--bg-tertiary);border:1px solid var(--border-glass);border-radius:var(--radius-md);color:var(--text-primary);font-size:.9rem;outline:none;transition:var(--transition)}
.search-box input:focus{border-color:var(--accent-1);box-shadow:0 0 20px var(--accent-1-dim)}
.search-box input::placeholder{color:var(--text-tertiary)}
.search-icon{position:absolute;left:12px;top:50%;transform:translateY(-50%);color:var(--text-tertiary)}
.hero{text-align:center;padding:48px 24px 32px}
.hero h1{font-size:2.5rem;font-weight:800}
.gradient-text{background:linear-gradient(135deg,var(--accent-1),var(--accent-2),var(--accent-3));-webkit-background-clip:text;-webkit-text-fill-color:transparent}
.hero p{color:var(--text-secondary);max-width:600px;margin:12px auto;line-height:1.6;font-size:.95rem}
.stats{display:flex;justify-content:center;gap:32px;flex-wrap:wrap;margin-top:20px}
.stat{text-align:center}
.stat .num{font-size:1.8rem;font-weight:700;background:linear-gradient(135deg,var(--accent-1),var(--accent-2));-webkit-background-clip:text;-webkit-text-fill-color:transparent}
.stat .lbl{color:var(--text-tertiary);font-size:.8rem;margin-top:2px}
.container{max-width:860px;margin:0 auto;padding:0 20px}
#status{text-align:center;padding:8px;font-size:.85rem;color:var(--warning);min-height:32px}
.loading{text-align:center;padding:40px;color:var(--text-tertiary)}
.spinner{width:32px;height:32px;border:3px solid var(--bg-tertiary);border-top-color:var(--accent-1);border-radius:50%;animation:spin .8s linear infinite;margin:0 auto 12px}
@keyframes spin{to{transform:rotate(360deg)}}
.card{background:var(--bg-glass);border:1px solid var(--border-glass);border-radius:var(--radius-md);padding:14px 18px;margin-bottom:8px;cursor:pointer;transition:var(--transition)}
.card:hover{background:var(--bg-glass-hover);border-color:var(--border-glass-hover);transform:translateY(-1px)}
.card-hdr{display:flex;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:4px}
.card-title{font-size:1rem;font-weight:600;color:var(--accent-1)}
.card-title-vi{font-size:.8rem;color:var(--text-tertiary);font-style:italic}
.badge{font-size:.65rem;padding:2px 10px;border-radius:20px;font-weight:500}
.badge-place{background:rgba(34,197,94,.15);color:var(--success)}
.badge-person{background:rgba(99,102,241,.15);color:var(--accent-1)}
.badge-event{background:rgba(234,179,8,.15);color:var(--warning)}
.badge-other{background:var(--bg-tertiary);color:var(--text-tertiary)}
.card-desc{color:var(--text-secondary);font-size:.85rem;line-height:1.5;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}
.card-note{font-size:.7rem;color:var(--text-tertiary);margin-top:4px}
.footer{text-align:center;padding:24px;color:var(--text-tertiary);font-size:.8rem;border-top:1px solid var(--border-glass);margin-top:40px}
.footer a{color:var(--accent-1);text-decoration:none}
.overlay{position:fixed;inset:0;background:rgba(0,0,0,.6);backdrop-filter:blur(4px);z-index:100;display:none;align-items:center;justify-content:center}
.overlay.open{display:flex}
.panel{background:var(--bg-secondary);border:1px solid var(--border-glass);border-radius:var(--radius-lg);padding:32px;max-width:620px;width:92%;max-height:85vh;overflow-y:auto;position:relative}
.panel-close{position:absolute;top:12px;right:16px;font-size:1.5rem;cursor:pointer;color:var(--text-tertiary);background:none;border:none}
.panel-close:hover{color:var(--text-primary)}
.panel-title{font-size:1.3rem;font-weight:700}
.panel-title-vi{font-size:.95rem;color:var(--text-tertiary);margin-bottom:8px}
.panel-desc{color:var(--text-secondary);line-height:1.6;margin:12px 0;font-size:.9rem}
.panel-meta{display:grid;grid-template-columns:1fr 1fr;gap:6px;font-size:.8rem;margin:12px 0}
.panel-meta dt{color:var(--text-tertiary)}.panel-meta dd{color:var(--text-primary);word-break:break-all}
.panel-section{margin-top:14px}
.panel-section h3{font-size:.9rem;color:var(--accent-2);margin-bottom:8px}
.panel-rel{display:flex;align-items:center;gap:6px;padding:3px 0;font-size:.8rem;color:var(--text-primary);cursor:pointer}
.panel-rel:hover{color:var(--accent-1)}
.rel-arrow{color:var(--text-tertiary);margin:0 4px}
.filter-bar{display:flex;gap:6px;flex-wrap:wrap;justify-content:center;margin:12px 0}
.filter-btn{padding:5px 14px;border:1px solid var(--border-glass);border-radius:20px;background:transparent;color:var(--text-secondary);cursor:pointer;font-size:.78rem;transition:var(--transition)}
.filter-btn:hover{border-color:var(--accent-1);color:var(--accent-1)}
.filter-btn.active{background:var(--accent-1-dim);border-color:var(--accent-1);color:var(--accent-1)}
.era-bar{display:flex;gap:6px;overflow-x:auto;padding:8px 0;margin:10px 0}
.era-btn{flex-shrink:0;padding:5px 12px;border:1px solid var(--border-glass);border-radius:20px;background:transparent;color:var(--text-tertiary);cursor:pointer;font-size:.72rem;white-space:nowrap;transition:var(--transition)}
.era-btn:hover{border-color:var(--accent-2);color:var(--accent-2)}
.era-btn.active{background:var(--accent-2-dim);border-color:var(--accent-2);color:var(--accent-2)}
.lang-btn{padding:6px 12px;border:1px solid var(--border-glass);border-radius:var(--radius-sm);background:var(--bg-tertiary);color:var(--text-secondary);cursor:pointer;font-size:.8rem;transition:var(--transition)}
.lang-btn.active{background:var(--accent-1);color:#fff;border-color:var(--accent-1)}
@media(max-width:768px){.hero h1{font-size:1.6rem}.stats{gap:16px}.stat .num{font-size:1.3rem}.panel{padding:24px}.panel-meta{grid-template-columns:1fr}}
</style>
</head>
<body>
<div id=bg-canvas><div class=orb></div><div class=orb></div><div class=orb></div></div>
<div class=header>
  <div class=header-logo><span class=logo-icon>&#x1F577;</span><span>MiniDi</span></div>
  <div class=search-box><span class=search-icon>&#x1F50D;</span><input type=text id=search-input placeholder="Search Vietnam... | Tim kiem..." oninput=onSearch()></div>
  <div><button class="lang-btn active" id=lang-en onclick="setLang('en')">EN</button><button class=lang-btn id=lang-vi onclick="setLang('vi')">VI</button></div>
</div>
<div class=hero>
  <h1><span class=gradient-text>&#x1F1FB;&#x1F1F3; Vietnam Knowledge Graph</span></h1>
  <p id=hero-sub>Exploring 14,292 WikiData entities: history, geography, people & culture</p>
  <div class=stats>
    <div class=stat><div class=num id=s-total>--</div><div class=lbl data-en=Entities data-vi="Thuc the">Entities</div></div>
    <div class=stat><div class=num id=s-event>--</div><div class=lbl data-en=Events data-vi="Su kien">Events</div></div>
    <div class=stat><div class=num id=s-person>--</div><div class=lbl data-en=People data-vi="Nhan vat">People</div></div>
    <div class=stat><div class=num id=s-place>--</div><div class=lbl data-en=Places data-vi="Dia danh">Places</div></div>
  </div>
</div>
<div class=container>
  <div class=filter-bar>
    <button class="filter-btn active" onclick="setFilter('all')" data-en=All data-vi="Tat ca">All</button>
    <button class=filter-btn onclick="setFilter('place')" data-en=Places data-vi="Dia danh">Places</button>
    <button class=filter-btn onclick="setFilter('person')" data-en=People data-vi="Nhan vat">People</button>
    <button class=filter-btn onclick="setFilter('event')" data-en=Events data-vi="Su kien">Events</button>
  </div>
  <div class=era-bar>
    <button class=era-btn onclick="setEra('hong_bang')">&#x1F3EE; Hong Bang</button>
    <button class=era-btn onclick="setEra('chinese_domination')">&#x1F4DC; Chinese Dom.</button>
    <button class=era-btn onclick="setEra('dynastic_vn')">&#x1F3EF; Dynastic</button>
    <button class=era-btn onclick="setEra('colonial')">&#x1F3DB; Colonial</button>
    <button class=era-btn onclick="setEra('vietnam_war')">&#x2694; Vietnam War</button>
    <button class=era-btn onclick="setEra('modern')">&#x1F3D9; Modern</button>
  </div>
  <div id=status>Loading index.json...</div>
  <div id=results></div>
  <div id=pagination></div>
</div>
<div class=overlay id=overlay onclick="closeDetail(event)">
  <div class=panel><button class=panel-close onclick="closeDetail()">&times;</button><div id=detail-content></div></div>
</div>
<div class=footer>
  <span data-en="Data from WikiData - CC0. Built with " data-vi="Du lieu tu WikiData - CC0. Xay dung voi ">Data from WikiData - CC0. Built with</span>
  <a href=https://github.com/minidivn/minidi-spider>minidi-spider</a> &middot;
  <a href=https://github.com/minidivn/minidi-vn-data>source</a> &middot;
  <a href=https://query.wikidata.org/sparql>SPARQL</a>
</div>
<script>
var INDEX=null,MODEL=null,MR=0,_filter='all',_era=null,_lang='en',_page=0,PS=40,bm=null;
document.addEventListener('DOMContentLoaded',function(){loadIndex();setTimeout(loadModel,2000)});
function loadIndex(){var s=document.getElementById('status');fetch('index.json').then(function(r){return r.json()}).then(function(d){INDEX=d;upd();s.textContent='Loaded '+d.meta.entity_count+' entities';showPage(0)}).catch(function(e){s.textContent='Error loading index.json';console.error(e)})}
function upd(){if(!INDEX)return;var t={place:0,person:0,event:0,concept:0,other:0};INDEX.nodes.forEach(function(n){var k=n.t||'other';t[k]=(t[k]||0)+1});document.getElementById('s-total').textContent=INDEX.meta.entity_count.toLocaleString();document.getElementById('s-event').textContent=(t.event||0).toLocaleString();document.getElementById('s-person').textContent=(t.person||0).toLocaleString();document.getElementById('s-place').textContent=(t.place||0).toLocaleString()}
function buildBM(ns){var k1=1.5,b=0.75,N=ns.length,ad=0,ln=[],tf=[],df={};for(var i=0;i<N;i++){var t=((ns[i].l||'')+' '+(ns[i].lv||'')+' '+(ns[i].d||'')).toLowerCase(),tr=t.split(/\\s+/).filter(function(t){return t.length>1});ln.push(tr.length);ad+=tr.length;var ft={};tr.forEach(function(t){ft[t]=(ft[t]||0)+1});tf.push(ft);for(var k in ft)df[k]=(df[k]||0)+1}ad/=N;return function(q){var qt=q.toLowerCase().split(/\\s+/).filter(function(t){return t.length>1}),sc=[];for(var i=0;i<N;i++){var s=0,l=ln[i],ft=tf[i];for(var j=0;j<qt.length;j++){var f=ft[qt[j]]||0;if(!f)continue;var idf=Math.log((N-(df[qt[j]]||0)+0.5)/((df[qt[j]]||0)+0.5)+1);s+=idf*((f*(k1+1))/(f+k1*(1-b+b*(l/ad))))}sc.push({i:i,s:s})}sc.sort(function(a,b){return b.s-a.s});return sc.filter(function(x){return x.s>0})}}
function onSearch(){clearTimeout(window._st);window._st=setTimeout(doSearch,300)}
function doSearch(){if(!INDEX)return;var q=document.getElementById('search-input').value.trim(),s=document.getElementById('status');if(!q){showPage(0);return}s.textContent='Searching...';var ns=INDEX.nodes;if(_filter!=='all')ns=ns.filter(function(n){return n.t===_filter});if(_era)ns=ns.filter(function(n){return mEra(n,_era)});if(!bm)bm=buildBM(INDEX.nodes);var sc=bm(q),rs=sc.slice(0,50).map(function(x){return{node:ns[x.i],score:x.s}});ren(rs)}
function ren(rs){var d=document.getElementById('results'),s=document.getElementById('status');if(!rs.length){s.textContent='No results';d.innerHTML='<div class=loading>No matches</div>';return}s.textContent=rs.length+' results';d.innerHTML=rs.map(function(r){var n=r.node,tc='badge-'+(n.t||'other');return'<div class=card onclick=\"showDetail(\\''+n.id+'\\')\"><div class=card-hdr><span class=card-title>'+esc(n.l)+'</span>'+(n.lv?' <span class=card-title-vi>'+esc(n.lv)+'</span>':'')+' <span class="badge '+tc+'">'+(n.t||'other')+'</span></div><div class=card-desc>'+esc(n.d||'')+'</div><div class=card-note>BM25: '+r.score.toFixed(4)+'</div></div>'}).join('')}
function esc(s){return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/\"/g,'&quot;')}
function setFilter(t){_filter=t;_page=0;document.querySelectorAll('.filter-btn').forEach(function(b){b.classList.toggle('active',b.getAttribute('onclick').indexOf(t)>=0)});var q=document.getElementById('search-input').value.trim();q?doSearch():showPage(0)}
function setEra(e){_era=_era===e?null:e;document.querySelectorAll('.era-btn').forEach(function(b){b.classList.toggle('active',b.getAttribute('onclick').indexOf(e)>=0)});var q=document.getElementById('search-input').value.trim();q?doSearch():showPage(0)}
function mEra(n,e){if(!n.m)return false;var era={hong_bang:[-2879,-258],chinese_domination:[-111,939],dynastic_vn:[939,1858],colonial:[1858,1954],vietnam_war:[1955,1975],modern:[1975,9999]},r=era[e];if(!r)return false;var f=['point_in_time','start_time','birth_date','death_date'];for(var i=0;i<f.length;i++){var v=n.m[f[i]];if(v){var m=String(v).match(/(-?\\d{4})/);if(m){var y=parseInt(m[1],10);if(y>=r[0]&&y<=r[1])return true}}}return false}
function showPage(p){if(!INDEX)return;_page=p;var ns=INDEX.nodes;if(_filter!=='all')ns=ns.filter(function(n){return n.t===_filter});if(_era)ns=ns.filter(function(n){return mEra(n,_era)});var t=ns.length,pgs=Math.ceil(t/PS),st=p*PS,en=Math.min(st+PS,t);document.getElementById('status').textContent=t+' total - page '+(p+1)+'/'+pgs;var d=document.getElementById('results');d.innerHTML=ns.slice(st,en).map(function(n){var tc='badge-'+(n.t||'other');return'<div class=card onclick=\"showDetail(\\''+n.id+'\\')\"><div class=card-hdr><span class=card-title>'+esc(n.l)+'</span>'+(n.lv?' <span class=card-title-vi>'+esc(n.lv)+'</span>':'')+' <span class="badge '+tc+'">'+(n.t||'other')+'</span></div><div class=card-desc>'+esc(n.d||'')+'</div></div>'}).join('');document.getElementById('pagination').innerHTML='<div style=text-align:center;display:flex;gap:8px;justify-content:center;margin-top:12px>'+(p>0?'<button class=filter-btn onclick=\"showPage('+(p-1)+')\">&larr; Prev</button>':'')+'<span style=color:var(--text-tertiary);padding:6px>'+(p+1)+' / '+pgs+'</span>'+(en<t?'<button class=filter-btn onclick=\"showPage('+(p+1)+')\">Next &rarr;</button>':'')+'</div>'}
function showDetail(id){var n=INDEX.nodes.find(function(x){return x.id===id});if(!n)return;var rel=INDEX.edges.filter(function(e){return e.s===id}).slice(0,30),meta='';if(n.m){var ks=Object.keys(n.m);if(ks.length)meta='<dl class=panel-meta>'+ks.map(function(k){return'<dt>'+esc(k)+'</dt><dd>'+esc(n.m[k])+'</dd>'}).join('')+'</dl>'}document.getElementById('detail-content').innerHTML='<div class=panel-title>'+esc(n.l)+'</div>'+(n.lv?'<div class=panel-title-vi>'+esc(n.lv)+'</div>':'')+'<span class="badge badge-'+(n.t||'other')+'">'+(n.t||'other')+'</span><div class=panel-desc>'+esc(n.d||'')+'</div>'+meta+'<div class=panel-section><h3>Relations ('+rel.length+')</h3>'+(rel.length?rel.map(function(e){return'<div class=panel-rel onclick=\"showDetail(\\''+e.t+'\\')\"><span>'+esc(e.r)+'</span><span class=rel-arrow>&rarr;</span><strong>'+esc(e.tl||e.t)+'</strong></div>'}).join(''):'<span style=color:var(--text-tertiary);font-size:.85rem>No relations</span>')+'</div><div style=margin-top:12px><a href='+esc(n.u)+' target=_blank style=color:var(--accent-1);font-size:.85rem>Open in WikiData &rarr;</a></div>';document.getElementById('overlay').classList.add('open')}
function closeDetail(e){if(e&&e.target!==document.getElementById('overlay'))return;document.getElementById('overlay').classList.remove('open')}
function setLang(l){_lang=l;document.getElementById('lang-en').className='lang-btn'+(l==='en'?' active':'');document.getElementById('lang-vi').className='lang-btn'+(l==='vi'?' active':'');document.querySelectorAll('[data-en]').forEach(function(el){el.textContent=l==='vi'?(el.getAttribute('data-vi')||el.textContent):(el.getAttribute('data-en')||el.textContent)})}
function loadModel(){try{if(typeof transformers==='undefined')return;transformers.pipeline('feature-extraction','Xenova/all-MiniLM-L6-v2',{quantized:true}).then(function(m){MODEL=m;MR=1;document.getElementById('status').textContent+=' | AI ready'})}catch(e){console.warn('No AI:',e)}}
</script>
</body>
</html>"""

path = os.path.join(DATA_DOCS, "index.html")
with open(path, "w", encoding="utf-8") as f:
    f.write(html)

print(f"Written: {path} ({len(html)} bytes)")
print(f"File exists: {os.path.isfile(path)}")
