"""Chubby five-digit raccoon-like hands, welded to the accepted forearms."""
import math
import heapq
import bpy
import bmesh
from mathutils import Vector
from mathutils.bvhtree import BVHTree
from mathutils.geometry import barycentric_transform
from hearthling_wrist import cut_distal_wrist


def smooth(t):
    t = max(0.0, min(1.0, t))
    return t*t*(3-2*t)


def refine_hands(triangles_per_hand=1150):
    rig = bpy.data.objects['Hearthling_Rig']
    assert not rig.get('raccoon_hand_revision'), 'Use the pre-hand checkpoint'
    body = bpy.data.objects['Hearthling_Body']
    previous = rig.data.pose_position
    rig.data.pose_position = 'REST'
    bpy.context.view_layer.update()
    donor = body.data.copy()
    donor.calc_loop_triangles()
    positions = [v.co.copy() for v in donor.vertices]
    triangles = [tuple(t.vertices) for t in donor.loop_triangles]
    tree = BVHTree.FromPolygons(positions, triangles, all_triangles=True)
    group_names = {g.index:g.name for g in body.vertex_groups}
    donor_weights = [{group_names[g.group]:g.weight for g in v.groups} for v in donor.vertices]
    original_boundary = None
    coat = bpy.data.materials['Hearthling | vertex-colored coat']
    skin = bpy.data.materials['Hearthling | mouth and nose']
    brown = tuple(next(n for n in skin.node_tree.nodes if n.type == 'BSDF_PRINCIPLED').inputs['Base Color'].default_value)
    for ob in list(bpy.data.objects):
        if ob.type == 'MESH' and ob.name.startswith(('Hearthling_Finger','Hearthling_Thumb')):
            bpy.data.objects.remove(ob, do_unlink=True)
    report = []
    for side, mirror in [('L',-1),('R',1)]:
        bone = rig.data.bones['hand.'+side]
        matrix = bone.matrix_local.copy()
        inverse = matrix.inverted()
        axis = (bone.tail_local-bone.head_local).normalized()
        group_index = body.vertex_groups['hand.'+side].index
        bm = bmesh.new()
        bm.from_mesh(body.data)
        ring_layer=bm.verts.layers.int.get('_RaccoonWristRing') or bm.verts.layers.int.new('_RaccoonWristRing')
        if original_boundary is None:
            original_boundary = sorted(tuple(sorted(tuple(round(c,6) for c in v.co) for v in e.verts)) for e in bm.edges if e.is_boundary)
        probe = bm.copy()
        wrist_points = cut_distal_wrist(probe,bone,group_index,cut_y=-.055)['wrist_points']
        probe.free()
        wrist = cut_distal_wrist(bm,bone,group_index,cut_y=-.11)
        for vertex in wrist['wrist_vertices']:vertex[ring_layer]=1 if side=='L' else 2
        # Narrow the retained collar around its own center, not the hand axis.
        # Exact ring seeds and arm-dominant weights exclude nearby torso surfaces.
        bm.verts.index_update()
        proximal=[inverse@v.co for v in wrist['wrist_vertices']]
        collar=Vector(((min(p.x for p in proximal)+max(p.x for p in proximal))/2,
                       -.11,(min(p.z for p in proximal)+max(p.z for p in proximal))/2))
        deform=bm.verts.layers.deform.active
        arm_ids=[body.vertex_groups[name+'.'+side].index for name in ['forearm','upper_arm']]
        distance={v:0.0 for v in wrist['wrist_vertices']}
        pending=[(0.0,v.index,v) for v in wrist['wrist_vertices']]
        heapq.heapify(pending)
        while pending:
            d,_,v=heapq.heappop(pending)
            if d!=distance[v]:continue
            for edge in v.link_edges:
                other=edge.other_vert(v)
                if sum(other[deform].get(i,0) for i in arm_ids)<.80:continue
                candidate=d+edge.calc_length()
                if candidate<.06 and candidate<distance.get(other,float('inf')):
                    distance[other]=candidate;heapq.heappush(pending,(candidate,other.index,other))
        for vertex,d in distance.items():
            strength=1-smooth(d/.06)
            local=inverse@vertex.co
            local.x=collar.x+(local.x-collar.x)*(1-.35*strength)
            local.z=collar.z+(local.z-collar.z)*(1-.35*strength)
            vertex.co=matrix@local
        offset=Vector(((min(p.x for p in wrist_points)+max(p.x for p in wrist_points))/2,0,(min(p.z for p in wrist_points)+max(p.z for p in wrist_points))/2))
        print('WRIST_RING',side,len(wrist_points),'offset',list(offset),flush=True)
        bm.to_mesh(body.data)
        bm.free()
        parts = []

        def mass(name,center,scale):
            bpy.ops.mesh.primitive_uv_sphere_add(segments=24,ring_count=16)
            ob = bpy.context.object
            ob.name = 'RaccoonHand_'+side+'_'+name
            for v in ob.data.vertices:
                v.co = matrix @ (offset+Vector((mirror*(center[0]+v.co.x*scale[0]),center[1]+v.co.y*scale[1],center[2]+v.co.z*scale[2])))
            if mirror<0:
                bm=bmesh.new();bm.from_mesh(ob.data);bmesh.ops.reverse_faces(bm,faces=list(bm.faces));bm.to_mesh(ob.data);bm.free()
            parts.append(ob)

        def digit(name,a,b,c,radius):
            a,b,c = [Vector((mirror*p[0],p[1],p[2])) for p in [a,b,c]]
            vs=[];fs=[];sides=12;rings=17
            across=Vector((1,0,0))
            for i in range(rings):
                t=i/(rings-1)
                center=(1-t)**2*a+2*(1-t)*t*b+t*t*c
                direction=((1-t)*(b-a)+t*(c-b)).normalized()
                across=(across-direction*across.dot(direction)).normalized()
                around=direction.cross(across)
                r=radius*(.92+.08*math.sin(t*math.pi))
                if t>.76:r*=math.sqrt(max(.0001,1-((t-.76)/.24)**2))
                for j in range(sides):
                    angle=j*math.tau/sides
                    vs.append(tuple(matrix @ (offset+center+r*(across*math.cos(angle)+around*math.sin(angle)))))
                if i:
                    for j in range(sides):
                        n=i*sides+j;m=i*sides+(j+1)%sides
                        fs.append((n-sides,m-sides,m,n))
            fs += [tuple(reversed(range(sides))),tuple(range(len(vs)-sides,len(vs)))]
            data=bpy.data.meshes.new(name);data.from_pydata(vs,[],fs);data.update()
            ob=bpy.data.objects.new('RaccoonHand_'+side+'_'+name,data)
            bpy.context.collection.objects.link(ob);parts.append(ob)

        mass('wrist',(0,-.020,0),(.085,.095,.050))
        mass('palm',(0,.090,-.015),(.105,.075,.055))
        mass('thenar',(.075,.077,-.040),(.052,.057,.045))
        for i,(x,end_x,length,r) in enumerate(zip([-.073,-.024,.025,.072],[-.103,-.035,.036,.103],[.115,.147,.153,.122],[.027,.029,.030,.027])):
            digit('finger'+str(i),(x,.119,-.014),(end_x,.119+length*.65,-.002),(end_x,.119+length,-.045),r)
        digit('side-digit',(.090,.055,-.015),(.143,.095,-.020),(.151,.168,-.045),.028)
        bpy.ops.object.select_all(action='DESELECT')
        for ob in parts:ob.select_set(True)
        bpy.context.view_layer.objects.active=parts[0]
        bpy.ops.object.join()
        hand=bpy.context.object
        rem=hand.modifiers.new('Connected palm and five fingers','REMESH')
        rem.mode='VOXEL';rem.voxel_size=.0045;rem.use_smooth_shade=True
        bpy.ops.object.modifier_apply(modifier=rem.name)
        bm=bmesh.new();bm.from_mesh(hand.data)
        for _ in range(2):
            bmesh.ops.smooth_vert(bm,verts=list(bm.verts),factor=.22,use_axis_x=True,use_axis_y=True,use_axis_z=True)
        bm.to_mesh(hand.data);bm.free()
        hand.data.calc_loop_triangles()
        dec=hand.modifiers.new('Hand surface budget','DECIMATE')
        dec.ratio=min(1,(triangles_per_hand-60)/len(hand.data.loop_triangles))
        bpy.ops.object.modifier_apply(modifier=dec.name)
        bm=bmesh.new();bm.from_mesh(hand.data)
        bmesh.ops.bisect_plane(bm,geom=list(bm.verts)+list(bm.edges)+list(bm.faces),
            plane_co=matrix @ Vector((0,.015,0)),plane_no=axis,
            clear_inner=True,clear_outer=False)
        bm.to_mesh(hand.data);bm.free()
        hand.data.materials.clear();hand.data.materials.append(coat)
        for name in group_names.values():hand.vertex_groups.new(name=name)
        for v in hand.data.vertices:
            hand.vertex_groups['hand.'+side].add([v.index],1.0,'REPLACE')
        new_uv=hand.data.uv_layers.new(name=body.data.uv_layers.active.name)
        bpy.ops.object.select_all(action='DESELECT')
        hand.select_set(True);bpy.context.view_layer.objects.active=hand
        bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT')
        bpy.ops.uv.smart_project(angle_limit=math.radians(66),island_margin=.01)
        bpy.ops.object.mode_set(mode='OBJECT')
        new_uv=hand.data.uv_layers.active
        for loop in new_uv.data:loop.uv=loop.uv*(48/2048)+Vector((8/2048,8/2048))
        rings=hand.data.attributes.new(name='_RaccoonWristRing',type='INT',domain='POINT')
        for vertex in hand.data.vertices:
            if abs((inverse@vertex.co).y-.015)<1e-5:rings.data[vertex.index].value=3 if side=='L' else 4
        region=hand.data.attributes.new(name='RaccoonHandRegion',type='INT',domain='FACE')
        for value in region.data:value.value=1 if side=='L' else 2
        attr=hand.data.color_attributes.new(name='CoatColor',type='FLOAT_COLOR',domain='POINT')
        for value in attr.data:value.color=brown
        hand.data.color_attributes.active_color=attr
        assert all(max(abs(value.color[k]-brown[k]) for k in range(4))<1e-6 for value in attr.data)
        for p in hand.data.polygons:p.use_smooth=True
        bpy.ops.object.select_all(action='DESELECT')
        body.select_set(True);hand.select_set(True);bpy.context.view_layer.objects.active=body
        bpy.ops.object.join()
        bm=bmesh.new();bm.from_mesh(body.data)
        ring_layer=bm.verts.layers.int['_RaccoonWristRing']
        ring_ids={1,3} if side=='L' else {2,4}
        boundary=[e for e in bm.edges if e.is_boundary and all(v[ring_layer] in ring_ids for v in e.verts)]
        selected=set(boundary)
        assert all(sum(edge in selected for edge in vertex.link_edges)==2
                   for edge in boundary for vertex in edge.verts), ('Open wrist ring',side)
        assert boundary,('Missing wrist loops',side)
        remaining=set(v for e in boundary for v in e.verts);loops=[]
        while remaining:
            todo=[remaining.pop()];loop=[]
            while todo:
                v=todo.pop();loop.append(v)
                for e in v.link_edges:
                    if e not in boundary:continue
                    other=e.other_vert(v)
                    if other in remaining:remaining.remove(other);todo.append(other)
            loops.append(loop)
        print('WRIST_LOOPS',side,[(len(l),[(min((inverse@v.co)[k] for v in l),max((inverse@v.co)[k] for v in l)) for k in range(3)]) for l in loops],flush=True)
        assert len(loops)==2,('Expected exactly two wrist rings',side,[len(l) for l in loops])
        bridge=bmesh.ops.bridge_loops(bm,edges=boundary)['faces']
        coat_index=next(i for i,m in enumerate(body.data.materials) if m==coat)
        uv=bm.loops.layers.uv.active
        region=bm.faces.layers.int.get('RaccoonHandRegion') or bm.faces.layers.int.new('RaccoonHandRegion')
        bm.normal_update()
        for f in bridge:
            f.material_index=coat_index;f.smooth=True
            f[region]=1 if side=='L' else 2
            # A tiny neutral-atlas patch is enough for the smooth wrist skin.
            normal=f.normal;drop=max(range(3),key=lambda k:abs(normal[k]));axes=[k for k in range(3) if k!=drop]
            center=f.calc_center_median()
            for loop in f.loops:loop[uv].uv=(.015+(loop.vert.co[axes[0]]-center[axes[0]])*.03,.015+(loop.vert.co[axes[1]]-center[axes[1]])*.03)
        lengthwise={edge for face in bridge for edge in face.edges
                    if abs((inverse@edge.verts[0].co).y-(inverse@edge.verts[1].co).y)>.025}
        bmesh.ops.subdivide_edges(bm,edges=list(lengthwise),cuts=1,use_grid_fill=True)
        deform=bm.verts.layers.deform.active
        forearm=body.vertex_groups['forearm.'+side].index
        for face in bm.faces:
            if face[region]!=(1 if side=='L' else 2):continue
            if not all(-.11001<=(inverse@v.co).y<=.01501 for v in face.verts):continue
            face.smooth=True
            for vertex in face.verts:
                local_y=(inverse@vertex.co).y
                if local_y < -.10999:continue
                t=smooth((local_y+.055)/.07)
                weights={}
                if local_y < -.055:
                    nearest,_,tid,_=tree.find_nearest(vertex.co)
                    ids=triangles[tid];a,b,c=(positions[j] for j in ids)
                    bary=barycentric_transform(nearest,a,b,c,Vector((1,0,0)),Vector((0,1,0)),Vector((0,0,1)))
                    bary=Vector(tuple(max(0,w) for w in bary));bary/=sum(bary)
                    for j,w in zip(ids,bary):
                        for name,weight in donor_weights[j].items():
                            index=body.vertex_groups[name].index
                            weights[index]=weights.get(index,0)+w*weight
                else:
                    weights={forearm:1-t,group_index:t}
                for key in list(vertex[deform].keys()):del vertex[deform][key]
                total=sum(weights.values())
                for key,value in weights.items():
                    if value>1e-8:vertex[deform][key]=value/total
        # Subdivision leaves some five-sided wrist faces. Ear-clip tessellation
        # can choose three collinear edge samples; make the nondegenerate
        # diagonal choice explicit in the editable source before glTF export.
        wrist_faces=[face for face in bm.faces
                     if face[region]==(1 if side=='L' else 2)
                     and all(-.11001<=(inverse@v.co).y<=.01501 for v in face.verts)]
        bmesh.ops.triangulate(bm,faces=wrist_faces,quad_method='BEAUTY',ngon_method='BEAUTY')
        bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces))
        bm.to_mesh(body.data);bm.free()
        report.append({'side':side,'digits':5,'bridge_faces':len(bridge)})
    body.data.attributes.remove(body.data.attributes['_RaccoonWristRing'])
    body.data.update();body.data.calc_loop_triangles()
    bm=bmesh.new();bm.from_mesh(body.data)
    boundary=sorted(tuple(sorted(tuple(round(c,6) for c in v.co) for v in e.verts)) for e in bm.edges if e.is_boundary)
    assert boundary==original_boundary,('Introduced hand/wrist boundary edges',len(original_boundary),len(boundary))
    bm.free()
    rig['raccoon_hand_revision']=1
    rig.data.pose_position=previous;bpy.context.view_layer.update()
    bpy.data.meshes.remove(donor)
    return {'body_triangles_after':len(body.data.loop_triangles),'hands':report}
