"""Original physical FV numbered FLH, flat Drive transform and two burst tails.

Physical INI cache setup, source lifecycle, source pose, HouseROF1, ScenarioSeed31
and second FireAt call time are supplied. This proves neither whole Unit AI nor
whole-scenario native identity prefix. Read ifv_fire_coord.md for exact coverage.
"""
import hashlib,json,struct,sys
from pathlib import Path
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
from tools.native_oracle import NATIVE_SHA256,NATIVE_FPCW,RET_MAGIC,run_checked,finish_vectors,provenance
from tools.projectile_oracle.bridge_render_inputs import assets_root,lexical
from tools.projectile_oracle.guided_step import create,i32,vec,xyz
from tools.spatial_oracle.building_body_rules import RULES,SP,dwords




def prepare():
 m,_,cells,init=create(False);u=m.u
 u.mem_write(0xa8ed84,dwords(0))
 for registry in (0xa8eb00,0xa83ce0):u.mem_write(registry,dwords(0x7eb6d4,m.alloc(4096),1024,1,0,10))
 typ=m.alloc(0xf00);m.invoke(0x7470d0,typ,(m.cstring('FV'),));layers=[]
 for name in ('RULESMD.INI','LANGRULE.INI','MPBattleMD.ini','Hills.map'):
  path=assets_root()/name
  if not path.exists():continue
  raw=path.read_bytes();sections,_=lexical(raw,{'FV'});m.rules_cache(sections)
  for begin,end in ((0x71284a,0x712a8f),(0x71338b,0x7133c8),(0x7147b4,0x7147ce),(0x714a49,0x714a63),(0x714016,0x714030)):
   for reg,v in ((UC_X86_REG_ESP,SP),(UC_X86_REG_EBP,typ),(UC_X86_REG_EBX,typ+0x24),(UC_X86_REG_ESI,RULES),(UC_X86_REG_EDI,RULES),(UC_X86_REG_EAX,u.mem_read(typ+0xd22,1)[0])):u.reg_write(reg,v)
   run_checked(u,begin,end)
  for reg,v in ((UC_X86_REG_ESP,SP),(UC_X86_REG_EDI,typ),(UC_X86_REG_EBP,typ+0x24),(UC_X86_REG_EBX,RULES)):u.reg_write(reg,v)
  run_checked(u,0x747b03,0x747b49)
  layers.append(dict(file=name,sha256=hashlib.sha256(raw).hexdigest(),burst_delays=list(struct.unpack('<4i',u.mem_read(typ+0xe48,16)))))
 src=m.alloc(0x1000);u.mem_write(src,dwords(0x7f5c70));u.mem_write(src+0x6c4,dwords(typ));u.mem_write(src+0x2b4,dwords(cells[16,20]));u.mem_write(src+0x90,b'\x01')
 heading=m.alloc(4)
 for offset in (0x388,0x3a0):
  m.invoke(0x4c91e0,src+offset,(0,));u.mem_write(heading,dwords(0 if offset==0x388 else 0x3fff));m.invoke(0x4c9300,src+offset,(heading,))
 u.reg_write(UC_X86_REG_ESI,src);u.reg_write(UC_X86_REG_ESP,SP);run_checked(u,0x735678,0x735691,required_addresses=(0x70dc70,))
 slot=m.invoke(0x6f3330,src,(cells[16,20],));w=m.read32(m.invoke(0x70e140,src,(slot,)))
 assert m.string(w+0x24)=='HoverMissile'
 return m,src,typ,w,cells,dict(**init,type_layers=layers,selected_slot=slot)

def launch_shot(m,source,st,w,cells,initial,origin,frame=0,bridge=False):
 u=m.u;u.mem_write(0xa8ed84,dwords(frame));p=m.read32(w+0xa0);target=cells[16,20]
 for xy,c in cells.items():u.mem_write(c+0x140,dwords(0x100 if bridge and xy==(10,20) else 0))
 scratch=m.alloc(12);m.invoke(0x486890,target,(scratch,));target_xyz=xyz(u,scratch)
 u.mem_write(SP,bytes(0x2000))
 for ptr,values in ((SP+0x40,[w]),(SP+0x44,origin),(SP+0x68,[p]),(SP+0x88,target_xyz),(SP+0x1000+8,[target,0])):u.mem_write(ptr,dwords(*values))
 for reg,value in ((UC_X86_REG_ESP,SP),(UC_X86_REG_EBP,SP+0x1000),(UC_X86_REG_ESI,source),(UC_X86_REG_EDI,m.read32(w+0xa4)),(UC_X86_REG_FPCW,NATIVE_FPCW)):u.reg_write(reg,value)
 scenario=m.read32(0xa8b230);events=[];bullet=[];visited=set();last=[]
 id_before=i32(u,scenario+0x214);scalars={}
 def observe(uc,a,n,d):
  if a>=0x20000000:raise AssertionError(('non-image-code',hex(a),'trace',last,'events',events,'bullet',bullet))
  last.append(hex(a));del last[:-15]
  if a in (0x773070,0x46b050,0x6c5090,0x466380,0x410230,0x68bcb0,0x4664c0,0x70bcb0,0x740f80,0x70e140,0x7177c0,0x70d590,0x468670,0x4e1130):visited.add(hex(a))
  if a in (0x65c640,0x65c660,0x65c780,0x65c7e0):events.append(dict(rng=hex(a),caller=hex(m.read32(u.reg_read(UC_X86_REG_ESP))),bounds=[i32(u,u.reg_read(UC_X86_REG_ESP)+4),i32(u,u.reg_read(UC_X86_REG_ESP)+8)] if a==0x65c7e0 else None))
  if a==0x68bcb0:events.append(dict(constructor_id_before=i32(u,scenario+0x214),caller=hex(m.read32(u.reg_read(UC_X86_REG_ESP)))))
  if a==0x65c84b:events.append(dict(rng_raw=u.reg_read(UC_X86_REG_ESI)))
  if a in (0x421ea0,0x62dc50,0x6b4a50,0x75e950):events.append(dict(effect_constructor=hex(a)))
  if a in (0x7258d0,0x5f65f0,0x466560,0x5f3b80):raise AssertionError(('unexpected launch detach/retirement',hex(a)))
  if a==0x46b072:
   sp=u.reg_read(UC_X86_REG_ESP);clsid,outer,context,iid,ppv=struct.unpack('<5I',u.mem_read(sp,20))
   assert (clsid,outer,context,iid)==(0x7e96e0,0,7,0x7f7c90)
   events.append(dict(call='CoCreateInstance activation to original registered factory',clsid_hex=bytes(u.mem_read(clsid,16)).hex(),iid_hex=bytes(u.mem_read(iid,16)).hex()))
   # Native stdcall factory consumes(this,outer,iid,ppv). Reuse the COM arg
   # stack with explicit native return so resulting ESP equals the COM call.
   u.mem_write(sp,dwords(0x46b078,0,outer,iid,ppv));u.reg_write(UC_X86_REG_EIP,0x6c5090)
  elif a==0x46afe5:
   # Verified PE import KERNEL32.InterlockedIncrement. The single-threaded
   # Windows API boundary updates COM refcount only; native Abstract ID remains
   # original410230/68BCB0 and is never assigned by this hook.
   sp=u.reg_read(UC_X86_REG_ESP);ptr=m.read32(sp);value=(m.read32(ptr)+1)&0xffffffff
   u.mem_write(ptr,dwords(value));u.reg_write(UC_X86_REG_EAX,value);u.reg_write(UC_X86_REG_ESP,sp+4);u.reg_write(UC_X86_REG_EIP,0x46afeb)
   events.append(dict(call='KERNEL32.InterlockedIncrement',com_refcount=value))
  elif a==0x6fe562:bullet.append(u.reg_read(UC_X86_REG_EAX))
  elif a==0x6fe53f:scalars['get_speed']=u.reg_read(UC_X86_REG_EAX)
  elif a==0x6fea52:scalars['launch_amount']=i32(u,SP+0x28)
  elif a in (0x5f4ec0,0x4a9770,0x4a9720):
   sp=u.reg_read(UC_X86_REG_ESP)
   if a==0x5f4ec0:
    u.mem_write(bullet[0]+0x9c,bytes(u.mem_read(m.read32(sp+4),12)));u.mem_write(bullet[0]+0x90,b'\x01')
   events.append(dict(call=hex(a),supplied_world_admission=True));m.ret(1,8 if a==0x5f4ec0 else 4)
 h=u.hook_add(UC_HOOK_CODE,observe)
 try:run_checked(u,0x6fe4f2,(0x6ff43f,0x6ff751,0x6ff93c),count=1000000,required_addresses=(0x6c5090,0x466380,0x68bcb0,0x70bcb0,0x740f80,0x468670))
 except Exception:
  print('LAUNCH FAILED',last,'ECX',hex(u.reg_read(UC_X86_REG_ECX)),file=sys.stderr);raise
 finally:u.hook_del(h)
 assert u.reg_read(UC_X86_REG_EIP)==0x6ff43f,hex(u.reg_read(UC_X86_REG_EIP))
 b=bullet[0]
 result=dict(native_sha256=NATIVE_SHA256,initial=initial,supplied=dict(origin=list(origin),source_heading=0x3fff,source_flags=0,weapon_slot=0,target_cell=[16,20],live_bridge_cell=[10,20] if bridge else None,binary_frame=frame),launch=dict(position=xyz(u,b+0x9c),target=target_xyz,velocity=vec(u,b+0xe8),**scalars,maximum_speed=i32(u,b+0x110),pitch=m.read32(SP+0x80)&0xffff,unique_id=i32(u,b+0x10),scenario_id_before=id_before,scenario_id_after=i32(u,scenario+0x214),course_locked=bool(u.mem_read(b+0x105,1)[0]),course_frames=i32(u,b+0x108),detector=dict(first_timer_start=i32(u,b+0xb8),first_timer_duration=i32(u,b+0xc0),arm_start=i32(u,b+0xc4),arm_duration=i32(u,b+0xcc),reference=xyz(u,b+0xd0),distance_watermark=i32(u,b+0xdc)),warhead=m.string(m.read32(b+0x128)+0x24),calls=sorted(visited),supplied_calls=events))
 result['launch']['next_burst']=i32(u,source+0x3b8);result['launch']['rearm']=[i32(u,source+0x2ec),i32(u,source+0x2f4)];return m,b,cells,result

def generate():
 m,source,typ,w,cells,initial=prepare();u=m.u
 u.reg_write(UC_X86_REG_ESP,SP);u.reg_write(UC_X86_REG_EBP,typ);run_checked(u,0x715b10,0x715f9e)
 loco=m.alloc(0x100);m.invoke(0x4af540,loco);u.mem_write(loco+0xc,dwords(source));u.mem_write(source+0x674,dwords(loco+4));u.mem_write(source+0x14,dwords(4))
 house=m.alloc(0x2000);u.mem_write(house+0x1a8,struct.pack('<d',1.0));u.mem_write(source+0x21c,dwords(house))
 u.mem_write(source+0x9c,dwords(2688,5248,800));out=m.alloc(12);scenario=m.read32(0xa8b230);m.invoke(0x65c6d0,scenario+0x218,(31,))
 rows=[];frame=0
 for shot in range(2):
  burst=i32(u,source+0x3b8);flh_calls=[]
  def observe_flh(uc,a,n,d):
   if a in (0x6f3ad0,0x70e140,0x7177c0,0x4aff60,0x55a730):flh_calls.append(hex(a))
   if a in (0x68bcb0,0x65c7e0,0x65c780):raise AssertionError(('unexpected FLH effect',hex(a)))
  h=u.hook_add(UC_HOOK_CODE,observe_flh)
  try:m.invoke(0x6f3ad0,source,(out,0,0,0,0))
  finally:u.hook_del(h)
  origin=xyz(u,out);rng_before=bytes(u.mem_read(scenario+0x218,0x3f4)).hex()
  _,b,_,row=launch_shot(m,source,typ,w,cells,initial,origin,frame)
  row['supplied']['source_flags']=4;row['supplied']['body_heading']=0;row['supplied']['source_lifecycle']='supplied Unit state; original Drive constructor and native matrix';row['supplied']['source_origin']=[2688,5248,800];row['launch']['burst_before']=burst;row['flh_calls']=flh_calls;row['rng_before']=rng_before;row['rng_after']=bytes(u.mem_read(scenario+0x218,0x3f4)).hex();rows.append(row)
  frame+=row['launch']['rearm'][1]
 return dict(native_sha256=NATIVE_SHA256,scope=__doc__,assumptions=['Partial physical type readers; UnitType concrete constructor and numbered FLH/BurstDelay readers execute','Supplied Unit source state and flat pose/body0/turret0x3fff/ownerROF1; original Drive constructor/DrawMatrix execute','Original Scenario Seed31 is supplied immediately before shot1; first call frame0 and second call at returned rearm duration4; no native Unit scheduler or intervening Bullet AI runs','Original full Weapon reader has empty sound registry; HoverMissile no muzzle Anim/Fire/Spark/Railgun systems; report sound itself not proven','Common native Bullet activation executes through native factory, Windows atomic increment supplied; ObjectUnlimbo and Display admission supplied','Stops before Sonic/post-fire tail at6FF43F; source construction/lifecycle, map placement, impact/retirement and other objects between calls remain unproved'],art_sha256=hashlib.sha256((assets_root()/'ARTMD.INI').read_bytes()).hexdigest(),flh=xyz(u,typ+0x89c),cases=rows)

def flh_controls():
 m,src,typ,w,cells,init=prepare();u=m.u;rows=[]
 cases=[('absent',{},4,1),('numbered',{'Weapon1FLH':'64,48,180'},4,1),('elite',{'Weapon1FLH':'64,48,180','EliteWeapon1FLH':'-100, +25,300tail'},4,1),('decimal_wrap',{'Weapon1FLH':'4294967297,-4294967297,2147483648'},4,1),('wrong_case',{'weapon1flh':'7,8,9'},4,1),('count_zero',{'Weapon1FLH':'7,8,9'},4,0),('ordinary',{'Weapon1FLH':'7,8,9','PrimaryFireFLH':'1,2,3'},0,1),('slot_three',{'Weapon1FLH':'1,2,3','Weapon3FLH':'4,5,6','EliteWeapon3FLH':'7,8,9'},4,3)]
 for name,keys,turret_count,weapon_count in cases:
  m.make_ini({'FV':keys});u.mem_write(typ+0x808,dwords(turret_count,weapon_count))
  for n in range(18):
   u.mem_write(typ+0x89c+n*0x1c,dwords(0,0,0));u.mem_write(typ+0xa98+n*0x1c,dwords(0,0,0))
  u.reg_write(UC_X86_REG_ESP,SP);u.reg_write(UC_X86_REG_EBP,typ);run_checked(u,0x715b10,0x715f9e)
  index=2 if name=='slot_three' else 0
  rows.append(dict(name=name,art=keys,turret_count=turret_count,weapon_count=weapon_count,index=index,normal=xyz(u,typ+0x89c+index*0x1c),elite=xyz(u,typ+0xa98+index*0x1c)))
 return rows


def corpus():
 result=generate();result['flh_controls']=flh_controls();return result


def metadata():
 out=provenance(scope=__doc__,entry_points={'unit_type_ctor':0x7470d0,'numbered_flh_reader':0x715b10,'read_coord':0x529ca0,'burst_delay_reader':0x747b03,'get_flh':0x6f3ad0,'drive_ctor':0x4af540,'drive_draw_matrix':0x4aff60,'launch_to_muzzle_gate':0x6fe4f2,'get_rof':0x6fcfa0},assumptions=['Physical retail strings supplied through native CRC caches; full physical file loader omitted','Partial FV type readers; original UnitType constructor/default BurstDelay loop and numbered FLH loop execute','Source Unit lifecycle/flat pose and HouseROF1 supplied; original Drive constructor and GetFLH execute','Scenario Seed31 and host shot frames0,4 supplied; no intervening world/Bullet AI or whole-world native-ID claim','HE constructor only, sound registry empty, no native impact/retirement or post6FF43F tail'],substitutions=['Original Bullet COM activation dispatched to actual6C5090 factory; Windows InterlockedIncrement supplied','ObjectUnlimbo and Display admission supplied; originalPE instructions unchanged'])
 out['physical_files']=[dict(name=p.name,sha256=hashlib.sha256(p.read_bytes()).hexdigest()) for name in ('RULESMD.INI','LANGRULE.INI','MPBattleMD.ini','Hills.map','ARTMD.INI') if (p:=assets_root()/name).exists()]
 return out

if __name__=='__main__':
 finish_vectors(corpus,Path(__file__).with_suffix('.json'),provenance=metadata)
