// Rebuild the idle orbit from the same offline map as the interactive globe.
// Run from the repository root: node scripts/render-vpn-orbit.mjs [ffmpeg-path]
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { readFileSync } from 'node:fs';
const base = 'apps/desktop/src/assets/';
const map = JSON.parse(readFileSync(base + 'vpn-globe.json'));
const detail = Buffer.from(map.detail, 'base64'), palette = Buffer.from(map.lime, 'base64');
const radius = 256, width = 2*radius, height = radius, frames = 1178;
const encoder = spawn(process.argv[2] || 'ffmpeg', ['-hide_banner','-loglevel','error','-y',
  '-f','rawvideo','-pix_fmt','rgb24','-s',width+'x'+height,'-r','30','-i','pipe:0',
  '-an','-c:v','libx264','-threads','2','-preset','fast','-crf','17','-pix_fmt','yuv420p',
  '-movflags','+faststart',base+'vpn-orbit.mp4'], {stdio:['pipe','inherit','inherit']});
const finished = once(encoder, 'close');
const basePixels = Buffer.alloc(width*height*3,17);
for(let y=0;y<height;y++){
  const half=Math.sqrt(radius**2-(y-radius)**2);
  basePixels.fill(0, 3*(y*width+Math.ceil(radius-half)),3*(y*width+Math.floor(radius+half)));
}
const dots=[];
for(let row=0;row<map.gridHeight;row+=4) for(let col=0;col<map.gridWidth;col+=4){
  const i=row*map.gridWidth+col, kind=(detail[i>>2]>>(2*(i&3)))&3;
  if(!kind) continue;
  const lat=((row+.5)/map.gridHeight-.5)*Math.PI, lon=((col+.5)/map.gridWidth-.5)*2*Math.PI;
  dots.push([Math.cos(lat)*Math.sin(lon),Math.sin(lat),Math.cos(lat)*Math.cos(lon),
    kind===2, (palette[i>>3]>>(i&7))&1]);
}
const sy=Math.sin(.38),cy=Math.cos(.38);
for(let frame=0;frame<frames;frame++){
  const buffer=Buffer.from(basePixels);
  const angle=.26+frame/frames*2*Math.PI, sx=Math.sin(angle),cx=Math.cos(angle);
  for(const [x,y,z,border,lime] of dots){
    const mer=x*sx+z*cx, front=y*sy+mer*cy;
    if(front<=0) continue;
    const px=(1+x*cx-z*sx)*radius, py=(1-y*cy+mer*sy)*radius;
    const brightness=border||front>.85?1:(front<.46?.52:.83);
    for(let by=Math.max(0,Math.floor(py-1.5));by<Math.min(height,py+1.5);by++){
      for(let bx=Math.max(0,Math.floor(px-1.5));bx<Math.min(width,px+1.5);bx++){
        const coverage=Math.max(0,Math.min(1,1.85-Math.hypot(Math.max(0,Math.abs(bx+.5-px)-.5),Math.max(0,Math.abs(by+.5-py)-.5))));
        const i=(by*width+bx)*3, v=brightness*coverage;
        buffer[i]=(lime?180:255)*v; buffer[i+1]=lime?255*v:0; buffer[i+2]=lime?0:157*v;
      }
    }
  }
  if(!encoder.stdin.write(buffer)) await once(encoder.stdin,'drain');
}
encoder.stdin.end();
if((await finished)[0] !== 0) process.exitCode=1;
