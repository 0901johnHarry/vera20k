"""Full native Bullet detonation: AreaDamage receipt, selection and nullify effect.

The Unit ReceiveDamage method is an explicit transport boundary. AreaDamage,
IC timer, SelectAnim, Scenario RNG and physical animation constructors execute.
"""
import hashlib,struct,os
from pathlib import Path
from collections import deque
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
from tools.projectile_oracle.ifv_select_anim import setup
from tools.projectile_oracle import ifv_impact as impact
from tools.native_oracle import NATIVE_SHA256,run_checked,finish_vectors,provenance
from tools.spatial_oracle.building_body_rules import SP,RULES,INI,dwords
from tools.rmg_oracle.gen_rng_vectors import seeded_struct

def execute(receiver_mode,em_effect):
 m,b,wh,cell,obj,typ,initial=setup();u=m.u;r=m.read32(0x8871e0);scenario=m.read32(0xa8b230)
 nullify_layers=[]
 for name in ('RULESMD.INI','LANGRULE.INI','MPBattleMD.ini','Hills.map'):
  p=impact.assets_root()/name
  if not p.exists():continue
  sections,_=impact.lexical(p.read_bytes(),{'General'});m.rules_cache(sections)
  u.mem_write(SP-4,dwords(128));u.reg_write(UC_X86_REG_ESP,SP-4);u.reg_write(UC_X86_REG_ECX,SP+0x50);u.reg_write(UC_X86_REG_ESI,r);u.reg_write(UC_X86_REG_EDI,RULES);run_checked(u,0x66e2af,0x66e2e5)
  selected=m.read32(r+0x350);nullify_layers.append(dict(file=name,sha256=hashlib.sha256(p.read_bytes()).hexdigest(),authored=sections.get('General',{}).get('WeaponNullifyAnim'),selected=m.string(selected+0x24) if selected else None))
 nullify=m.read32(r+0x350);assert m.string(nullify+0x24)=='IRONFX'
 asset=Path(os.environ.get('VERA20K_IFV_NULLIFY_ASSETS',str(impact.assets_root())))/'ironfx.shp'
 raw=asset.read_bytes();m.assets['IRONFX.SHP']=raw
 art,_=impact.lexical((impact.assets_root()/'ARTMD.INI').read_bytes(),{'IRONFX'});m.make_ini(art);m.asset_loaded=[];admitted=m.invoke(0x427d00,nullify,(INI,));art_state=m.result('IRONFX',nullify,admitted);art_state['assets']=m.asset_loaded
 initial.update(nullify_reference_layers=nullify_layers,art_sha256=hashlib.sha256((impact.assets_root()/'ARTMD.INI').read_bytes()).hexdigest(),bullet_constructor_combat_light=bool(u.mem_read(b+0xe0,1)[0]))
 assert art_state['art_body_read'] and art_state['raw_shp_frame_count']==10
 m.rules_cache({'HE':{'EMEffect':'yes' if em_effect else 'no'}});m.invoke(0x75d3a0,wh,(RULES,))
 for y in range(16,25):
  for x in range(6,25):
   p=m.read32(0x26000000+4*(y*512+x));u.mem_write(p+0x38,dwords(-1));u.mem_write(p+0x44,dwords(-1));u.mem_write(p+0xe4,dwords(0));u.mem_write(p+0x140,dwords(0))
 u.mem_write(0xabdc50+0x38,dwords(-1));u.mem_write(0xabdc50+0x44,dwords(-1));u.mem_write(0xa8ed84,dwords(100));u.mem_write(0xa8e9a0,b'\1');u.mem_write(0x87f914,dwords(64,64))
 for start,end in ((0x40b540,0x40b5ab),(0x725850,0x725886),(0x4e6d60,0x4e6d96)):
  u.reg_write(UC_X86_REG_ESP,SP);run_checked(u,start,end)
 live=[2688,5248,624];placement=[2688,5248,624];position=m.alloc(12);u.mem_write(position,dwords(*placement));u.mem_write(b+0x9c,dwords(*live));u.mem_write(b+0x90,b'\1')
 u.mem_write(cell+0xe4,dwords(0 if receiver_mode=='empty' else obj));u.mem_write(obj+0x14,dwords(1));u.mem_write(obj+0x30,dwords(0));u.mem_write(obj+0x6c,dwords(100));u.mem_write(obj+0x90,b'\1');u.mem_write(obj+0x74,b'\1');u.mem_write(obj+0x81,b'\0');u.mem_write(obj+0x9c,dwords(*placement));u.mem_write(obj+0x18c,dwords(100));u.mem_write(obj+0x194,dwords(10 if receiver_mode=='iron_curtain' else 0));u.mem_write(obj+0x1c4,dwords(0))
 seed=seeded_struct(31);rng=scenario+0x218;u.mem_write(rng,seed);events=[];trace=deque(maxlen=20);pending={}
 id_before=impact.i32(u,scenario+0x214)
 def rng_hash():return hashlib.sha256(bytes(u.mem_read(rng,len(seed)))).hexdigest()
 def event(kind,**kw):events.append(dict(event=kind,**kw))
 def obs(uc,a,n,d):
  trace.append(a);sp=u.reg_read(UC_X86_REG_ESP)
  if a in pending:
   kind=pending.pop(a);value=u.reg_read(UC_X86_REG_EAX);kw=dict(result=value,pc=hex(a),rng_hash=rng_hash())
   if kind=='SelectAnim':kw['selected']=m.string(value+0x24) if value else None
   if kind=='AnimCtor':kw.update(native_id=impact.i32(u,value+0x10),name=m.string(m.read32(value+0xc8)+0x24),retained=impact.xyz(u,value+0x9c),alive=u.mem_read(value+0x90,1)[0])
   event(kind+'_return',**kw)
  if a==0x737c90:event('ReceiveDamageBoundary',damage=impact.i32(u,m.read32(sp+4)),distance=impact.i32(u,sp+8),supplied_result=0);m.ret(0,28)
  if a in (0x489280,0x48a4f0,0x421ea0,0x65c7e0):
   kind={0x489280:'AreaDamage',0x48a4f0:'SelectAnim',0x421ea0:'AnimCtor',0x65c7e0:'RandomRanged'}[a];pending[m.read32(sp)]=kind;kw=dict(pc=hex(a),return_pc=hex(m.read32(sp)),rng_hash=rng_hash())
   if kind=='AreaDamage':kw.update(position=impact.xyz(u,u.reg_read(UC_X86_REG_ECX)),damage=u.reg_read(UC_X86_REG_EDX))
   if kind=='SelectAnim':kw.update(position=impact.xyz(u,m.read32(sp+8)),land=impact.i32(u,sp+4),damage=u.reg_read(UC_X86_REG_ECX))
   if kind=='AnimCtor':kw.update(name=m.string(m.read32(sp+4)+0x24),position=impact.xyz(u,m.read32(sp+8)),delay=impact.i32(u,sp+12),loop=impact.i32(u,sp+16),flags=m.read32(sp+20),z_adjust=impact.i32(u,sp+24),reverse=m.read32(sp+28))
   if kind=='RandomRanged':kw.update(low=impact.i32(u,sp+4),high=impact.i32(u,sp+8))
   event(kind,**kw)
  if a in (0x65c84b,0x65c79d):event('RawWord',pc=hex(a),word=u.reg_read(UC_X86_REG_ESI))
  if a in (0x48a620,0x46a2a1,0x5f4ec0,0x4a9720,0x424ce0):event({0x48a620:'CombatLight',0x46a2a1:'NullifyBranch',0x5f4ec0:'Unlimbo',0x4a9720:'DisplaySubmit',0x424ce0:'AnimStart'}[a],pc=hex(a))
 h=u.hook_add(UC_HOOK_CODE,obs)
 try:m.invoke(0x4690b0,b,(position,))
 except Exception:
  print('FAILED',list(map(hex,trace)),events);raise
 finally:u.hook_del(h)
 def only(name):
  found=[e for e in events if e['event']==name];assert len(found)==1,(name,found);return found[0]
 area=only('AreaDamage_return');selected=only('SelectAnim_return');ctor=only('AnimCtor');constructed=only('AnimCtor_return')
 assert events.index(area)<events.index(selected)<events.index(ctor)<events.index(constructed)
 assert selected['rng_hash']==ctor['rng_hash']==constructed['rng_hash']==rng_hash()
 assert constructed['alive']==1 and constructed['retained']==placement
 return dict(input=dict(name=f'{receiver_mode}_em_{str(em_effect).lower()}',receiver_mode=receiver_mode,em_effect=em_effect,live=live,placement=placement,binary_frame=100,ic_start=100,ic_duration=10 if receiver_mode=='iron_curtain' else 0,ic_kind=0,receiver_health=100,receiver_result_boundary=0,scenario_rng_seed=31),initial=initial,art=art_state,area_result=area['result'],selected=selected['selected'],constructed=constructed['name'],constructor=ctor,events=events,rng_before_sha256=hashlib.sha256(seed).hexdigest(),rng_after_sha256=rng_hash(),anim_count=impact.i32(u,0xa8e9b8),native_cursor_before=id_before,native_cursor_after=impact.i32(u,scenario+0x214),bullet_native_id=impact.i32(u,b+0x10),bullet_alive=bool(u.mem_read(b+0x90,1)[0]))

def generate():
 rows=[execute(mode,em) for mode in ('receiver','empty','iron_curtain') for em in (False,True)]
 initial=rows[0].pop('initial');art=rows[0].pop('art')
 for row in rows[1:]:assert row.pop('initial')==initial and row.pop('art')==art
 assert {row['area_result'] for row in rows}=={0,1,2}
 return dict(native_sha256=NATIVE_SHA256,initial=initial,nullify_art=art,rows=rows)

def metadata():
 return provenance(scope='Six full native Bullet detonation controls joining AreaDamage0/1/2, EMEffect selector RNG and ordinary versus WeaponNullifyAnim construction',assumptions=[
  'Full original4690B0 runs from entry to return, including full AreaDamage489280, original IC timer/admission/receipt, Bullet selection caller, full SelectAnim48A4F0 and full AnimCtor421EA0/Start/Unlimbo/Display submission. AreaDamage return is observed, never supplied.',
  'Physical HoverMissile/AAHeatSeeker2 and HE reader setup is inherited from ifv_select_anim. Full original HE reader consumes explicit EMEffect=no/yes controls; ordinary physical HE has EMEffect=false. General WeaponNullifyAnim reads physical ordered RULESMD, optional LANGRULE, MPBattleMD and Hills layers through original66E2AF block. This is not full Rules Process chronology.',
  'Full physical IRONFX ART and unchanged10-frame SHP load through native reader427D00; the eight ordinary HE animation types/assets come from ifv_impact.prepare. Sound registry is empty, so Report remains-1 and audio binding/playback is outside this witness.',
  'Prepared flat level6 mapped cells have no bridge/tile/overlay damage effects. Live BulletXYZ and detonation placement are both2688,5248,624. Supplied GameActive1 permits native animation admission; binary frame100, seed31, receiver health100, alive/marked/nonlimbo membership and IC start100/duration0or10/kind0 are explicit inputs. No source/receiver gameplay lifecycle or full Unit damage mutation is claimed.',
  'Bullet constructor CombatLight+E0 remains false. These controls establish selection/animation order and do not exercise the ordinary CombatLight body.',
  'Native partial construction cursor before/after is recorded, not a whole-scenario prefix. Detonate alone does not schedule Bullet retirement or subsequent AnimAI; retained Bullet alive and one newly admitted animation are bounded outputs. Existing joined impact fixtures cover their ordinary lifecycle.',
 ],substitutions=[
  'Inherited lexical CRC INI caches, exact archive byte transport, allocator/delete/CRT/TLS boundaries. Unit ReceiveDamage737C90 returns explicit0 with28bytecleanup and no health mutation; native AreaDamage independently produces its receipt. No detonation/AreaDamage/selector/RNG/animation instruction is replaced.',
 ],entry_points={'detonate':0x4690b0,'area_damage':0x489280,'receiver_boundary':0x737c90,'select_anim':0x48a4f0,'random_ranged':0x65c7e0,'nullify_branch':0x46a2a1,'anim_ctor':0x421ea0,'anim_start':0x424ce0,'object_unlimbo':0x5f4ec0,'nullify_anim_reader':0x66e2af,'anim_type_reader':0x427d00})

if __name__=='__main__':finish_vectors(generate,Path(__file__).with_suffix('.json'),provenance=metadata)
