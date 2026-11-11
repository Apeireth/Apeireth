#version 300 es
precision highp float;
uniform vec2  uRes;
uniform vec2  uCenter;
uniform float uScale;
uniform float uTime;
uniform float uRot;
uniform float uBright;
uniform float uTurb;
uniform float uPulse;
uniform float uIncl;
uniform float uAzim;

uniform float uDiskAzim;
uniform float uDiskIncl;
uniform float uAmbStr;
uniform float uEclipse;
uniform float uTlBright;
uniform float uTlGold;
out vec4 fragColor;

float hash21(vec2 p){
    p = fract(p * vec2(234.34, 435.345));
    p += dot(p, p + 34.23);
    return fract(p.x * p.y);
}
float vnoise(vec2 p){
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    float a = hash21(i);
    float b = hash21(i + vec2(1.0, 0.0));
    float c = hash21(i + vec2(0.0, 1.0));
    float d = hash21(i + vec2(1.0, 1.0));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}
float fbm(vec2 p){
    float v = 0.0;
    float a = 0.5;
    for(int i = 0; i < 5; i++){
        v += a * vnoise(p);
        p = p * 2.03 + vec2(17.3, 9.1);
        a *= 0.5;
    }
    return v;
}

vec3 diskRadiance(float r, float phi, float t){
    if(r > 5.6) return vec3(0.0);
    float rIn  = 1.45;
    float rOut = 5.0;
    float band = smoothstep(rIn, rIn + 0.40, r) * (1.0 - smoothstep(rIn + 1.3, rOut, r));
    
    
    
    
    
    float pr = uRot * (2.6 / (r * r * 0.5 + 0.5)) + t * (0.70 * pow(1.45 / r, 1.5));
    float pa = phi * 2.0 + pr;
    
    
    
    
    
    
    
    float n  = fbm(vec2(pa * 2.6, r * 0.9 - t * 0.008));
    float n2 = fbm(vec2(pa * 8.4 + 7.7, r * 1.8 - t * 0.015));
    
    
    
    
    float cl = fbm(vec2(pa * 1.15 + 3.3, r * 0.55 - t * 0.006));
    float clump = smoothstep(0.50, 0.60, cl);
    
    
    float streak = 0.50 + (0.85 * n + 0.60 * n2) * uTurb + (0.90 * n - 0.27) * (1.0 - uTurb);
    streak *= (0.40 + 0.60 * uTurb) + (1.80 - 0.60 * uTurb) * clump;  
    float hr = clamp((r - rIn) / (rOut - rIn), 0.0, 1.0);
    vec3 whiteHot = vec3(1.000, 0.950, 0.820);
    vec3 gold     = vec3(1.000, 0.824, 0.478);
    vec3 amber    = vec3(0.910, 0.639, 0.239);
    vec3 c = mix(gold, amber, smoothstep(0.30, 1.0, hr));
    c = mix(whiteHot, c, smoothstep(0.02, 0.22, hr));
    
    
    float dop = 1.0 - 0.72 * cos(phi);
    c = mix(c, vec3(1.0, 0.955, 0.830), clamp((dop - 1.0) * 0.45, 0.0, 1.0));
    
    float lum = dot(c, vec3(0.3333));
    c = mix(c, lum * vec3(0.92, 0.99, 1.14), (1.0 - uTlGold) * 0.45);
    float rim = exp(-pow((r - 1.52) * 5.0, 2.0));
    
    
    
    vec3 radiance = c * band * streak * dop * uBright * uTlBright * 1.9;
    radiance += vec3(1.0, 0.96, 0.87) * rim * dop * (0.6 + 0.7 * streak) * uBright * uTlBright * 1.1;
    return radiance;
}


vec3 starLayer(vec2 suv, vec2 voff, float k, float scale, float thr, float sizeMul, float boost){
    vec2 g = (suv + voff * k) * scale;
    vec2 id = floor(g);
    float h = hash21(id);
    if(h <= thr) return vec3(0.0);
    vec2 jit = vec2(hash21(id + 7.13), hash21(id + 3.71)) - 0.5;
    float sd = length(fract(g) - 0.5 - jit * 0.7);
    float mag = (h - thr) / (1.0 - thr);
    float rad = 0.14 * sizeMul * (0.6 + 0.8 * mag);
    float s = 1.0 - smoothstep(0.0, rad, sd);
    float hc = hash21(id + 17.7);
    vec3 tint = vec3(0.95, 0.85, 0.62);
    if(hc < 0.12) tint = vec3(0.72, 0.80, 1.00);
    else if(hc < 0.55) tint = vec3(0.98, 0.93, 0.80);
    return tint * s * (0.08 + 0.55 * mag) * boost;
}

void main(){
    vec2 frag = gl_FragCoord.xy;
    vec2 p = (frag - uCenter) / uScale;
    
    
    
    float ca = cos(uDiskAzim);
    float sa = sin(uDiskAzim);
    p = mat2(ca, -sa, sa, ca) * p;
    float incl = uDiskIncl;
    float rp = length(p);
    float t = uTime;
    float hole = smoothstep(0.985, 1.045, rp);
    float pulseWave = 0.5 + 0.5 * sin(t * 1.9);

    vec3 sky = vec3(0.027, 0.027, 0.047);

    
    
    {
        vec2 suv = frag / uRes.y;
        vec2 voff = vec2(uAzim, (uIncl - 0.30) * 0.8);
        vec2 bsuv = suv + voff * 0.45;
        float bandAxis = dot(bsuv, vec2(-0.485, 0.874));
        float bd = (bandAxis - 0.60) * 2.2;
        float mw = exp(-bd * bd);
        float mwTex = fbm(vec2(bsuv.x * 2.2 + bsuv.y * 1.1, bandAxis * 7.0));
        float neb = fbm(bsuv * 3.5 + 4.7);
        sky += vec3(0.90, 0.80, 0.58) * mw * (0.20 + 0.55 * mwTex + 0.35 * neb) * 0.16;
        float starBoost = (1.0 + 2.2 * mw) * (1.0 - 0.55 * uAmbStr);
        sky += starLayer(suv, voff, 0.50, 230.0, 0.9960, 0.80, starBoost);
        sky += starLayer(suv, voff, 0.90, 150.0, 0.9955, 1.15, starBoost);
        sky += starLayer(suv, voff, 1.40,  95.0, 0.9965, 1.70, starBoost);
    }

    
    sky += vec3(0.42, 0.58, 0.92) * uAmbStr * (0.10 + 0.16 * (frag.y / uRes.y));
    sky *= 1.0 + 0.18 * uAmbStr;
    sky *= 1.0 - 0.80 * uEclipse;

    vec3 col = sky;

    
    float outer = max(rp - 1.0, 0.0);
    col += vec3(1.00, 0.78, 0.38) * 0.060 * exp(-outer * 0.85) * (0.3 + uBright) * (1.0 - 0.60 * uEclipse);
    col += vec3(0.92, 0.62, 0.24) * 0.030 * exp(-outer * 0.26) * (0.3 + uBright) * (1.0 - 0.60 * uEclipse);

    
    {
        float back = smoothstep(-0.30, 0.55, p.y);
        float lift = 1.15 * exp(-outer * 0.85);
        vec2 pa2 = vec2(p.x, (p.y - lift * back) / incl);
        vec3 arcTop = diskRadiance(length(pa2), atan(pa2.y, pa2.x), t) * back;
        vec2 pb = vec2(p.x, (p.y + lift * back * 0.55) / incl);
        vec3 arcBot = diskRadiance(length(pb), atan(pb.y, pb.x), t) * back * 0.38;
        col += (arcTop + arcBot) * hole;
    }

    
    {
        float front = 1.0 - smoothstep(-0.18, 0.30, p.y);
        vec2 pf = vec2(p.x, p.y / incl);
        col += diskRadiance(length(pf), atan(pf.y, pf.x), t) * front * hole;
    }

    
    col = 1.0 - exp(-col * 1.55 * (1.0 + 0.15 * uAmbStr));
    col *= hole;

    
    float ringI = exp(-pow((rp - 1.045) * 42.0, 2.0));
    col += vec3(1.0, 0.96, 0.86) * ringI * (0.60 + 0.30 * uBright + 0.50 * uPulse * pulseWave) * (1.0 + 0.8 * uEclipse);
    float ring2 = exp(-pow((rp - 1.14) * 13.0, 2.0));
    col += vec3(1.0, 0.82, 0.50) * ring2 * 0.16 * uBright;

    
    col += vec3(1.0, 0.95, 0.85) * exp(-rp * 2.4) * uPulse * (0.22 + 0.38 * pulseWave) * 0.55;

    fragColor = vec4(col, 1.0);
}