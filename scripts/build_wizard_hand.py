"""Approved disembodied wizard hand: metric source, bone rig and five poses.

Blender CLI, independent factory scene. Organic masses are fused before binding;
the exported mesh has explicit segment weights and rounded separate fingernails.
"""
import json
import math
import sys
from pathlib import Path
import bpy
from mathutils import Vector, Matrix, noise

sys.path.insert(0,str(Path(__file__).resolve().parent))
from island_art_common import ROOT, REVIEW, material, mesh, setup_scene, camera, render, export, reference, blueprint
from build_island_assets import Geometry

OUT=ROOT/'crates/alife_game_app/assets/hand'


def select(ob):
    bpy.ops.object.select_all(action='DESELECT');ob.select_set(True);bpy.context.view_layer.objects.active=ob


def build():
    OUT.mkdir(parents=True,exist_ok=True);REVIEW.mkdir(parents=True,exist_ok=True)
    for ob in list(bpy.data.objects):bpy.data.objects.remove(ob,do_unlink=True)
    scene=setup_scene();collection=bpy.data.collections.new('Wizard_God_Hand');scene.collection.children.link(collection)
    refs=bpy.data.collections.new('Approved_Image_References');scene.collection.children.link(refs)
    for f in ('wizard-god-hand-review-20pct.png','wizard-god-hand-wireframe-blueprint.png'):
        reference(ROOT/'target/artifacts/art-review/2026-09-30-terrain-hand-v3'/f,f,refs)
    skinmat=material('Wizard warm skin',(.62,.32,.16),True,.58)
    nailmat=material('Short rounded ivory nails',(.74,.57,.40),False,.4)
    vs=[];fs=[];rings=15;sides=24
    for j in range(rings):
        t=j/(rings-1);z=t*1.35
        width=.24+.255*math.sin(t*math.pi/2)**.65
        depth=.085+.08*math.sin(t*math.pi*.85)
        # Rounded wrist terminus; slim palm with a gentle dorsal arch.
        if j==0:width*=.75;depth*=.75
        for i in range(sides):
            a=i*math.tau/sides
            x=width*math.cos(a)
            y=depth*math.sin(a)+.025*math.sin(t*math.pi)
            vs.append((x,y,z))
    for j in range(rings-1):
        for i in range(sides):
            a=j*sides+i;b=j*sides+(i+1)%sides;fs.append((a,b,b+sides,a+sides))
    fs.extend([tuple(reversed(range(sides))),tuple((rings-1)*sides+i for i in range(sides))])
    g=Geometry();g.v=vs;g.f=fs;g.c=[(.6,.3,.15)]*len(vs)
    chains={};radii={};nails=[]
    for name,x,z,length,radius,spread in [('Index',-.345,1.29,1.55,.105,-.12),
            ('Middle',-.11,1.35,1.65,.105,.01),('Ring',.135,1.30,1.58,.095,.045),('Little',.345,1.18,1.25,.084,.11)]:
        base=Vector((x,0,z));direction=Vector((spread,-.018,1)).normalized()
        points=[base+direction*length*t for t in (0,.44,.76,1)]
        chains[name]=points;radii[name]=radius
        centers=[];rs=[]
        for j in range(22):
            t=j/21
            p=base+direction*length*t;p.y-=.022*math.sin(t*math.pi)
            knuckle=1+.13*math.exp(-((t-.44)/.065)**2)+.08*math.exp(-((t-.76)/.055)**2)
            r=radius*(1-.25*t)*knuckle
            if t>.91:r*=math.sqrt(max(.035,1-((t-.91)/.095)**2))
            centers.append(p);rs.append(r)
        # Embed roots into the palm, then rounded ring-loft fingertips.
        g.tube([base-direction*.18]+centers,[radius*1.03]+rs,(.6,.3,.15),16)
        g.crown(base+Vector((0,.012,-.055)),(radius*1.30,.16,.18),(.6,.3,.15),13,2)
        # Thin integrated dorsal tendons: tapered, no separate floating strings.
        g.tube([(x*.50,.095,.4),(x*.8,.15,.8),(x,.125,z+.03)],
               [.007,.010,.013],(.6,.3,.15),8)
        center=base+direction*length*.925+Vector((0,radius*.73,0))
        nails.append((name,center,radius,length*.058))
    points=[Vector((-.26,-.02,.52)),Vector((-.56,-.025,.86)),Vector((-.82,-.03,1.15)),Vector((-.98,-.04,1.40))]
    # Thumb MCP-to-tip is precisely .86 m; its basal segment remains within palm.
    d=(points[3]-points[1]).normalized();points[2]=points[1]+d*.48;points[3]=points[1]+d*.86
    chains['Thumb']=points;radii['Thumb']=.092
    samples=[];rs=[]
    for j in range(25):
        t=j/24;p=points[0].lerp(points[1],t*3) if t<1/3 else points[1].lerp(points[3],(t-1/3)*1.5)
        r=.12*(1-.4*t)
        if t>.90:r*=math.sqrt(max(.04,1-((t-.90)/.105)**2))
        samples.append(p);rs.append(r)
    g.tube(samples,rs,(.6,.3,.15),16)
    g.crown((-.32,-.055,.65),(.19,.13,.27),(.6,.3,.15),2,2)
    skin=mesh('WizardHand_Skin',g.v,g.f,skinmat,None,True,collection)
    select(skin);remesh=skin.modifiers.new('Continuous organic skin','REMESH');remesh.mode='VOXEL';remesh.voxel_size=.018;remesh.use_smooth_shade=True
    bpy.ops.object.modifier_apply(modifier=remesh.name)
    smooth=skin.modifiers.new('Blend palm and finger roots','SMOOTH');smooth.factor=.55;smooth.iterations=5
    bpy.ops.object.modifier_apply(modifier=smooth.name)
    tris=sum(len(p.vertices)-2 for p in skin.data.polygons)
    dec=skin.modifiers.new('Runtime skin budget','DECIMATE');dec.ratio=min(1,10500/tris)
    bpy.ops.object.modifier_apply(modifier=dec.name)
    color=skin.data.color_attributes.new(name='ArtColor',type='BYTE_COLOR',domain='POINT')
    for v,dst in zip(skin.data.vertices,color.data):
        n=noise.noise_vector(v.co*36+Vector((5,3,7))).x
        freckle=max(0,n-.38)*.35 if v.co.y>0 else 0
        f=1+.035*n-freckle
        dst.color=(.62*f,.32*f,.16*f,1)
    for p in skin.data.polygons:p.use_smooth=True
    # Separate short oval nails with soft domed geometry.
    for name,center,width,length in nails+ [('Thumb',points[3]-(points[3]-points[1]).normalized()*.085+Vector((0,.061,0)),.075,.085)]:
        ng=Geometry();ng.crown(center,(width*.64,.018,length),(.7,.55,.4),7,2)
        nail=mesh('WizardHand_Nail_'+name,ng.v,ng.f,nailmat,None,True,collection)
    # Explicit rig, stable names for direct runtime pose interpolation.
    armdata=bpy.data.armatures.new('WizardHand_Rig');rig=bpy.data.objects.new('WizardHand_Rig',armdata);collection.objects.link(rig)
    select(rig);bpy.ops.object.mode_set(mode='EDIT')
    root=armdata.edit_bones.new('HandRoot');root.head=(0,0,0);root.tail=(0,0,1.0)
    for name,points in chains.items():
        for j in range(3):
            bone=armdata.edit_bones.new(f'{name}_{j}');bone.head=points[j];bone.tail=points[j+1]
            bone.parent=root if j==0 else armdata.edit_bones[f'{name}_{j-1}']
            bone.use_connect=j>0
    bpy.ops.object.mode_set(mode='OBJECT')
    groups={b.name:skin.vertex_groups.new(name=b.name) for b in armdata.bones}
    def distance_segment(p,a,b):
        d=b-a;t=max(0,min(1,(p-a).dot(d)/d.length_squared));return (p-(a+d*t)).length,t
    for v in skin.data.vertices:
        p=v.co;best=None
        for name,points in chains.items():
            # Palm stays on the root, including dorsal tendon geometry.
            if name!='Thumb' and p.z<points[0].z-.06:continue
            if name=='Thumb' and p.x>-.34:continue
            for j in range(3):
                dist,t=distance_segment(p,points[j],points[j+1])
                if best is None or dist<best[0]:best=(dist,name,j,t)
        if best is None or best[0]>.20:
            groups['HandRoot'].add([v.index],1,'REPLACE');continue
        _,name,j,t=best
        weights={f'{name}_{j}':1.0}
        if t<.2:
            previous='HandRoot' if j==0 else f'{name}_{j-1}';blend=.5*(1-t/.2)
            weights={f'{name}_{j}':1-blend,previous:blend}
        elif t>.8 and j<2:
            blend=.5*(t-.8)/.2;weights={f'{name}_{j}':1-blend,f'{name}_{j+1}':blend}
        # Smooth root transition prevents the palm from tearing at the thumb web.
        points=chains[name]
        amount=max(0,min(1,(-p.x-.30)/.25)) if name=='Thumb' else max(0,min(1,(p.z-points[0].z+.14)/.27))
        amount=amount*amount*(3-2*amount)
        weights={key:w*amount for key,w in weights.items()}
        weights['HandRoot']=weights.get('HandRoot',0)+1-amount
        for key,w in weights.items():
            if w>0:groups[key].add([v.index],w,'REPLACE')
    skin.parent=rig;modifier=skin.modifiers.new('Segment skinning','ARMATURE');modifier.object=rig
    for nail in [o for o in collection.objects if o.name.startswith('WizardHand_Nail')]:
        name=nail.name.split('_')[-1];vg=nail.vertex_groups.new(name=name+'_2');vg.add(list(range(len(nail.data.vertices))),1,'REPLACE')
        nail.parent=rig;mod=nail.modifiers.new('Nail follows fingertip','ARMATURE');mod.object=rig
    # Poses are baked into ordinary skeletal clips; no shader-dependent gestures.
    pose_targets={'Hover':{'Index':(.14,.20,.10),'Middle':(.20,.24,.12),'Ring':(.25,.28,.14),'Little':(.30,.31,.15),'Thumb':(.08,.18,.10)},
        'Point':{'Index':(.02,.03,.02),'Middle':(1.15,1.35,.8),'Ring':(1.2,1.4,.8),'Little':(1.2,1.4,.8),'Thumb':(.55,.65,.2)},
        'Grab':{'Index':(.75,1.2,.65),'Middle':(.9,1.2,.65),'Ring':(1.,1.3,.7),'Little':(1.,1.3,.7),'Thumb':(.50,.65,.35)},
        'Carry':{'Index':(.65,1.25,.65),'Middle':(.75,1.05,.55),'Ring':(.80,1.1,.55),'Little':(.9,1.15,.6),'Thumb':(.60,.75,.4)},
        'Release':{'Index':(.06,.12,.08),'Middle':(.1,.16,.08),'Ring':(.15,.18,.10),'Little':(.18,.20,.12),'Thumb':(.03,.08,.04)}}
    scene.render.fps=24
    def pinch(name,target):
        first=1 if name=='Thumb' else 0
        if first:
            rig.pose.bones['Thumb_0'].rotation_euler=(0,0,0)
            bpy.context.view_layer.update()
        rest=chains[name][first:];count=len(rest)-1
        lengths=[(rest[j+1]-rest[j]).length for j in range(count)]
        p=[v.copy() for v in rest];origin=p[0].copy()
        p[1].y=-.3
        if count==3:p[2].y=-.52
        p[count]=Vector(target)
        target=Vector(target)
        for _ in range(32):
            p[count]=target.copy()
            for j in reversed(range(count)):p[j]=p[j+1]+(p[j]-p[j+1]).normalized()*lengths[j]
            p[0]=origin.copy()
            for j in range(count):p[j+1]=p[j]+(p[j+1]-p[j]).normalized()*lengths[j]
        for j in range(count):
            bone=rig.pose.bones[f'{name}_{j+first}']
            direction=(p[j+1]-p[j]).normalized()
            rest_direction=(rest[j+1]-rest[j]).normalized()
            rotation=rest_direction.rotation_difference(direction)@bone.bone.matrix_local.to_quaternion()
            bone.matrix=Matrix.Translation(p[j])@rotation.to_matrix().to_4x4()
            bpy.context.view_layer.update()
    for label,targets in pose_targets.items():
        rig.animation_data_create();rig.animation_data.action=bpy.data.actions.new(label)
        rig.animation_data.action.use_fake_user=True
        for frame in (1,25):
            for name,angles in targets.items():
                for j,angle in enumerate(angles):
                    b=rig.pose.bones[f'{name}_{j}'];b.rotation_mode='XYZ';b.rotation_euler=(angle,0,0)
                    if name=='Thumb' and j==0:b.rotation_euler.z=-.40 if label in ('Carry','Grab') else -.08
                    b.keyframe_insert(data_path='rotation_euler',frame=frame)
            if label in ('Carry','Grab'):
                pinch('Index',(-.26,-.59,1.46))
                pinch('Thumb',(-.38,-.59,1.46))
                for name in ('Index','Thumb'):
                    for j in range(3):rig.pose.bones[f'{name}_{j}'].keyframe_insert(data_path='rotation_euler',frame=frame)
        rig.animation_data.action=None
    for b in rig.pose.bones:b.rotation_euler=(0,0,0)
    scene.frame_start=1;scene.frame_end=25;scene.frame_set(1)
    scene.camera=camera('Hand dorsal',(3.2,6,4.0),(0,0,1.45),4.4)
    filldata=bpy.data.lights.new('Hand soft fill','AREA');filldata.energy=400;filldata.shape='DISK';filldata.size=5
    fill=bpy.data.objects.new('Hand soft fill',filldata);scene.collection.objects.link(fill);fill.location=(1,4,5)
    from island_art_common import focus
    focus(fill,(0,0,1.5))
    camera('Hand palm',(-2.5,-6,3.6),(0,0,1.45),4.4)
    camera('Hand side',(6,0,1.6),(0,0,1.45),4.4)
    rig['blueprint_units']='metres';rig['overall_length_m']=3;rig['palm_width_m']=.99;rig['palm_depth_m']=.33
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'wizard-god-hand.blend'))
    export(list(collection.objects),OUT/'wizard-god-hand.glb',export_animations=True,export_animation_mode='ACTIONS')
    if '--no-review' not in sys.argv:
        render(scene.camera,REVIEW/'hand-dorsal.png',(1000,1000))
        render(bpy.data.objects['Hand palm'],REVIEW/'hand-palm.png',(1000,1000))
        rig.animation_data.action=bpy.data.actions['Carry'];scene.frame_set(1)
        render(scene.camera,REVIEW/'hand-carry.png',(1000,1000))
        rig.animation_data.action=None
        for b in rig.pose.bones:b.rotation_euler=(0,0,0)
        bpy.context.view_layer.update()
        blueprint([skin]+[o for o in collection.objects if o.name.startswith('WizardHand_Nail')],
                  'wizard-hand',REVIEW,[('dorsal',scene.camera),('palm',bpy.data.objects['Hand palm']),('side',bpy.data.objects['Hand side'])])
    receipt={'blender':bpy.app.version_string,'skin_triangles':sum(len(p.vertices)-2 for p in skin.data.polygons),
             'bones':len(armdata.bones),'poses':list(pose_targets),'units':'metres','palm_width_m':.99,'palm_depth_m':.33,
             'overall_length_m':3,'fingers_m':{'Index':1.55,'Middle':1.65,'Ring':1.58,'Little':1.25,'Thumb_MCP_to_tip':.86}}
    (OUT/'hand-specification.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print('WIZARD_HAND',json.dumps(receipt),flush=True)


if __name__=='__main__':build()
