"""Original radiation boundary regression after native Verses precision correction.

This records selected final health/admission, not full radiation equivalence.
See radiation_damage_boundary.md for the observed producer precision residual.
"""
import json,struct
from pathlib import Path
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
from tools.rules_oracle.bridge_landing_inputs import Landing,WH
from tools.spatial_oracle.building_body_rules import TYPE,INI,SP,dwords
from tools.native_oracle import run_checked,RET_MAGIC,finish_vectors,provenance
from tools.spatial_oracle.estimated_damage import CLASSES
RAW='100%,100%,100%,50%,10%,10%,0%,0%,0%,100%,100%'
def execute(frame=16,cell=(6,5),armor=5,category='unit'):
 cls=CLASSES[category]
 m=Landing();u=m.u
 visits=[]
 u.hook_add(UC_HOOK_CODE,lambda _u,a,_s,_d: visits.append(a) if a in (0x65C780,0x65C7E0,0x65B9C0,0x487CB0,0x489180,0x5F5390) else None)
 wh=m.warhead('RadSite')
 m.make_ini({'RadSite':{'Verses':RAW},'Radiation':{'RadDurationMultiple':'1','RadApplicationDelay':'16','RadLevelMax':'500','RadLevelDelay':'90','RadLevelFactor':'.2','RadSiteWarhead':'RadSite'}})
 for key,(kp,a,b) in WH.items():
  u.reg_write(UC_X86_REG_ESP,SP);u.reg_write(UC_X86_REG_ESI,wh);u.reg_write(UC_X86_REG_EDI,INI);u.reg_write(UC_X86_REG_EBP,wh+0x24)
  run_checked(u,a,b)
 m.invoke(0x66CF70,TYPE,(INI,));assert m.read32(TYPE+0x1834)==wh
 site=m.alloc(0x100);table=m.alloc(0x100000);obj=m.alloc(0x1000);typ=m.alloc(0x2000);scenario=m.alloc(0x2000)
 u.mem_write(0xA8B230,dwords(scenario));u.mem_write(0x87f924,dwords(table,0x40000))
 cells={}
 for y in range(3,8):
  for x in range(3,8):
   c=m.alloc(0x200);cells[x,y]=c;u.mem_write(c,dwords(0x7E4EEC));u.mem_write(c+0x24,struct.pack('<hh',x,y));u.mem_write(c+0x44,dwords(-1));u.mem_write(table+(y*512+x)*4,dwords(c))
 u.mem_write(site+0x40,struct.pack('<hh',5,5));m.invoke(0x65B4D0,site,(2,));m.invoke(0x65B4F0,site,(500,))
 m.invoke(0x65B9C0,site)
 levelbits=struct.unpack('<Q',u.mem_read(cells[cell]+0xf0,8))[0]
 u.mem_write(obj,dwords(cls['object_vtable']));u.mem_write(obj+cls['type_offset'],dwords(typ));u.mem_write(typ,dwords(cls['type_vtable']));u.mem_write(typ+0x9c,dwords(armor));u.mem_write(typ+0xa0,dwords(300));u.mem_write(obj+0x6c,dwords(300));u.mem_write(obj+0x90,b'\1');u.mem_write(obj+0x9c,dwords(cell[0]*256+128,cell[1]*256+128,0));u.mem_write(0xA8ED84,dwords(frame))
 u.reg_write(UC_X86_REG_ESI,obj);u.reg_write(UC_X86_REG_EBX,0);u.reg_write(UC_X86_REG_ESP,SP)
 endpoint=run_checked(u,0x4DA554,(0x4DA629,0x4DA63B))
 out=dict(frame=frame,cell=cell,armor=armor,category=category,verses_bits=m.whstate(wh)['verses_bits'],factor_bits=f'{struct.unpack("<Q",u.mem_read(TYPE+0x1818,8))[0]:016x}',level_bits=f'{levelbits:016x}',level=struct.unpack('<d',u.mem_read(cells[cell]+0xf0,8))[0],admitted=endpoint==0x4DA629)
 if endpoint==0x4DA629:
  sp=u.reg_read(UC_X86_REG_ESP);damage_ptr=m.read32(sp);base=struct.unpack('<i',u.mem_read(damage_ptr,4))[0];args=[m.read32(sp+4*i) for i in range(7)]
  assert args[1:]==[0,wh,0,0,1,0],args
  out['base_damage']=base;out['cell_damaging_level']=m.invoke(0x487CB0,cells[cell])
  # Original kernel uses __fastcall: damage ECX, warhead EDX, armor/distance stack.
  u.mem_write(SP,dwords(RET_MAGIC,armor,0));u.reg_write(UC_X86_REG_ESP,SP);u.reg_write(UC_X86_REG_ECX,base);u.reg_write(UC_X86_REG_EDX,wh)
  run_checked(u,0x489180,RET_MAGIC)
  out['kernel_damage']=u.reg_read(UC_X86_REG_EAX)
  dp=m.alloc(4);u.mem_write(dp,dwords(base));out['object_receive_result']=m.invoke(0x5F5390,obj,(dp,0,wh,0,0,1,0));out['final_health']=m.read32(obj+0x6c)
 out['final_health']=m.read32(obj+0x6c)
 out['visited']=[f'{a:08X}' for a in visits]
 out['rng_calls']=[f'{a:08X}' for a in visits if a in (0x65C780,0x65C7E0)]
 assert not out['rng_calls']
 return out

def generate():
 rows=[execute(f,c,a,k) for f in (15,16,17) for c,a,k in (((5,5),0,'infantry'),((6,5),5,'unit'))]
 return dict(schema_version=1,input=dict(verses=RAW,radiation={'RadDurationMultiple':'1','RadApplicationDelay':'16','RadLevelMax':'500','RadLevelDelay':'90','RadLevelFactor':'.2','RadSiteWarhead':'RadSite'},site_level=500,spread=2,source_cell=[5,5],initial_health=300),rows=rows)

def metadata():
 return provenance(scope='Independent original RadSite warhead percentage reader, Radiation reader, flat-map RadSite spread, Foot radiation admission/producer and direct Object nonlethal receiver. Six bounded rows; no whole Radiation or concrete Techno receiver parity claim.',assumptions=[
 'Synthetic radiation_rules fixture input: eleven authored Verses tokens, RadLevelFactor .2, application delay16, cap500, level500/spread2, flat cells3..7 around5,5, fresh objecthealth300 and armor0/5. These are explicit regression inputs, not a claim of full retail layer loading.',
 'Original Warhead factory/constructor and complete selected scalar/Verses blocks75DDCC..75DE5A execute. Full original Rules Radiation reader66CF70 executes using supplied lexical INI indexes; original CRT floating scanner initialization executes. FPCW0E7F.',
 'Original spread/radius setter65B4D0, level/duration setter65B4F0 and full25-cell spread65B9C0 execute using original Cell vtables/getters and map lookup5657A0. Fresh zeroed cell radiation fields. No light activation or site AI/decay scheduling.',
 'Foot4DA554 begins after its ordinary TechnoAI/alive prologue, with valid fresh nonimmune ground Infantry/Unit objects and EBX0; stops before concrete ReceiveDamage4DA629 or at no-application continuation4DA63B. Exact damage ABI asserted: distance0, selectedwarhead, source0, ignoredefenses0, arg6true, sourcehouse0.',
 'The emitted base then independently enters original kernel489180 and original Object5F5390 nonlethal receiver, retaining actual Infantry/Unit object/type vtables. Concrete Infantry/Unit/Techno receivedamage modifiers and downstream nonlethal effects are not executed. This fixture corresponds to ordinary unranked targets with neutral effective modifiers, but does not prove those omitted gates.',
 'Rows atframes15/16/17 are independent fixtures, not native elapsed-site scheduling. No source in this comparison emits RNG draws; site duration stores are from original65B4F0. No death/detach calls are reached.',
 'Native side cell stores299.99999999999994, clamps/truncates299 and emits59. Rust currently stores300 and emits60; both resolve5 versus corrected10%heavy. Native RadLevelFactor ReadDouble bits3fc99999a0000000 differ from current directf64 parser. These are separately recorded unresolved radiation prerequisites, not matching intermediates.'],substitutions=[
 'Landing reader helper supplies cached lexical INI entries/CRC indexes, allocator bump storage, no-op delete and CRTTLS storage. No original executable instructions replaced.',
 'Fixture supplies sparse actual map cell table, explicit instance/type memory and neutral scenario/rules state. Receiver entry is selected explicitly after capturing original Foot arguments; no claim that the intervening concrete receiver body executed.'
 ],entry_points={'warhead_verses':0x75DDCC,'radiation_reader':0x66CF70,'spread_setter':0x65B4D0,'level_setter':0x65B4F0,'spread':0x65B9C0,'foot_gate':0x4DA554,'cell_level':0x487CB0,'foot_receiver_call':0x4DA629,'warhead_damage':0x489180,'object_receiver':0x5F5390})

if __name__=='__main__':finish_vectors(generate,Path(__file__).with_suffix('.json'),provenance=metadata)
