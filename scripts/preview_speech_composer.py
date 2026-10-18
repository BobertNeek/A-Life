#!/usr/bin/env python3
"""Replay CPU Bevy node geometry as a self-contained browser visual preview.

Generate fixtures with ALIFE_SPEECH_COMPOSER_CPU_FIXTURE and the composer Rust
layout test. This is a browser replay with browser text layout, not a native
Bevy screenshot or evidence of production GPU/input acceptance.
"""
import argparse
import base64
import hashlib
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fixture", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--font", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    receipt = json.loads(args.fixture.read_text())
    assets = {}
    for fixture in receipt["fixtures"]:
        for node in fixture["nodes"]:
            if node.get("image"):
                path = node["image"]["source_path"]
                assets[path] = base64.b64encode((root / path).read_bytes()).decode()
    data = json.dumps({"receipt": receipt, "assets": assets}).replace("<", "\\u003c")
    font = base64.b64encode(args.font.read_bytes()).decode()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("""<!doctype html><meta charset="utf-8"><title>A-Life speech composer preview</title>
<style>@font-face{font-family:Bevy;src:url(data:font/ttf;base64,FONT)}
*{box-sizing:border-box}body{margin:0;background:#0c151b;color:#faf5d6;font:14px system-ui}header{padding:16px;max-width:1000px}h1{font-size:20px;margin:0 0 8px}p{margin:6px 0;line-height:1.5}select{padding:8px;margin:8px 0;background:#1b3038;color:#faf5d6}#stage{position:relative;background:#263c3d;overflow:hidden}#stage div{position:absolute}#stage img{width:100%;height:100%}.text{font-family:Bevy,monospace;white-space:pre-wrap;overflow-wrap:anywhere;line-height:1.2;margin:0}</style>
<header><h1>Speech composer · source-derived preview</h1><p>Actual CPU Bevy container geometry and shipping icons. Browser text layout and compositing. No native Bevy render, GPU simulation or production input acceptance is claimed.</p><label>Fixture <select id="fixture"></select></label></header><main id="stage"></main>
<script>const data=DATA,fixtures=data.receipt.fixtures,stage=document.querySelector('#stage'),select=document.querySelector('select');
function color(c){return c?`rgba(${c.slice(0,3).map(x=>Math.round(x*255)).join(',')},${c[3]})`:'transparent'}
function px(v){return v?.unit==='px'?v.value:0}
fixtures.forEach((f,i)=>{const o=document.createElement('option');o.value=i;o.textContent=`${f.viewport.join('×')} · ${f.case}${f.error?' · rejected/error gutter':''}`;select.append(o)});
function draw(index){window.previewReady=false;const f=fixtures[index];stage.replaceChildren();stage.style.width=f.viewport[0]+'px';stage.style.height=f.viewport[1]+'px';const elements=new Map,byId=new Map(f.nodes.map(n=>[n.entity,n]));
for(const n of f.nodes){if(!n.visible_in_hierarchy||n.style.display==='None')continue;let p=n.parent,hidden=false;while(p&&byId.has(p)){const a=byId.get(p);if(a.style.display==='None'||!a.visible_in_hierarchy){hidden=true;break}p=a.parent}if(hidden)continue;
const e=document.createElement('div'),s=n.style,b=n.computed_box;const parent=byId.get(n.parent),origin=parent?.computed_box.min||[0,0];e.dataset.role=n.role;e.dataset.name=n.name||'';e.style.left=(b.min[0]-origin[0]-px(parent?.style.border.left))+'px';e.style.top=(b.min[1]-origin[1]-px(parent?.style.border.top))+'px';e.style.width=b.size[0]+'px';e.style.height=b.size[1]+'px';e.style.background=color(s.background);e.style.borderRadius=px(s.border_radius.top_left)+'px';if(s.border_colors){e.style.borderStyle='solid';for(const side of ['top','right','bottom','left']){e.style['border'+side[0].toUpperCase()+side.slice(1)+'Width']=px(s.border[side])+'px';e.style['border'+side[0].toUpperCase()+side.slice(1)+'Color']=color(s.border_colors[side])}}
if(n.image){const img=document.createElement('img');img.src='data:image/png;base64,'+data.assets[n.image.source_path];img.alt='';e.append(img)}
if(n.text){e.className='text';e.textContent=n.text.value;e.style.fontSize=n.text.font_size+'px';e.style.height='auto'}
if(n.role==='speech_draft_viewport'){e.style.overflow='hidden';e.style.border='none';e.style.padding='4px'}elements.set(n.entity,e);(elements.get(n.parent)||stage).append(e)}
// CPU fixture does not shape glyphs; use browser layout and explicitly follow
// the end caret inside the same clipped draft viewport for visual inspection.
document.fonts.ready.then(()=>requestAnimationFrame(()=>{for(const e of stage.querySelectorAll('[data-role="speech_draft_viewport"]')){if(f.case!=='keyboard_help')e.scrollTop=e.scrollHeight}window.previewReady=true}))}select.onchange=()=>draw(+select.value);draw(0);</script>""".replace("FONT", font).replace("DATA", data), encoding="utf-8")
    source = root / "crates/alife_game_app/src/production_conversation_lineage_ui/composer.rs"
    print(json.dumps({"preview": str(args.output), "fixtures": len(receipt["fixtures"]),
        "fixture_sha256": hashlib.sha256(args.fixture.read_bytes()).hexdigest(),
        "composer_source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
        "evidence": "browser replay of actual CPU Bevy node geometry; browser glyphs; no native acceptance"}))


if __name__ == "__main__":
    main()
