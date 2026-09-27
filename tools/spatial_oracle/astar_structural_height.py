"""Original AStar node-height prefix and selected-list fragment controls.

Execute 42A460..42A523 and 429E54..429E7F on supplied scalar Cells, node
descriptors and current heights. No CanEnter, full route or loader reachability
is claimed. Rust bridge_walkable is deliberately metadata only: native code
tests Cell+140 structural0x100 and signed ground bytes.
"""
from pathlib import Path
import hashlib,json,struct
from unicorn.x86_const import *
from tools.native_oracle import NATIVE_SHA256,RET_MAGIC,run_checked,provenance,finish_vectors
from tools.spatial_oracle.tube_hierarchy import initialized_process_fixture
from tools.spatial_oracle.map_queries import dwords,packed
from tools.spatial_oracle.guarded_arena import Arena


def execute(row):
    u,sp=initialized_process_fixture();arena=Arena(u)
    alloc=lambda n:arena.allocate(n,'supplied AStar scalar fragment input')
    pf=alloc(0xD00);node_pool=alloc(0x100004);descriptor_pool=alloc(0x180004)
    parent_cell=alloc(0x200);candidate_cell=alloc(0x200)
    parent_slot=alloc(4);candidate_slot=alloc(4);parent_descriptor=alloc(12);parent_node=alloc(16);goal=alloc(4)
    u.mem_write(pf+0x0C,dwords(descriptor_pool,node_pool));u.mem_write(pf+0x30,dwords(row['current_height']))
    for ptr,xy,key in ((parent_cell,[10,10],'parent'),(candidate_cell,[11,9],'candidate')):
        u.mem_write(ptr+0x24,packed(*xy));u.mem_write(ptr+0x11B,bytes([row[key+'_ground_raw']]))
        u.mem_write(ptr+0x140,dwords(row[key+'_flags']))
    u.mem_write(parent_slot,dwords(parent_cell));u.mem_write(candidate_slot,dwords(candidate_cell))
    u.mem_write(parent_descriptor,dwords(parent_slot,row['current_height'],0));u.mem_write(parent_node,dwords(parent_descriptor,0,0,1))
    u.mem_write(goal,packed(11,9))
    spans=((0x42A460,0x42A523),(0x429E54,0x429E7F))
    code=[bytes(u.mem_read(a,b-a))for a,b in spans]
    u.mem_write(sp,dwords(RET_MAGIC,0 if row.get('initial_node',False)else parent_node,candidate_slot,goal,0))
    u.reg_write(UC_X86_REG_ESP,sp);u.reg_write(UC_X86_REG_ECX,pf)
    run_checked(u,0x42A460,0x42A523,count=2000)
    descriptor=u.reg_read(UC_X86_REG_EDI)
    assert descriptor==descriptor_pool
    height=struct.unpack('<i',u.mem_read(descriptor+4,4))[0]
    counts=[struct.unpack('<I',u.mem_read(node_pool+0x100000,4))[0],struct.unpack('<I',u.mem_read(descriptor_pool+0x180000,4))[0]]
    assert counts==[1,1]
    u.mem_write(sp,bytes(0x80));u.mem_write(sp+0x60,b'\x7f')
    u.reg_write(UC_X86_REG_ESP,sp);u.reg_write(UC_X86_REG_ESI,pf);u.reg_write(UC_X86_REG_EBX,candidate_cell)
    run_checked(u,0x429E54,0x429E7F,count=2000)
    selected=bytes(u.mem_read(sp+0x60,1))[0];assert selected in (0,1)
    assert code==[bytes(u.mem_read(a,b-a))for a,b in spans]
    arena.guard_check()
    return dict(input=row,node_height=height,selected_list='ground'if selected else 'deck',
                selected_list_native_local=selected,pool_counters=counts,code_unchanged=True,allocation_guards_intact=True)

def rows():
    result=[]
    def add(name,pg,pflags,current,cg,cflags,**extra):
        result.append(dict(name=name,parent_ground_raw=pg,parent_flags=pflags,current_height=current,
                           candidate_ground_raw=cg,candidate_flags=cflags,**extra))
    add('ordinary_ground_hills10',10,0,10,10,0)
    for cg in (5,6,7,8,9,10,11):add(f'structural_candidate_raw{cg}_from_ground10',10,0,10,cg,0x100)
    add('candidate_walkable_only_raw6_from_ground10',10,0,10,6,0,candidate_rust_bridge_walkable=True)
    add('candidate_not_walkable_raw6_from_ground10',10,0,10,6,0,candidate_rust_bridge_walkable=False)
    add('structural_parent_on_deck_continues_structural_candidate',6,0x100,10,2,0x100)
    add('walkable_only_parent_does_not_force_deck_continuation',6,0,10,2,0x100,parent_rust_bridge_walkable=True)
    add('structural_parent_under_deck_keeps_candidate_ground',6,0x100,6,3,0x100)
    add('walkable_only_parent_allows_ground_to_deck_transition',6,0,6,3,0x100,parent_rust_bridge_walkable=True)
    add('structural_parent_deck_to_candidate_walkable_only',6,0x100,10,6,0,candidate_rust_bridge_walkable=True)
    add('raw127_structural_deck_is131',127,0x100,131,127,0x100)
    add('raw127_walkable_only_candidate_stays127',127,0x100,131,127,0,candidate_rust_bridge_walkable=True)
    add('raw128_structural_deck_is_minus124',128,0x100,-124,128,0x100)
    add('raw255_ground_to_raw252_structural_deck_is0',255,0,-1,252,0x100)
    add('signed_minus128_to127_is_large_selected_deck',128,0,-128,127,0x100)
    add('signed_minus1_to0_is_adjacent_selected_ground',255,0,-1,0,0x100)
    add('initial_node_preserves_supplied131',127,0,131,127,0,initial_node=True)
    return result

def generate():
    result=dict(schema_version=1,native_sha256=NATIVE_SHA256,scope=__doc__,cases=[execute(row)for row in rows()],
                harness_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
    return result

def metadata():
    return provenance(scope=__doc__,entry_points={'node_height_prefix_begin':0x42A460,'node_height_prefix_stop':0x42A523,'selected_list_begin':0x429E54,'selected_list_stop':0x429E7F},
                    assumptions=['Supplied Cells and parent node/descriptor data. Arena provides distinct preallocated original-size node/descriptor pools; original first allocation counters and descriptor writes execute.',
                                 'Node prefix stops before g/heuristic computation and node completion; selected-list fragment stops before hierarchy lookup. No concrete entry, closed-list admission, route reconstruction or full-search comparison.',
                                 'Raw ground bytes cover native storage-domain sign extension and deck+4. These controls do not claim map-loader or legal gameplay reachability for all values.',
                                 'Rust bridge_walkable hints are labels only and never appear in the native image. Structural Cell+140 bit0x100 alone changes the original branches.'],substitutions=[])

if __name__ == "__main__":
    finish_vectors(generate, Path(__file__).with_suffix(".json"), provenance=metadata)
