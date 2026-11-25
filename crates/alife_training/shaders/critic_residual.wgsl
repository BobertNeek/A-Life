// Offline experiment only. Loss = mean(0.5 * (V(s)-r-gamma*V(s'))^2).
struct Header { shape:vec4<u32>, adam:vec4<f32>, regularization:vec4<f32>, reserved:vec4<u32> }
@group(0) @binding(0) var<uniform> h:Header;
@group(0) @binding(1) var<storage,read> features:array<f32>;
@group(0) @binding(2) var<storage,read> rows:array<vec4<u32>>;
@group(0) @binding(3) var<storage,read_write> head:array<f32>;
@group(0) @binding(4) var<storage,read_write> result:array<vec4<f32>>;
// Inspect the encoding so overflow rejection does not depend on float comparisons.
fn finite(x:f32)->bool{return (bitcast<u32>(x)&0x7f800000u)!=0x7f800000u;}
var<workgroup> pred_cur:array<f32,64>;
var<workgroup> pred_nxt:array<f32,64>;
var<workgroup> upd_f32:array<f32,64>;
var<workgroup> upd_u32:array<u32,64>;
@compute @workgroup_size(64) fn predict(@builtin(workgroup_id) wid:vec3<u32>,@builtin(local_invocation_index) lane:u32){
    let row=wid.x;if row>=h.shape.y{return;} let n=h.shape.x;let r=rows[row];
    var cur=0.0;var nxt=0.0;
    for(var j=lane;j<n;j+=64u){let w=head[j];cur+=w*features[r.x+j];nxt+=w*features[r.y+j];}
    pred_cur[lane]=cur;pred_nxt[lane]=nxt;workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u){
        if lane<stride{pred_cur[lane]+=pred_cur[lane+stride];pred_nxt[lane]+=pred_nxt[lane+stride];}
        workgroupBarrier();
    }
    if lane==0u{
        let current=head[n]+pred_cur[0];let next=head[n]+pred_nxt[0];
        let reward=bitcast<f32>(r.z);let discount=bitcast<f32>(r.w);
        result[row]=vec4<f32>(current,next,current-reward-discount*next,discount);
    }
}
@compute @workgroup_size(64) fn gradient(@builtin(global_invocation_id) id:vec3<u32>){
    let p=id.x;let n=h.shape.x;if p>n{return;}let width=n+1u;var total=0.0;
    for(var row=0u;row<h.shape.z;row++){
        let r=rows[row];let discount=bitcast<f32>(r.w);var delta=1.0-discount;
        if p<n{delta=features[r.x+p]-discount*features[r.y+p];}
        total+=result[row].z*delta;
    }
    head[3u*width+p]=h.regularization.z*total/f32(h.shape.z);
}
@compute @workgroup_size(64) fn update(@builtin(local_invocation_index) lane:u32){
    let width=h.shape.x+1u;
    if lane==0u{head[4u*width]=0.0;}
    var bad=0u;
    for(var row=lane;row<h.shape.z;row+=64u){
        let v=result[row];if !finite(v.x)||!finite(v.y)||!finite(v.z)||!finite(v.w){bad=1u;}
    }
    upd_u32[lane]=bad;workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u){
        if lane<stride{upd_u32[lane]|=upd_u32[lane+stride];}
        workgroupBarrier();
    }
    if upd_u32[0]!=0u{return;}
    var norm_squared=0.0;var grad_bad=0u;
    for(var p=lane;p<width;p+=64u){
        let g=head[3u*width+p];if !finite(g){grad_bad=1u;}norm_squared+=g*g;
    }
    upd_f32[lane]=norm_squared;upd_u32[lane]=grad_bad;workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u){
        if lane<stride{upd_f32[lane]+=upd_f32[lane+stride];upd_u32[lane]|=upd_u32[lane+stride];}
        workgroupBarrier();
    }
    let total_norm=upd_f32[0];
    if upd_u32[0]!=0u||!finite(total_norm){return;}
    let scale=min(1.0,h.regularization.y/max(sqrt(total_norm),1e-20));
    let beta1=h.adam.y;let beta2=h.adam.z;
    let correct1=1.0-pow(beta1,f32(h.shape.w));let correct2=1.0-pow(beta2,f32(h.shape.w));
    // Inspect every prospective coordinate before changing any weight or moment.
    var check_bad=0u;
    for(var p=lane;p<width;p+=64u){
        let g=head[3u*width+p]*scale;
        let m=beta1*head[width+p]+(1.0-beta1)*g;
        let v=beta2*head[2u*width+p]+(1.0-beta2)*g*g;
        let change=m/correct1/(sqrt(v/correct2)+h.adam.w)+h.regularization.x*head[p];
        let next=head[p]-h.adam.x*change;
        if !finite(next)||!finite(m)||!finite(v){check_bad=1u;}
    }
    upd_u32[lane]=check_bad;workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u){
        if lane<stride{upd_u32[lane]|=upd_u32[lane+stride];}
        workgroupBarrier();
    }
    if upd_u32[0]!=0u{return;}
    for(var p=lane;p<width;p+=64u){
        let g=head[3u*width+p]*scale;
        let m=beta1*head[width+p]+(1.0-beta1)*g;
        let v=beta2*head[2u*width+p]+(1.0-beta2)*g*g;
        let change=m/correct1/(sqrt(v/correct2)+h.adam.w)+h.regularization.x*head[p];
        head[p]-=h.adam.x*change;head[width+p]=m;head[2u*width+p]=v;
    }
    workgroupBarrier();
    if lane==0u{head[4u*width]=1.0;}
}
