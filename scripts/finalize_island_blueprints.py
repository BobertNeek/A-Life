"""Add measured viewport guides and pack approved references into editable sources."""
import json
import sys
from pathlib import Path
import bpy

ROOT=Path(__file__).resolve().parents[1]
ASSETS=ROOT/'crates/alife_game_app/assets'


def line(collection,name,start,end):
    curve=bpy.data.curves.new(name,'CURVE');curve.dimensions='3D'
    spline=curve.splines.new('POLY');spline.points.add(1)
    for p,co in zip(spline.points,(start,end)):p.co=(*co,1)
    ob=bpy.data.objects.new(name,curve);collection.objects.link(ob)
    ob.hide_render=True


def label(collection,text,position,size):
    data=bpy.data.curves.new(text,'FONT');data.body=text;data.size=size
    ob=bpy.data.objects.new(text,data);collection.objects.link(ob)
    ob.location=position;ob.rotation_euler=(1.57079632679,0,0);ob.hide_render=True


def dimensions(collection,name,origin,width,depth,height):
    x,y,z=origin
    line(collection,name+' width',(x-width/2,y-depth*.6,z),(x+width/2,y-depth*.6,z))
    line(collection,name+' depth',(x+width*.6,y-depth/2,z),(x+width*.6,y+depth/2,z))
    line(collection,name+' height',(x-width*.6,y,z),(x-width*.6,y,z+height))
    label(collection,f'{name}: {width:g} W x {depth:g} D x {height:g} H metres',
          (x-width/2,y-depth*.6,z-.3),max(.10,min(.26,height*.025)))


for relative in ['landscape/island/island-assets.blend','landscape/island/island-terrain.blend','hand/wizard-god-hand.blend']:
    path=ASSETS/relative;bpy.ops.wm.open_mainfile(filepath=str(path))
    if relative.startswith('hand/'):
        assert {'Hover','Point','Grab','Carry','Release'} <= set(bpy.data.actions.keys()), 'Hand source lost its poses'
        for action in bpy.data.actions: action.use_fake_user=True
    previous=bpy.data.collections.get('Measured_Blueprint_Guides')
    if previous:
        for ob in list(previous.objects):bpy.data.objects.remove(ob,do_unlink=True)
        bpy.data.collections.remove(previous)
    guides=bpy.data.collections.new('Measured_Blueprint_Guides');bpy.context.scene.collection.children.link(guides)
    if 'island-assets' in relative:
        for ob in bpy.data.collections['Island_Asset_Blueprints'].objects:
            if ob.name.endswith('_L0'):
                dimensions(guides,ob.name[:-3],ob.location,*ob['target_width_depth_height_m'])
    elif 'island-terrain' in relative:
        dimensions(guides,'Island',(0,0,0),1000,1200,240)
        ocean=bpy.data.objects.get('Island ocean')
        for v in ocean.data.vertices:
            v.co.x=5000 if v.co.x>0 else -5000
            v.co.y=5000 if v.co.y>0 else -5000
    else:
        dimensions(guides,'Wizard hand',(0,0,0),.99,.33,3)
        label(guides,'Palm 1.35 m; middle finger 1.65 m; wrist 0.48 m',(-.6,-.3,3.2),.10)
    for image in bpy.data.images:
        if image.source=='FILE' and image.filepath and not image.packed_file:image.pack()
    bpy.ops.wm.save_as_mainfile(filepath=str(path))
    print('MEASURED_BLUEPRINT',relative,flush=True)
