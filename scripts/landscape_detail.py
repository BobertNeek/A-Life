"""Reusable solid-mesh landscape details for Blender 5.2 LTS."""
import math
import random
import bpy
from mathutils import Vector, noise


def material(name, color, vertex=False):
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    mat.diffuse_color = (*color, 1)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get('Principled BSDF')
    bsdf.inputs['Base Color'].default_value = (*color, 1)
    bsdf.inputs['Roughness'].default_value = .86
    if vertex:
        vc = mat.node_tree.nodes.new('ShaderNodeVertexColor')
        vc.layer_name = 'LandscapeColor'
        mat.node_tree.links.new(vc.outputs['Color'], bsdf.inputs['Base Color'])
    return mat


def mesh(name, verts, faces, mat, colors=None, smooth=True):
    data = bpy.data.meshes.new(name)
    data.from_pydata(verts, [], faces)
    data.update()
    ob = bpy.data.objects.new(name, data)
    bpy.context.scene.collection.objects.link(ob)
    data.materials.append(mat)
    for poly in data.polygons:
        poly.use_smooth = smooth
    if colors:
        attr = data.color_attributes.new(name='LandscapeColor', type='BYTE_COLOR', domain='POINT')
        for row, color in zip(attr.data, colors):
            row.color = (*color[:3], 1)
    return ob


def stone(name, seed, cliff=False):
    """Fractured, stratified solids with uneven ledges and moss on upper faces."""
    rng = random.Random(seed)
    verts, faces, colors = [], [], []
    blocks = 6 if cliff else 3
    for block in range(blocks):
        if cliff:
            cx = (block % 3 - 1) * 2.55 + rng.uniform(-.4,.4)
            cy = (block // 3 - .5) * 2.0 + rng.uniform(-.4,.4)
            height = rng.uniform(2.0, 5.0)
            sx, sy = rng.uniform(1.7, 2.2), rng.uniform(1.5, 1.9)
        else:
            cx, cy = [(0, 0), (.62, .08), (-.48, -.22)][block]
            height = [1.3, .65, .48][block] * rng.uniform(.8, 1.15)
            sx, sy = [.8, .46, .38][block], [.64, .42, .32][block]
        base = len(verts)
        sides, rings = 10, 9
        angular = [rng.uniform(.82, 1.16) for _ in range(sides)]
        for j in range(rings):
            t = j / (rings - 1)
            ledge = [1.02, 1.01, .97, 1.02, .93, .97, .91, .93, .82][j]
            for i in range(sides):
                a = i * math.tau / sides + .16
                n = noise.noise_vector(Vector((i*.6, j*.7, seed+block)))
                x = cx + math.cos(a)*sx*angular[i]*ledge + .12*t + n.x*.045
                y = cy + math.sin(a)*sy*angular[i]*ledge + n.y*.035
                z = max(0, height*t + (n.z*.045 if j else 0))
                if j == rings-1:
                    z += height*.045*math.sin(a*2+seed)
                verts.append((x, y, z))
                shade = .8 + .16*n.x + .10*(j % 2)
                color = (.29*shade, .31*shade, .29*shade)
                if j in (2, 4, 6):
                    color = tuple(v*.9 for v in color)
                if j >= 7 and math.sin(a*3+seed) > -.2:
                    color = (.16*shade, .24*shade, .055*shade)
                colors.append(color)
        for j in range(rings-1):
            for i in range(sides):
                a=base+j*sides+i; b=base+j*sides+(i+1)%sides
                faces.append((a, b, b+sides, a+sides))
        faces.append(tuple(base+i for i in reversed(range(sides))))
        faces.append(tuple(base+(rings-1)*sides+i for i in range(sides)))
    return mesh(name, verts, faces, material('Highlands | painted stone', (.3,.32,.29), True), colors, False)


def foliage(name, seed, kind='grass'):
    """Curved, folded leaves and petals; shared material, no alpha sorting."""
    rng=random.Random(seed)
    vs, fs, cs = [], [], []

    def leaf(base, tip, width, color):
        base, tip = Vector(base), Vector(tip)
        delta=tip-base
        across=delta.cross(Vector((0,0,1))).normalized()
        if across.length < .5: across=Vector((1,0,0))
        start=len(vs)
        for j in range(5):
            t=j/4
            center=base+delta*t+Vector((0,0,math.sin(math.pi*t)*delta.length*.17))
            half=width*math.sin(math.pi*t)**.8+.001
            for k in (-1,0,1):
                p=center+across*half*k+Vector((0,0,(1-abs(k))*half*.35))
                vs.append(tuple(p)); cs.append(tuple(c*(.67+.33*t)*(1-.12*abs(k)) for c in color))
        for j in range(4):
            for k in range(2):
                a=start+j*3+k; fs.append((a,a+1,a+4,a+3))

    if kind=='fern':
        for frond in range(9):
            a=frond*2.399; length=rng.uniform(.7,1.2)
            direction=Vector((math.cos(a),math.sin(a),0))
            for j in range(1,9):
                t=j/9
                p=direction*(t*length)+Vector((0,0,.18+.52*math.sin(t*2)))
                for side in (-1,1):
                    across=Vector((-direction.y,direction.x,0))*side
                    tip=p+across*(.24*math.sin(t*math.pi))+direction*.10+Vector((0,0,.03))
                    leaf(p,tip,.041,(.15,.32,.035))
    else:
        for i in range(32 if kind=='grass' else 16):
            a=rng.random()*math.tau
            radius=rng.random()*.30
            base=Vector((math.cos(a)*radius, math.sin(a)*radius, 0))
            tip=base+Vector((math.cos(a)*.23,math.sin(a)*.23,rng.uniform(.22,.64)))
            leaf(base,tip,rng.uniform(.018,.037),(.22,.37,.048))
        if kind=='flowers':
            for i in range(7):
                a=i*2.399
                center=Vector((math.cos(a)*.27,math.sin(a)*.27,.32+.18*rng.random()))
                leaf((center.x,center.y,0),center,.012,(.10,.25,.025))
                for j in range(7):
                    b=j*math.tau/7
                    tip=center+Vector((math.cos(b)*.10,math.sin(b)*.10,.01))
                    leaf(center,tip,.042, (.92,.77,.16) if i%3==0 else (.9,.91,.75))
    mat=material('Highlands | vegetation', (.2,.35,.05), True)
    mat.use_backface_culling=False
    return mesh(name,vs,fs,mat,cs)


def refine_props():
    names=['Boulder_variant_1','Boulder_variant_2','Boulder_variant_3','Grass_clump',
           'Cliff_outcrop','Fern_patch','Wildflower_patch']
    for name in names:
        old=bpy.data.objects.get(name)
        if old: bpy.data.objects.remove(old,do_unlink=True)
    for i in range(3): stone('Boulder_variant_'+str(i+1),43+i)
    stone('Cliff_outcrop',77,True)
    foliage('Grass_clump',51)
    foliage('Fern_patch',52,'fern')
    foliage('Wildflower_patch',53,'flowers')
    return names


def ground_detail(mat):
    """Pack seamless procedural grain and a tangent normal map for standard glTF."""
    size=256
    heights=[]
    rng=random.Random(917)
    octaves=[(n,amp,[rng.uniform(-1,1) for _ in range(n*n)])
             for n,amp in [(8,.5),(16,.28),(32,.16),(64,.08)]]
    for y in range(size):
        for x in range(size):
            h=0
            for n,amp,grid in octaves:
                u=x/size*n;v=y/size*n;i=int(u);j=int(v)
                fx=u-i;fy=v-j;fx=fx*fx*(3-2*fx);fy=fy*fy*(3-2*fy)
                a=grid[j*n+i]*(1-fx)+grid[j*n+(i+1)%n]*fx
                b=grid[((j+1)%n)*n+i]*(1-fx)+grid[((j+1)%n)*n+(i+1)%n]*fx
                h+=(a*(1-fy)+b*fy)*amp
            heights.append(h)
    albedo=[];normal=[]
    for y in range(size):
        for x in range(size):
            h=heights[y*size+x]
            value=.80+h*.20
            albedo.extend((value,value,value,1))
            dx=heights[y*size+(x+1)%size]-heights[y*size+(x-1)%size]
            dy=heights[((y+1)%size)*size+x]-heights[((y-1)%size)*size+x]
            n=Vector((-dx*.8,-dy*.8,1)).normalized()
            normal.extend((n.x*.5+.5,n.y*.5+.5,n.z*.5+.5,1))
    nodes=mat.node_tree.nodes;links=mat.node_tree.links
    for node in list(nodes):
        if node.type not in {'BSDF_PRINCIPLED','OUTPUT_MATERIAL','VERTEX_COLOR'}:
            nodes.remove(node)
    bsdf=nodes.get('Principled BSDF')
    vc=next(n for n in nodes if n.type=='VERTEX_COLOR')
    tex=nodes.new('ShaderNodeTexImage');normtex=nodes.new('ShaderNodeTexImage')
    for name,pixels,node in [('Highlands_Grain',albedo,tex),('Highlands_Normal',normal,normtex)]:
        im=bpy.data.images.new(name,width=size,height=size,alpha=False)
        im.colorspace_settings.name='Non-Color'
        im.pixels.foreach_set(pixels);im.pack();node.image=im
    mix=nodes.new('ShaderNodeMixRGB');mix.blend_type='MULTIPLY';mix.inputs[0].default_value=1
    links.new(vc.outputs['Color'],mix.inputs[1]);links.new(tex.outputs['Color'],mix.inputs[2])
    links.new(mix.outputs['Color'],bsdf.inputs['Base Color'])
    normalmap=nodes.new('ShaderNodeNormalMap');normalmap.inputs['Strength'].default_value=.65
    links.new(normtex.outputs['Color'],normalmap.inputs['Color']);links.new(normalmap.outputs['Normal'],bsdf.inputs['Normal'])
