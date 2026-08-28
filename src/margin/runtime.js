/* Margin figure runtime. Character-agnostic: it plays whatever beat table the
 * build emitted, so adding a character or a routine never touches this file.
 * Beat: [ms, y, opacity, walking, pose, prop_lit, fade_ms]. Rationale for the
 * choreography lives in src/margin/routine.rs, not on the wire. */
(function(){
var D=__MG_DATA__,r=document.getElementById('mg');
if(!r)return;
// The figure and its light travel together. A character with no light emits no
// aura element, so this is a list rather than a pair.
var mv=[].slice.call(r.querySelectorAll('.mg-fig,.mg-aura')),
    fr=[].slice.call(r.querySelectorAll('.mg-fr')),i;
// Persists the wall-clock time an idle wait is due, so a same-tab navigation
// resumes the countdown instead of restarting it. sessionStorage throws in
// some private-browsing modes; a lost wait just falls back to a fresh one.
var K='mg-due';
function due(ms){try{sessionStorage.setItem(K,Date.now()+ms);}catch(e){}return ms;}
function forget(){try{sessionStorage.removeItem(K);}catch(e){}}
function resume(fallbackMs){
  var s=null;
  try{s=sessionStorage.getItem(K);}catch(e){}
  return s?Math.max(0,s-Date.now()):fallbackMs;
}
// Same idea for a crossing in progress: "routine index,start time" is enough
// to replay the beat table's cumulative durations and find where he'd be now.
var C='mg-cross';
function track(i,t){try{sessionStorage.setItem(C,i+','+t);}catch(e){}}
function untrack(){try{sessionStorage.removeItem(C);}catch(e){}}
function locate(beats,elapsed){
  var acc=0;
  for(var k=0;k<beats.length;k++){acc+=beats[k][0];if(elapsed<acc)return{k:k,rem:acc-elapsed};}
  return null;  // the crossing finished during the time we were away
}
function set(y,op,dur,fade){
  var t='translateY('+(-y*D.s)+'px)',d=dur+'ms, '+fade+'ms';
  for(var k=0;k<mv.length;k++){
    mv[k].style.transitionDuration=d;mv[k].style.transform=t;mv[k].style.opacity=op;
  }
}
function show(pose,walk){
  r.classList.toggle('mg-walk',walk);
  // Clearing the inline value hands the frame back to the walk keyframes.
  for(var j=0;j<fr.length;j++)fr[j].style.opacity=walk?'':(fr[j].dataset.r===pose?'1':'0');
}
function apply(b,dur,fade){
  show(b[4],!!b[3]);
  r.classList.toggle('mg-lit',!!b[5]);
  set(b[1],b[2],dur,fade===undefined?b[6]:fade);
}
function pick(){
  var n=Math.random()*D.t,a=0;
  for(var j=0;j<D.r.length;j++){a+=D.r[j].w;if(n<a)return j;}
  return 0;
}
// at, if given, is {i,t} for a crossing already under way: jump straight to
// wherever it would be now instead of restarting it from off-stage.
function cross(done,at){
  var idx=at?at.i:pick(),start=at?at.t:Date.now();
  var s=D.r[idx].b,j=0,first=60;
  track(idx,start);
  r.classList.remove('mg-walk','mg-lit','mg-idle');
  if(at){
    var loc=locate(s,Date.now()-start);
    apply(s[loc.k],0,0);j=loc.k+1;first=loc.rem;  // snap, no transition to catch up on
  }else{
    set(D.o,0,0,0);
  }
  setTimeout(function step(){
    if(j>=s.length){r.classList.add('mg-idle');untrack();done();return;}
    var b=s[j];
    apply(b,(j===0||b[3])?b[0]:380);  // only travelling beats interpolate; a turn snaps
    j++;setTimeout(step,b[0]);
  },first);
}
function loop(){
  setTimeout(function(){
    forget();
    // Width is read here, once per crossing, not by a resize listener.
    if(document.hidden||innerWidth<D.m){loop();return;}
    cross(loop);
  },due((D.a+Math.random()*(D.b-D.a))*1000));
}
function idleWait(){
  r.classList.add('mg-idle');  // idle keeps no compositing layer alive
  setTimeout(function(){
    forget();
    (!document.hidden&&innerWidth>=D.m)?cross(loop):loop();
  },due(resume(D.f*1000)));
}
function boot(){
  if(matchMedia('(prefers-reduced-motion: reduce)').matches){
    var b=D.r[0].b,st=b[0];
    for(i=0;i<b.length;i++)if(!b[i][3]&&b[i][5]){st=b[i];break;}
    show(st[4],false);r.classList.add('mg-lit');set(st[1],1,0,0);
  }else{
    var cr=null;
    try{cr=sessionStorage.getItem(C);}catch(e){}
    var p=cr?cr.split(','):null;
    // A recorded crossing can still be under way even when document.hidden
    // reads true here — that reads true on plenty of perfectly normal
    // same-tab loads (a click that lands mid-paint, prerendering, a nav link
    // opened in a background tab), not just tabs nobody is looking at. So it
    // is not a reason to skip resuming: resuming costs nothing if the tab
    // really is backgrounded (the browser already throttles it), whereas
    // skipping it strands the record unrendered while real time keeps
    // accruing against it — the next page then resumes it far later than it
    // actually is, jumping straight into the dissolve. Width is the only
    // real reason not to render: below it there is no gutter to draw in.
    // Only clear the record once the crossing has genuinely run its course.
    var loc=p?locate(D.r[+p[0]].b,Date.now()-p[1]):null;
    if(loc&&innerWidth>=D.m)cross(loop,{i:+p[0],t:+p[1]});
    else{if(p&&!loc)untrack();idleWait();}
  }
}
// A hovered/pressed link can make Chrome prerender its destination in a
// hidden document before it's clicked; document.hidden would read true
// there and wrongly abandon a crossing that's about to become visible.
if(document.prerendering)document.addEventListener('prerenderingchange',boot,{once:true});
else boot();
})();
