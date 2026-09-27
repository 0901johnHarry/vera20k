"""Original sinking waterline rectangle and draw-gate evidence.

Interior render locals and projected rectangles are explicit supplied boundaries.
Original intersection421B60 executes. This is not VXL rasterization or a captured
live-process frame, and does not establish the ambient native x87 control word.
"""
from pathlib import Path
import hashlib, struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
from tools.native_oracle import load_image, run_checked, finish_vectors, provenance, STACK_BASE, STACK_SIZE, SCRATCH, SCRATCH_SIZE, RET_MAGIC

HERE=Path(__file__).resolve().parent
ACTOR=SCRATCH;TACTICAL=SCRATCH+0x1000;RECT=SCRATCH+0x2000;POINT=SCRATCH+0x2100
SP=STACK_BASE+0x80000
SLICES={'unit_clip':(0x73BEA4,0x73BF7B),'techno_clip':(0x70709E,0x70716D),'shadow_gate':(0x73C1D2,0x73C202),'extras_gate':(0x6F5190,0x6F51B0),'ctor':(0x6F2F1E,0x6F2F37)}
def dwords(*x):return struct.pack('<'+'I'*len(x),*(a&0xffffffff for a in x))
def rectangle(u,p):return list(struct.unpack('<4i',u.mem_read(p,16)))

def fixture():
 u=Uc(UC_ARCH_X86,UC_MODE_32);load_image(u)
 u.mem_map(STACK_BASE,STACK_SIZE);u.mem_map(SCRATCH,SCRATCH_SIZE);u.mem_map(RET_MAGIC,0x1000)
 u.reg_write(UC_X86_REG_ESP,SP);u.reg_write(UC_X86_REG_EBP,ACTOR);u.reg_write(UC_X86_REG_FPCW,0xE7F)
 u.mem_write(0x887324,dwords(TACTICAL))
 return u

def step(u,c,kind):
 u.mem_write(SP,bytes(0x800));u.reg_write(UC_X86_REG_ESP,SP);u.reg_write(UC_X86_REG_EBP,ACTOR)
 u.mem_write(ACTOR+0x3CD,bytes([c['sinking']]))
 u.mem_write(TACTICAL+0xB4,dwords(c['camera_world_y']));u.mem_write(0x886FA8,dwords(c['view_width']))
 if kind=='unit':
  u.mem_write(SP+0x1CC,dwords(c['draw_y'],*c['caller_rect']))
  u.mem_write(0xB1CFC4,dwords(c['raster_y']));u.mem_write(0xB1CFCC,dwords(c['raster_height']))
  start,end=0x73BEA4,0x73BF7B;out=SP+0x50
 else:
  u.mem_write(RECT,dwords(*c['caller_rect']));u.mem_write(POINT,dwords(0,c['draw_y']))
  u.mem_write(SP+0x14C,dwords(POINT,RECT));u.mem_write(SP+0x38,dwords(c['raster_y']));u.mem_write(SP+0x48,dwords(c['raster_height']))
  u.mem_write(0x887314,dwords(0x11110000));u.mem_write(0xB1D13C,dwords(0x11110000 if c.get('offscreen_surface') else 0x22220000))
  start,end=0x70709E,0x70716D;out=SP+0x20
 writes=[];calls=[]
 def write(u,a,p,n,v,d):
  if ACTOR<=p<ACTOR+0x800:writes.append(dict(pc=hex(u.reg_read(UC_X86_REG_EIP)),offset=hex(p-ACTOR),size=n,value=v))
 def code(u,p,n,d):
  if p==0x421B60:calls.append(dict(pc=hex(p),caller=rectangle(u,u.reg_read(UC_X86_REG_EDX)),candidate=rectangle(u,struct.unpack('<I',u.mem_read(u.reg_read(UC_X86_REG_ESP)+4,4))[0])))
 h=u.hook_add(UC_HOOK_MEM_WRITE,write);h2=u.hook_add(UC_HOOK_CODE,code)
 before=struct.unpack('<h',u.mem_read(ACTOR+0x3CA,2))[0]
 run_checked(u,start,end,count=20000)
 u.hook_del(h);u.hook_del(h2)
 return dict(waterline_before=before,waterline_after=struct.unpack('<h',u.mem_read(ACTOR+0x3CA,2))[0],clip=rectangle(u,out),calls=calls,writes=writes)

def cases():
 base=dict(sinking=1,camera_world_y=1000,view_width=640,draw_y=200,raster_y=100,raster_height=40,caller_rect=[0,0,640,480])
 states=[('first',0,{}),('repeat',1212,{'draw_y':250}),('camera_down',1212,{'camera_world_y':1100}),('camera_above',1212,{'camera_world_y':500}),('fully_clipped',1212,{'camera_world_y':1300}),('partial_caller',1212,{'caller_rect':[20,190,100,100]}),('caller_below',1212,{'caller_rect':[20,230,100,100]}),('flag0_no_cache',0,{'sinking':0}),('flag0_retained',1212,{'sinking':0}),('negative_cache',-1,{}),('signed_wrap',0,{'camera_world_y':32760}),('raw255',0,{'sinking':255})]
 rows=[]
 for kind in ('unit','techno'):
  for name,cache,extra in states:
   c=dict(base,**extra)
   if kind=='techno':c['raster_y']-=128
   u=fixture();u.mem_write(ACTOR+0x3CA,struct.pack('<h',cache));r=step(u,c,kind)
   rows.append(dict(name=kind+'_'+name,input=dict(kind=kind,waterline=cache,**c),output=r))
  if kind=='techno':
   for cache in (0,1212):
    c=dict(base,offscreen_surface=True);c['raster_y']-=128;u=fixture();u.mem_write(ACTOR+0x3CA,struct.pack('<h',cache))
    rows.append(dict(name=f'techno_offscreen_{cache}',input=dict(kind=kind,waterline=cache,**c),output=step(u,c,kind)))
 # Retained object across genuine successive original calls, with camera change.
 for kind in ('unit','techno'):
  u=fixture();cs=[]
  for extra in ({},{'draw_y':250},{'camera_world_y':1100,'draw_y':250},{'camera_world_y':1300,'draw_y':250}):
   c=dict(base,**extra)
   if kind=='techno':c['raster_y']-=128
   cs.append(dict(input=c,output=step(u,c,kind)))
  rows.append(dict(name=kind+'_retained_sequence',input=dict(kind=kind,waterline=0),sequence=cs))
 return rows

def gates():
 rows=[]
 for flag in (0,1,255):
  u=fixture();u.mem_write(ACTOR+0x3CD,bytes([flag]));u.mem_write(ACTOR+0x6D3,b'\1')
  end=run_checked(u,0x73C1D2,(0x73C202,0x73C5C9),count=1000)
  shadow=dict(endpoint=hex(end),shadow_tail_entered=end==0x73C202,flag6d3=u.mem_read(ACTOR+0x6D3,1)[0])
  u=fixture();u.mem_write(ACTOR+0x3CD,bytes([flag]));u.reg_write(UC_X86_REG_ECX,ACTOR);u.mem_write(SP,dwords(RET_MAGIC,RECT,POINT))
  end=run_checked(u,0x6F5190,(0x6F51B0,RET_MAGIC),count=1000)
  rows.append(dict(input=dict(sinking=flag),shadow=shadow,extras=dict(endpoint=hex(end),extras_body_entered=end==0x6F51B0)))
 return rows

def generate():
 u=fixture();u.mem_write(ACTOR+0x3CA,b'\xA5'*5);u.reg_write(UC_X86_REG_ESI,ACTOR);u.reg_write(UC_X86_REG_EBX,0)
 run_checked(u,0x6F2F1E,0x6F2F37,count=1000)
 return dict(schema=1,ctor=dict(supplied_bx=0,waterline=struct.unpack('<h',u.mem_read(ACTOR+0x3CA,2))[0],byte3cc=u.mem_read(ACTOR+0x3CC,1)[0],sinking=u.mem_read(ACTOR+0x3CD,1)[0],seen=u.mem_read(ACTOR+0x3CE,1)[0]),cases=cases(),gates=gates())

def metadata():
 u=fixture()
 p=provenance(scope=__doc__,assumptions=['Original renderer frame locals are supplied: caller clip, draw anchor Y, raster bounds Y/height, Tactical+B4 world camera Y, view width and current/offscreen surface identity. No values derive from Rust.', 'Original Unit73BEA4..73BF7B cached composite and Techno70709E..70716D generic piece rectangles run through native421B60. Unit capture includes -128 because its cached raster uses a128px center; generic inputs use corresponding shifted raster bounds. First capture leaves the caller clip unchanged.', 'Constructor field stores6F2F1E..6F2F37 execute with the enclosing constructor zero-register invariant supplied. Raw-load/no-init persistence is separately executed in naval_lifetime_controls.', 'Shadow gate73C1D2..73C202/73C5C9 and full early DrawExtras6F5190 return for nonzero sinking execute. Sinking0 stops before the remaining ordinary tails. Raw255 is a supplied legacy-byte control, not an active producer claim.', 'All executed instructions in this packet are integer arithmetic, stores and branches. FPCW0E7F is supplied and immaterial to these slices. No RNG calls, timer writes or detach calls are present in these executed slices; preceding raster production excluded.'],substitutions=['No calls are replaced; native421B60 rectangle intersection executes. Projection, raster creation, full render dispatcher and surface blits are outside the measured boundaries.'],entry_points={k:v[0] for k,v in SLICES.items()}|{'intersection':0x421B60})
 p['original_slices']={k:dict(start=hex(a),end=hex(b),bytes=bytes(u.mem_read(a,b-a)).hex()) for k,(a,b) in SLICES.items()}
 return p

if __name__=='__main__':finish_vectors(generate,HERE/'naval_sinking_clip.json',provenance=metadata)
