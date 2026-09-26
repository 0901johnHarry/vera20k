"""Original AreaDamage admission/receiver-dispatch receipt and IC isolation.
Receiver method is an explicit external-result boundary; AreaDamage runs whole.
"""
import hashlib,struct
from pathlib import Path
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
from tools.projectile_oracle.ifv_select_anim import setup
from tools.projectile_oracle.ifv_impact import i32,xyz
from tools.native_oracle import RET_MAGIC,NATIVE_SHA256,run_checked,finish_vectors,provenance
from tools.spatial_oracle.building_body_rules import SP,RULES,dwords
from tools.rmg_oracle.gen_rng_vectors import seeded_struct

def generate():
 m,b,wh,cell,obj,typ,initial=setup();u=m.u;m.invoke(0x561910,0)
 for x in range(6,25):
  for y in range(16,25):
   p=m.read32(0x26000000+4*(y*512+x));u.mem_write(p+0x38,dwords(-1));u.mem_write(p+0x44,dwords(-1));u.mem_write(p+0xe4,dwords(0));u.mem_write(p+0x140,dwords(0))
 u.mem_write(0x87f914,dwords(64,64));u.mem_write(0xabdc50+0x38,dwords(-1));u.mem_write(0xabdc50+0x44,dwords(-1));u.mem_write(0xa8ed84,dwords(100))
 scenario=m.read32(0xa8b230);position=m.alloc(12);u.mem_write(position,dwords(2688,5248,624));fresh=bytes(u.mem_read(wh,0x180));rows=[]
 specs=[dict(name='empty',empty=True),dict(name='dispatch_return0'),dict(name='dispatch_return1',receiver_result=1),dict(name='dispatch_return2',receiver_result=2),dict(name='dispatch_return4',receiver_result=4),dict(name='dead',alive=False),dict(name='zero_health',health=0),dict(name='limbo',limbo=True),dict(name='unmarked',marked=False),dict(name='zero_damage',damage=0),dict(name='scenario_no_damage',scenario_flags=0x20),dict(name='null_warhead',null_wh=True)]
 for distance in (0,84,85):
  for spread in ('0.5','0.5001'):
   specs.append(dict(name=f'ic_distance{distance}_spread{spread}',ic=True,distance=distance,spread=spread))
 specs += [dict(name='ic_kind1',ic=True,ic_kind=1),dict(name='ic_expired',ic=True,duration=0),dict(name='ic_elapsed_equal',ic=True,start=90,duration=10),dict(name='ic_elapsed_inside',ic=True,start=90,duration=11)]
 for overrides in specs:
  spec=dict(damage=25,health=100,alive=True,marked=True,limbo=False,ic=False,distance=0,spread='0.5',receiver_result=0,scenario_flags=0,start=100,duration=10,ic_kind=0);spec.update(overrides)
  u.mem_write(wh,fresh);m.rules_cache({'HE':{'CellSpread':spec['spread']}});m.invoke(0x75d3a0,wh,(RULES,))
  u.mem_write(cell+0xe4,dwords(0 if spec.get('empty') else obj));u.mem_write(obj,dwords(0x7f5c70));u.mem_write(obj+0x14,dwords(1));u.mem_write(obj+0x30,dwords(0));u.mem_write(obj+0x6c,dwords(spec['health']));u.mem_write(obj+0x90,bytes((spec['alive'],)));u.mem_write(obj+0x74,bytes((spec['marked'],)));u.mem_write(obj+0x81,bytes((spec['limbo'],)));u.mem_write(obj+0x9c,dwords(2688+spec['distance'],5248,624));u.mem_write(obj+0x18c,dwords(spec['start']));u.mem_write(obj+0x194,dwords(spec['duration'] if spec['ic'] else 0));u.mem_write(obj+0x1c4,dwords(spec['ic_kind']));u.mem_write(scenario,dwords(spec['scenario_flags']));seed=seeded_struct(31);u.mem_write(scenario+0x218,seed)
  dispatch=[];flags=[];events=[]
  def observe(uc,a,n,d):
   sp=u.reg_read(UC_X86_REG_ESP)
   if a==0x737c90:
    dispatch.append(dict(receiver=hex(u.reg_read(UC_X86_REG_ECX)),damage=i32(u,m.read32(sp+4)),distance=i32(u,sp+8),supplied_result=spec['receiver_result']))
    m.ret(spec['receiver_result'],28)
   if a in (0x489968,0x4894a4,0x489b12,0x48a47e):flags.append(dict(pc=hex(a),isolated=bool(u.mem_read(sp+0x17,1)[0]),dispatched=bool(u.mem_read(sp+0x1f,1)[0])))
   if a in (0x41bf40,0x489e87):events.append(hex(a))
   if a in (0x65c640,0x65c660,0x65c780,0x65c7e0):raise AssertionError(('unexpected_rng',hex(a)))
  h=u.hook_add(UC_HOOK_CODE,observe)
  try:
   u.mem_write(SP,dwords(RET_MAGIC,0,0 if spec.get('null_wh') else wh,0,0));u.reg_write(UC_X86_REG_ESP,SP);u.reg_write(UC_X86_REG_ECX,position);u.reg_write(UC_X86_REG_EDX,spec['damage']);run_checked(u,0x489280,RET_MAGIC,count=1000000)
  finally:u.hook_del(h)
  rows.append(dict(input=spec,result=u.reg_read(UC_X86_REG_EAX),dispatch=dispatch,flags=flags,events=events,rng_unchanged=bytes(u.mem_read(scenario+0x218,len(seed)))==seed))
 return dict(native_sha256=NATIVE_SHA256,scope='Full original AreaDamage with supplied Unit occupancy/state and explicit ReceiveDamage result boundary; original collection/admission/IC timer/dispatch/continuation/receipt execute',initial=initial,rows=rows)

def metadata():
 return provenance(scope='Original AreaDamage returns0/1/2 and receiver dispatch versus IronCurtain isolation controls',assumptions=['Physical HE full reader, prepared flat level6 cells and original Unit vtable/getters. Original CellSpread reader consumes0.5/0.5001. Incoming health/flags/occupancy/XYZ/IC timer/kind are supplied; no Unit lifecycle claim.','Full AreaDamage489280 executes; original IC timer41BF40 and collector gates execute. No bridge overlay or damageable structural state. Seed31 remains unchanged. Result records receipt semantics, not actual receiver damage/kill behavior.'],substitutions=['Inherited reader archive/allocation/TLS boundaries. Unit ReceiveDamage737C90 is an explicit result boundary returning supplied0/1/2/4 with28bytecleanup; it does not mutate health. No AreaDamage, getter, timer, collection or receipt instruction replaced.'],entry_points={'area_damage':0x489280,'unit_receiver_boundary':0x737c90,'ic_timer':0x41bf40,'ground_isolation':0x489968,'air_isolation':0x4894a4,'return_isolated':0x489b3b,'return_receipt':0x48a47e})

if __name__=='__main__':finish_vectors(generate,Path(__file__).with_suffix('.json'),provenance=metadata)
