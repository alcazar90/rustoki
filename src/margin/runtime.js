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
function apply(b,dur){
  show(b[4],!!b[3]);
  r.classList.toggle('mg-lit',!!b[5]);
  set(b[1],b[2],dur,b[6]);
}
function pick(){
  var n=Math.random()*D.t,a=0;
  for(var j=0;j<D.r.length;j++){a+=D.r[j].w;if(n<a)return D.r[j].b;}
  return D.r[0].b;
}
function cross(done){
  var s=pick(),j=0;
  r.classList.remove('mg-walk','mg-lit','mg-idle');
  set(D.o,0,0,0);
  setTimeout(function step(){
    if(j>=s.length){r.classList.add('mg-idle');done();return;}
    var b=s[j];
    apply(b,(j===0||b[3])?b[0]:380);  // only travelling beats interpolate; a turn snaps
    j++;setTimeout(step,b[0]);
  },60);
}
function loop(){
  setTimeout(function(){
    // Width is read here, once per crossing, not by a resize listener.
    if(document.hidden||innerWidth<D.m){loop();return;}
    cross(loop);
  },(D.a+Math.random()*(D.b-D.a))*1000);
}
if(matchMedia('(prefers-reduced-motion: reduce)').matches){
  var b=D.r[0].b,st=b[0];
  for(i=0;i<b.length;i++)if(!b[i][3]&&b[i][5]){st=b[i];break;}
  show(st[4],false);r.classList.add('mg-lit');set(st[1],1,0,0);
}else{
  r.classList.add('mg-idle');  // idle keeps no compositing layer alive
  setTimeout(function(){(!document.hidden&&innerWidth>=D.m)?cross(loop):loop();},D.f*1000);
}
})();
