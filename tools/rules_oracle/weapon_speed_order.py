"""Original Rules constructor/Process/type-postpass/AudioVisual chronology.

Supplied cached INI objects, allocator/CRT/archive boundaries are inherited from
BulletReader. This does not execute the physical INI loader or a complete game.
Original Process is void; its incidental EAX is deliberately not a success flag.
"""
from pathlib import Path
import struct
from unicorn import UC_HOOK_CODE
from tools.native_oracle import NATIVE_SHA256,finish_vectors,provenance
from tools.projectile_oracle.bridge_render_inputs import BulletReader
from tools.spatial_oracle.building_body_rules import RULES,dwords

def s32(m,p):return struct.unpack('<i',m.u.mem_read(p,4))[0]
def fresh():
 m=BulletReader({})
 m.u.mem_write(0x887568,dwords(0x7eb6d4,m.alloc(4096),1024,1,0,10))
 w=m.invoke(0x772fa0,m.cstring('OrderProbe'))
 r=m.alloc(0x2000);m.u.mem_write(0x8871e0,dwords(r));m.invoke(0x665650,r)
 return m,w,r

def chronology(cold_gravity=None):
 m,w,r=fresh();ctor=dict(speed=s32(m,w+0xa8),gravity=s32(m,r+0x16b8))
 cold=None
 if cold_gravity is not None:
  m.rules_cache({'AudioVisual':{'Gravity':str(cold_gravity)}})
  m.invoke(0x6691e0,r,(RULES,))
  cold=dict(authored_gravity=cold_gravity,result=s32(m,r+0x16b8))
 phases={0x668ef0:'before_type_data',0x668ef5:'after_type_data',0x668f56:'before_audio_visual',0x668f5b:'after_audio_visual'}
 events=[]
 def observe(u,a,size,data):
  if a in phases:events.append(dict(at=f'{a:08x}',phase=phases[a],speed=s32(m,w+0xa8),gravity=s32(m,r+0x16b8)))
 m.u.hook_add(UC_HOOK_CODE,observe)
 rows=[]
 cases=[
  ('base_ballistic_gravity_changes_late',{'OrderProbe':{'Speed':'40','Range':'5','Projectile':'OrderBullet'},'OrderBullet':{'ROT':'0'},'AudioVisual':{'Gravity':'12'}}),
  ('absent_type_sections_still_postprocess',{'AudioVisual':{'Gravity':'24'}}),
  ('switch_guided_inherit_prior_postpass',{'OrderBullet':{'ROT':'60'},'AudioVisual':{'Gravity':'6'}}),
  ('guided_minus_one_retains',{'OrderProbe':{'Speed':'-1'}}),
  ('guided_authored_readspeed',{'OrderProbe':{'Speed':'40'}}),
  ('switch_back_ballistic_then_new_gravity',{'OrderBullet':{'ROT':'0'},'AudioVisual':{'Gravity':'3'}}),
  ('negative_rot_preserves_stored_speed',{'OrderBullet':{'ROT':'-1'}}),
  ('completely_empty_pass_preserves_nonzero_rot',{}),
  ('clear_projectile_retains_speed',{'OrderProbe':{'Projectile':'none'}}),
  ('empty_projectile_retains_null',{'OrderProbe':{'Projectile':'','Speed':'50'}}),
  ('rebind_floater_uses_prior_gravity',{'OrderProbe':{'Projectile':'OrderBullet'},'OrderBullet':{'ROT':'0','Floater':'yes'},'AudioVisual':{'Gravity':'-6'}}),
  ('negative_gravity_postpass_then_zero',{'AudioVisual':{'Gravity':'0'}}),
  ('zero_gravity_postpass_then_restore',{'AudioVisual':{'Gravity':'6'}}),
  ('empty_projectile_keeps_live_binding',{'OrderProbe':{'Projectile':'','Range':'-1'}}),
 ]
 for name,sections in cases:
  m.rules_cache(sections);events.clear();m.invoke(0x668bf0,r,(RULES,))
  assert [e['phase'] for e in events]==list(phases.values())
  rows.append(dict(name=name,cached_sections=sections,original_process_visits=list(events),result_speed=s32(m,w+0xa8),result_gravity=s32(m,r+0x16b8),get_speed_500=m.invoke(0x773070,w,(500,))))
 return dict(constructor=ctor,cold_audio_visual=cold,rows=rows)

def generate():
 return dict(native_sha256=NATIVE_SHA256,sequences=[chronology(),chronology(6)])

def metadata():
 return provenance(scope='Retained Weapon Speed and prior-pass Rules Gravity through complete original RulesClass Process',assumptions=[
  'Original RulesClass665650 and WeaponType771C70 constructors initialize Gravity3 and Speed0. Original Process668BF0 executes all its calls, including full ReadTypeData679A10, Weapon postpass7729F0 and AudioVisual6691E0. The Process function is void; incidental EAX is not treated as a success code.',
  'One Weapon is constructed before the first supplied Process, standing for prior type discovery. Unrelated native type registries are initially empty. Cached minimal source sections supply inputs, not native physical INI loading or complete active-retail type discovery.',
  'The cold_gravity6 sequence first executes full AudioVisual6691E0, matching the inspected cold caller52D132. The complete startup wrapper and destructive reset6686C0 are not executed. Their retained RulesClass lifetime still requires separate evidence.',
  'All phase observations are read-only hooks at actual Process call/return sites668EF0/668EF5/668F56/668F5B. GetSpeed773070 executes after each completed pass, so it sees the newly read Gravity while the stored postpass speed used the earlier value.',
  'Native x87 fixture uses 53-bit precision and truncation0E7F. No arithmetic, reader, postpass, or phase result is replaced with Python output.',
 ],substitutions=[
  'Inherited BulletReader supplies signed-CRC INI cache structures, allocator/delete/CRT/TLS and archive file boundaries. Unrelated Color fallback points to a supplied fixture entry; no color behavior is claimed.',
 ],entry_points={'rules_ctor':0x665650,'weapon_ctor':0x771c70,'rules_process':0x668bf0,'type_data':0x679a10,'weapon_postpass':0x7729f0,'audio_visual':0x6691e0,'read_speed':0x474810,'get_speed':0x773070})

if __name__=='__main__':
 finish_vectors(generate,Path(__file__).with_suffix('.json'),provenance=metadata)
