// Offline experiment only. Loss = mean(0.5 * (V(s)-r-gamma*V(s'))^2).
struct Header { shape:vec4<u32>, adam:vec4<f32>, regularization:vec4<f32>, reserved:vec4<u32> }
@group(0) @binding(0) var<uniform> h:Header;
@group(0) @binding(1) var<storage,read> features:array<f32>;
@group(0) @binding(2) var<storage,read> rows:array<vec4<u32>>;
@group(0) @binding(3) var<storage,read_write> head:array<f32>;
@group(0) @binding(4) var<storage,read_write> result:array<vec4<f32>>;
fn finite(x:f32)->bool{return x==x && abs(x)<=3.402823466e+38;}
@compute @workgroup_size(64) fn predict(@builtin(global_invocation_id) id:vec3<u32>){
    let row=id.x;if row>=h.shape.y{return;} let n=h.shape.x;let r=rows[row];
    var current=head[n];var next=head[n];
    for(var j=0u;j<n;j++){current+=head[j]*features[r.x+j];next+=head[j]*features[r.y+j];}
    let reward=bitcast<f32>(r.z);let discount=bitcast<f32>(r.w);
    result[row]=vec4<f32>(current,next,current-reward-discount*next,discount);
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
@compute @workgroup_size(1) fn update(){
    let width=h.shape.x+1u;head[4u*width]=0.0;var norm_squared=0.0;
    for(var row=0u;row<h.shape.z;row++){
        let v=result[row];if !finite(v.x)||!finite(v.y)||!finite(v.z)||!finite(v.w){return;}
    }
    for(var p=0u;p<width;p++){let g=head[3u*width+p];if !finite(g){return;}norm_squared+=g*g;}
    if !finite(norm_squared){return;}
    let scale=min(1.0,h.regularization.y/max(sqrt(norm_squared),1e-20));
    let beta1=h.adam.y;let beta2=h.adam.z;
    let correct1=1.0-pow(beta1,f32(h.shape.w));let correct2=1.0-pow(beta2,f32(h.shape.w));
    // Inspect every prospective coordinate before changing any weight or moment.
    for(var p=0u;p<width;p++){
        let g=head[3u*width+p]*scale;
        let m=beta1*head[width+p]+(1.0-beta1)*g;
        let v=beta2*head[2u*width+p]+(1.0-beta2)*g*g;
        let change=m/correct1/(sqrt(v/correct2)+h.adam.w)+h.regularization.x*head[p];
        let next=head[p]-h.adam.x*change;
        if !finite(next)||!finite(m)||!finite(v){return;}
    }
    for(var p=0u;p<width;p++){
        let g=head[3u*width+p]*scale;
        let m=beta1*head[width+p]+(1.0-beta1)*g;
        let v=beta2*head[2u*width+p]+(1.0-beta2)*g*g;
        let change=m/correct1/(sqrt(v/correct2)+h.adam.w)+h.regularization.x*head[p];
        head[p]-=h.adam.x*change;head[width+p]=m;head[2u*width+p]=v;
    }
    head[4u*width]=1.0;
}
