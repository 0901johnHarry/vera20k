"""Bounded original Anim bounce-contact -> Terrain damage and immediate death cleanup.

All substitutions and fixture state are declared in the companion metadata.
"""
import json,struct
from pathlib import Path
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
from tools.spatial_oracle.bridge_damage_admission import *
OBJ=MEM+0x30000
TYPE=MEM+0x31000
DAMAGE=MEM+0x32000
DELETE=MEM+0x33000
ANIM=MEM+0x35000
ATYPE=MEM+0x36000
OBJ2=MEM+0x37000
C4=MEM+0x38000

def run(case):
 u=base(dict(flags=0,level=2,anchor_overlay=-1))
 call(u,0x49f2f0)
 neighbors=[]
 for i,(dx,dy) in enumerate(struct.iter_unpack('<hh',u.mem_read(0x89f688,32))):
  ptr=MEM+0x50000+i*0x200;neighbors.append(ptr)
  u.mem_write(ptr,words(0x7e4eec));u.mem_write(ptr+0x24,struct.pack('<hh',10+dx,20+dy));u.mem_write(ptr+0x44,words(-1));u.mem_write(ptr+0x122,b'\x01')
  u.mem_write(TABLE+((20+dy)*512+10+dx)*4,words(ptr))
 u.mem_write(0x89E7C0,words(104));u.mem_write(0x87F914,words(64,64))
 u.mem_write(0x8871E0,words(RULES));u.mem_write(0xA8B230,words(SCENARIO))
 u.mem_write(SCENARIO+0x218,seed_bytes(1));u.mem_write(RULES+0x16c8,words(10000));u.mem_write(RULES+0x1708,struct.pack('<d',0.5))
 u.mem_write(RULES+0xFA8,words(C4));u.mem_write(C4+0xa0,struct.pack('<11d',*([1.0]*11)))
 u.mem_write(0xA8ED84,words(1000))
 u.mem_write(0xAC1398,words(1,0x7fff7fff))
 u.mem_write(0xB0F69C,words(DELETE,32));u.mem_write(0xB0F6A8,words(0))
 u.mem_write(OBJ,words(0x7F522C));u.mem_write(OBJ+0x14,words(2));u.mem_write(OBJ+0x6c,words(case.get('health',10)))
 u.mem_write(OBJ+0x83,b'\x00');u.mem_write(OBJ+0x90,b'\x01');u.mem_write(OBJ+0x98,b'\x01')
 u.mem_write(0xA8E9A0,b'\x01')
 u.mem_write(OBJ+0x9c,words(2688,5248,208));u.mem_write(OBJ+0xc8,words(TYPE))
 u.mem_write(TYPE,words(0x7f5458));u.mem_write(TYPE+0x2b8,words(MEM+0x34000));u.mem_write(MEM+0x34000,words(0,0x7fff7fff));u.mem_write(TYPE+0x2a8,words(4,4));u.mem_write(TYPE+0x234,b'\x01\x01');u.mem_write(TYPE+0xa0,words(200));u.mem_write(TYPE+0x9c,words(6));u.mem_write(TYPE+0x233,bytes([case.get('immune',0)]));u.mem_write(TYPE+0x2b1,bytes([case.get('spawns',0)]))
 u.mem_write(WARHEAD+0x147,bytes([case.get('wood',1)]));u.mem_write(WARHEAD+0xa0,struct.pack('<11d',1,1,1,.7,.7,.35,.75,.4,.2,.8,1));u.mem_write(WARHEAD+0x124,struct.pack('<f',.5));u.mem_write(WARHEAD+0x12c,struct.pack('<f',.5))
 u.mem_write(DAMAGE,words(case.get('damage',20)));u.mem_write(CELL+0xe4,words(OBJ));u.mem_write(CELL+0x124,words(0xfc));u.mem_write(CELL+0x122,b'\x00')
 u.mem_write(0xB0CD48,struct.pack('<Q',0x3FC25E5374344960))
 if case.get('contact'):
  u.mem_write(ANIM,words(0x7E3354));u.mem_write(ANIM+0xC8,words(ATYPE));u.mem_write(ANIM+0x90,b'\x01')
  u.mem_write(ANIM+0x9c,words(2688+case.get('offset_x',0),5248+case.get('offset_y',0),208));u.mem_write(ANIM+0x140,struct.pack('<3f',2688.0+case.get('offset_x',0),5248.0+case.get('offset_y',0),208.0))
  u.mem_write(ATYPE+0x2a8,struct.pack('<d',case.get('damage',20)))
  u.mem_write(ATYPE+0x330,words(0 if case.get('null_warhead') else WARHEAD,case.get('radius',80)))
  if case.get('second'):
   u.mem_write(OBJ2,bytes(u.mem_read(OBJ,0x200)));u.mem_write(OBJ2+0x6c,words(200));u.mem_write(OBJ+0x30,words(OBJ2))
 u.mem_write(0x87f778,words(0x7e18fc,MEM+0x3a000,4))
 u.mem_write(0x87f788,words(2 if case.get('second') else 1))
 u.mem_write(MEM+0x3a000,words(OBJ,OBJ2))
 events=[];recent=[];heap=MEM+0x40000;substitutions=[];receiver_results=[];pending_receivers=[];rng_calls=[]
 rng_before=bytes(u.mem_read(SCENARIO+0x218,0x3f4))
 def ret(value,cleanup):
  sp=u.reg_read(UC_X86_REG_ESP);dest=read32(u,sp)
  u.reg_write(UC_X86_REG_EAX,value);u.reg_write(UC_X86_REG_ESP,sp+4+cleanup);u.reg_write(UC_X86_REG_EIP,dest)
 def hook(uc,a,s,d):
  nonlocal heap
  recent.append(a)
  if len(recent)>32:recent.pop(0)
  sp=u.reg_read(UC_X86_REG_ESP)
  if a in (0x47ea90,0x71c070,0x71b920,0x5f5390,0x5f5280,0x7258d0,0x5f65f0,0x71c930,0x5f4d30,0x71bfb0,0x5687f0,0x489280,0x5f42f0,0x422b80):
   e=dict(a=f'{a:08X}',obj=f'{u.reg_read(UC_X86_REG_ECX):08X}',caller=f'{read32(u,sp):08X}',first_health=signed(read32(u,OBJ+0x6c)));events.append(e)
   if a==0x71b920:
    dp=read32(u,sp+4);e.update(damage=signed(read32(u,dp)),distance=signed(read32(u,sp+8)),args=[read32(u,sp+i*4) for i in range(1,8)]);pending_receivers.append((read32(u,sp),u.reg_read(UC_X86_REG_ECX),dp))
   elif a==0x489280:
    e.update(coord=list(struct.unpack('<3i',u.mem_read(u.reg_read(UC_X86_REG_ECX),12))),damage=signed(u.reg_read(UC_X86_REG_EDX)),args=[read32(u,sp+i*4) for i in range(1,5)])
  if pending_receivers and a==pending_receivers[-1][0]:
   _,obj,dp=pending_receivers.pop();receiver_results.append(dict(obj=obj,result=signed(u.reg_read(UC_X86_REG_EAX)),damage_after=signed(read32(u,dp)),health=signed(read32(u,obj+0x6c))))
  if a in (0x65c780,0x65c7e0):rng_calls.append(dict(a=f'{a:08X}',caller=f'{read32(u,sp):08X}'))
  if a in (0x7c8e17,0x7c8b3d,0x439b00,0x71d160,0x6d2790,0x5f5850,0x439150,0x4a9770,0x47d2b0,0x56d460,0x584550,0x6551c0):
   substitutions.append(dict(a=f'{a:08X}',caller=f'{read32(u,sp):08X}',this=u.reg_read(UC_X86_REG_ECX)))
  if a==0x7c8e17:
   if read32(u,sp)==0x71b9c5:ret(0,0)
   else:
    p=heap;heap+=(read32(u,sp+4)+15)&~15;ret(p,0)
  elif a==0x7c8b3d:ret(0,0)
  elif a==0x439b00:ret(1,0)
  elif a==0x71d160:
   out=read32(u,sp+4);u.mem_write(out,words(0,0,1,1));ret(out,4)
  elif a==0x6d2790:ret(0,20)
  elif a==0x5f5850:ret(1,4)
  elif a in (0x439150,0x4a9770,0x47d2b0,0x56d460,0x584550,0x6551c0):ret(0,4)
 u.hook_add(UC_HOOK_CODE,hook)
 u.mem_write(SP,words(RET_MAGIC,DAMAGE,case.get('distance',0),0 if case.get('null_warhead') else WARHEAD,0,0,0,0));u.reg_write(UC_X86_REG_ESP,SP);u.reg_write(UC_X86_REG_ECX,ANIM if case.get('contact') else OBJ)
 try:run_checked(u,0x423930 if case.get('contact') else 0x71B920,RET_MAGIC,count=200000)
 except Exception:
  print(json.dumps(events,indent=2));print('recent',','.join(hex(a) for a in recent));raise
 if pending_receivers:
  _,obj,dp=pending_receivers.pop();receiver_results.append(dict(obj=obj,result=signed(u.reg_read(UC_X86_REG_EAX)),damage_after=signed(read32(u,dp)),health=signed(read32(u,obj+0x6c))))
 result=signed(u.reg_read(UC_X86_REG_EAX))
 rng_unchanged=rng_before==bytes(u.mem_read(SCENARIO+0x218,0x3f4))
 continuation=[call(u,0x65c780,SCENARIO+0x218) for _ in range(4)]
 return dict(case=case,events=events,receiver_results=receiver_results,substitutions=substitutions,neighbor_counts=[u.mem_read(p+0x122,1)[0] for p in neighbors],rng_unchanged=rng_unchanged,next_rng=continuation,result=result,health=signed(read32(u,OBJ+0x6c)),alive=u.mem_read(OBJ+0x90,1)[0],limbo=u.mem_read(OBJ+0x81,1)[0],ground_head=read32(u,CELL+0xe4),occupation=read32(u,CELL+0x124),delete_count=read32(u,0xB0F6A8),successor=read32(u,OBJ+0x30),second_hp=signed(read32(u,OBJ2+0x6c)),logic_count=read32(u,0x87f788),in_logic=u.mem_read(OBJ+0x98,1)[0])
def generate():
 cases=[]
 for health in (200,16,15,14,10,1,0):
  cases.append(dict(name=f'large_contact_hp_{health}',contact=True,health=health,damage=20))
 for damage in (10,0,-20):
  cases.append(dict(name=f'contact_damage_{damage}',contact=True,health=10,damage=damage))
 for gate in ('immune','wood','null_warhead'):
  cases.append(dict(name=f'contact_gate_{gate}',contact=True,health=10,damage=20,**{gate:0 if gate=='wood' else 1}))
 for radius,damage in ((80,20),(50,10)):
  for distance in (radius-1,radius,radius+1):
   cases.append(dict(name=f'contact_radius_{radius}_distance_{distance}',contact=True,health=200,damage=damage,radius=radius,offset_x=distance))
 for health in (200,10):
  cases.append(dict(name=f'linked_contact_hp_{health}',contact=True,health=health,second=True))
 cases.append(dict(name='custom_destructible_spawner_allocator_failure',contact=True,health=10,spawns=True))
 cases.append(dict(name='stock_immune_spawner',contact=True,health=200,spawns=True,immune=True))
 cases.append(dict(name='direct_receive',health=10,damage=20,distance=11))
 return dict(cases=[run(case) for case in cases])

def metadata():
 return provenance(scope='Original423930 bounce-contact admission through original71B920 Terrain receiver, Object damage kernel, immediate Limbo/Cell unlink/Logic removal/deferred deletion. Bounded plain-tree state; custom destructible SpawnsTiberium row supplies allocation failure and executes original nested C4 area body. Not full debris physics, animation, observers, pathfinding, or whole Terrain parity.', assumptions=[
  'Pinned retail image loaded; no executable code writes. Native x87 control word0x0E7F. Original49F2F0 initializes actual8-neighbor offsets; actual map table contains cell10,20 level2 and all8 neighboringcells. One-cell footprint; originalvtable pointers for Terrain/TerrainType/Anim/Cell/Logic; no houses/entities/bombs/animations in their global vectors.',
  'Terrain location2688,5248,208, maxHP200, armorwood6; originalTerrainCtor defaults and production AssetManager+IniFile export tools/spatial_oracle/bridge-retail-terrain-inputs.json establish relevant stock TREE01/GeneralTreeStrength200, HE and DBRIS1LG/1SM values. Input fixtures directly supply resolved fields; this harness does not execute INI readers.',
  'HE verses[1,1,1,.7,.7,.35,.75,.4,.2,.8,1], CellSpread.5/PercentAtMax.5; RulesMaxDamage10000 and yellow.5. Stock C4Warhead Super represented with allverses1, Woodfalse, spread0, emptyAnimList. NoDamagefalse; gameRunningtrue; frame1000; ScenarioRNG originalseed1.',
  'Second linked Terrain is a synthetic mixed-list continuation boundary, not legal map-placement proof. Both occupy suppliedsamecell; only first has neighbor contribution. Allfixtures use one-cell native footprint, terrain occupationbits4 (production retail TREE01), rawoccupation0xFC, Logic membershippresent, no tag/audio/activewarp effect state.',
  'Process initialized footprint sentinel supplied atAC1398; B0CD48 supplied native render-depth double bits0x3FC25E5374344960. Pending-delete buffer32 and Logicbuffer4 preallocated. Unrelated global vectors empty. Neighbor counts initially1; center count0.',
 ], substitutions=[
  '439B00 BounceClass::Update returnscontact1 with suppliedbodyfloat position. This bounds evidence after physics; original423930 reads/ftol/body and handles contact/distance/receiver/next-link logic.',
  '7C8E17 operatornew supplied bump allocator; allocation from71B9C0 alwaysfails in explicitcustomSpawns row, skippingExplosionAnim selection/construction. Other area collection allocationssucceed. 7C8B3D delete no-op.',
  '71D160 Terrain render rectangle returns supplied0,0,1,1;6D2790 TacticalDirty no-op;5F5850 ObjectMark returns1 so originalTerrainMark/ExitCell/CellRemoveContent/occupation execute.',
  '439150 empty BombListPointerGotInvalid boundary;4A9770 displayremoval boundary;47D2B0 cellrecalc,56D460 orphanZone,584550 incrementalZone,6551C0 radarDirty boundaries eachreturn0. Originalcall order logged, callees notproved.',
 ],entry_points={'contact':0x423930,'terrain_receiver':0x71B920,'object_receiver':0x5F5390,'damage_kernel':0x489180,'terrain_limbo':0x71C930,'object_limbo':0x5F4D30,'cell_remove':0x47EA90,'logic_unregister':0x55BAE0,'uninit':0x5F65F0,'area':0x489280,'neighbor_init':0x49F2F0})
if __name__=='__main__':
 finish_vectors(generate,Path(__file__).with_suffix('.json'),provenance=metadata)
