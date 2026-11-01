#version 300 es
precision highp float;
uniform vec2  uRes;
uniform float uAspect;
uniform float uTime;
uniform float uStars;
uniform float uStarStretch;
uniform float uMilky;
uniform float uMoon;
uniform vec2  uMoonPos;
uniform float uMoonR;
uniform float uFurnace;
uniform float uFurnaceR;
uniform float uEngine;
uniform float uShock;
uniform vec2  uShockApex;
uniform float uBody;
uniform vec2  uBodyPos;
uniform float uBodyLen;
uniform float uBodyMolten;
uniform float uBodyMorph;
uniform float uPlanetHint;
uniform float uGroundGlow;
uniform vec2  uGroundPos;
uniform float uCoreGlow;
uniform vec2  uCorePos;
uniform float uFlash;
uniform vec2  uFlashPos;
uniform float uVig;
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

void main(){

    vec2 suv = (gl_FragCoord.xy / uRes.y) * 2.0 - vec2(uAspect, 1.0);
    vec3 col = vec3(0.027,0.027,0.047);


    if (uMilky > 0.001) {
        vec2 nrm = vec2(-0.485, 0.874);
        float bd = (dot(suv, nrm) - 0.55) * 2.0;
        float mw = exp(-bd * bd);
        float tex = fbm(suv * 2.6 + 4.7);
        float side = smoothstep(-0.4, 0.9, suv.x / uAspect);
        col += vec3(0.9,0.8,0.58) * mw * (0.25 + 0.6 * tex) * 0.13 * side * uMilky;
    }


    if (uStars > 0.001) {
        vec2 g = (suv + vec2(uAspect, 1.0)) * 95.0;
        vec2 id = floor(g);
        float h = hash21(id);
        if (h > 0.9955) {
            vec2 jit = vec2(hash21(id + 7.13), hash21(id + 3.71)) - 0.5;
            vec2 off = fract(g) - 0.5 - jit * 0.7;
            off.x /= 1.0 + uStarStretch * 7.0;
            float sd = length(off);
            float mag = (h - 0.9955) / 0.0045;
            float s = 1.0 - smoothstep(0.0, 0.16 * (0.6 + 0.8 * mag), sd);
            float hc = hash21(id + 17.7);
            vec3 tint = vec3(0.95,0.85,0.62);
            if (hc < 0.12) tint = vec3(0.72,0.8,1.0);
            else if (hc < 0.55) tint = vec3(0.98,0.93,0.8);
            col += tint * s * (0.08 + 0.5 * mag) * uStars;
        }
    }


    if (uMoon > 0.001) {
        vec2 mp = suv - uMoonPos;
        float mr = length(mp);
        vec2 L = normalize(vec2(0.55, -0.35));
        float nl = dot(mp / max(mr, 1e-4), L);
        float disk = 1.0 - smoothstep(uMoonR - 0.004, uMoonR + 0.002, mr);
        float crat = fbm(mp * 9.0 / uMoonR + 3.1) * 0.6 + fbm(mp * 23.0 / uMoonR) * 0.4;
        float lit = smoothstep(-0.35, 0.85, nl);
        vec3 surf = mix(vec3(0.010, 0.010, 0.014), vec3(0.40, 0.42, 0.46) * (0.40 + 0.60 * crat), lit);
        col = mix(col, surf, disk * 0.96 * uMoon);
        float rim = exp(-pow((mr - uMoonR) * 60.0, 2.0)) * smoothstep(-0.15, 0.7, nl);
        col += vec3(0.85, 0.88, 0.92) * rim * 0.75 * uMoon;
        col += vec3(0.50, 0.55, 0.65) * exp(-max(mr - uMoonR, 0.0) * 9.0) * 0.045 * uMoon;
    }


    if (uPlanetHint > 0.001) {
        float horizon = -0.24;
        float towardHole = smoothstep(-0.3 * uAspect, 0.9 * uAspect, suv.x);
        float below = 1.0 - smoothstep(horizon - 0.02, horizon + 0.015, suv.y);
        float ringTex = fbm(vec2(suv.x * 3.0, suv.y * 40.0));
        vec3 sand = vec3(0.62, 0.56, 0.44) * (0.30 + 0.70 * towardHole) * (0.70 + 0.55 * ringTex);
        col = mix(col, sand, below * uPlanetHint * 0.85);
        col += vec3(0.95, 0.85, 0.60) * exp(-pow((suv.y - horizon) * 30.0, 2.0)) * (0.10 + 0.28 * towardHole) * uPlanetHint;
        vec2 pp = suv - vec2(-0.60 * uAspect, 0.75);
        float pr = length(pp);
        float limb = (1.0 - smoothstep(1.16, 1.22, pr)) * smoothstep(horizon - 0.04, horizon + 0.10, suv.y);
        float limbShade = smoothstep(1.16, 0.55, pr);
        vec3 limbCol = mix(vec3(0.26, 0.25, 0.23), vec3(0.76, 0.73, 0.64), limbShade);
        limbCol *= 0.85 + 0.3 * fbm(pp * vec2(2.0, 6.0));
        col = mix(col, limbCol, limb * uPlanetHint * 0.9);
    }


    if (uGroundGlow > 0.001) {
        vec2 gq = (suv - uGroundPos) * vec2(1.0, 2.4);
        col += vec3(1.0, 0.50, 0.15) * uGroundGlow * 0.35 * exp(-dot(gq, gq) * 6.0);
    }


    if (uEngine > 0.001) {
        float ax = abs(suv.x);
        float chamber = (1.0 - smoothstep(0.33, 0.37, ax))
                      * smoothstep(-0.42, -0.36, suv.y)
                      * (1.0 - smoothstep(0.36, 0.42, suv.y));
        float metal = 1.0 - chamber;
        float tex = fbm(suv * vec2(7.0, 14.0));
        vec3 steel = vec3(0.026, 0.028, 0.034) + vec3(0.10, 0.11, 0.12) * tex * 0.5;
        float rimIn = exp(-pow((ax - 0.345) * 40.0, 2.0)) * step(abs(suv.y), 0.40)
                    + exp(-pow((suv.y - 0.385) * 40.0, 2.0)) * step(ax, 0.345)
                    + exp(-pow((suv.y + 0.385) * 40.0, 2.0)) * step(ax, 0.345);
        steel += vec3(1.0, 0.60, 0.22) * rimIn * (1.0 - chamber) * 0.28;
        float plug = exp(-pow(length((suv - vec2(0.0, 0.47)) * vec2(14.0, 20.0)), 2.0));
        steel += vec3(0.60, 0.62, 0.66) * plug * 0.45;
        col = mix(col, steel, metal * 0.97 * uEngine);
    }


    if (uFurnace > 0.001) {
        float fr = length(suv);
        float ring1 = uFurnaceR + 0.30;
        float iron = smoothstep(uFurnaceR - 0.008, uFurnaceR + 0.008, fr)
                   * (1.0 - smoothstep(ring1 - 0.02, ring1 + 0.06, fr));
        float ang = atan(suv.y, suv.x);
        float rivet = pow(0.5 + 0.5 * cos(ang * 14.0), 24.0)
                    * smoothstep(uFurnaceR + 0.10, uFurnaceR + 0.16, fr)
                    * (1.0 - smoothstep(uFurnaceR + 0.20, uFurnaceR + 0.26, fr));
        float tex = fbm(suv * 9.0);
        vec3 ironCol = vec3(0.028, 0.026, 0.028) + vec3(0.05, 0.045, 0.04) * tex;
        ironCol += vec3(0.90, 0.55, 0.20) * rivet * 0.10;
        col = mix(col, ironCol, iron * 0.98 * uFurnace);
        col += vec3(1.0, 0.62, 0.22) * exp(-pow((fr - uFurnaceR) * 26.0, 2.0)) * 0.50 * uFurnace;
        float steam = fbm(suv * 3.0 + vec2(uTime * 0.10, -uTime * 0.06));
        float steamMask = smoothstep(ring1 + 0.05, ring1 + 0.35, fr) * (1.0 - smoothstep(1.0, 1.45, fr));
        col += vec3(0.55, 0.58, 0.62) * steam * steamMask * 0.05 * uFurnace;
    }


    if (uShock > 0.001) {
        vec2 sp = suv - uShockApex;
        float sr = length(sp);
        float sang = abs(atan(sp.y, sp.x));
        float sector = 1.0 - smoothstep(0.25, 0.75, sang);
        float arcs = pow(0.5 + 0.5 * sin(sr * 34.0 - uTime * 7.0), 6.0) * exp(-sr * 1.9);
        col += vec3(0.75, 0.82, 0.95) * arcs * sector * 0.10 * uShock;
        float cone = exp(-pow((sang - 0.42 - sr * 0.10) * 14.0, 2.0)) * exp(-sr * 1.4);
        col += vec3(0.80, 0.85, 1.0) * cone * 0.06 * uShock;
    }


    if (uBody > 0.001) {
        vec2 bp = suv - uBodyPos;
        float rad = mix(0.036, 0.028, uBodyMorph);
        float hl2 = uBodyLen * 0.5 - rad;
        float taper = mix(0.65, 0.92, uBodyMorph);
        float rr = rad * (1.0 - taper * smoothstep(hl2 - rad, hl2 + rad * 2.2, bp.x));
        float d = length(vec2(max(abs(bp.x) - hl2, 0.0), bp.y)) - rr;
        float inside = 1.0 - smoothstep(-0.004, 0.004, d);
        if (inside > 0.0) {
            float brush = fbm(vec2(bp.x * 40.0, bp.y * 90.0));
            vec3 metalC = mix(vec3(0.16, 0.12, 0.07), vec3(0.55, 0.42, 0.22),
                              smoothstep(-rad, rad, bp.y) * 0.6 + brush * 0.25);
            float moltenBand = fbm(vec2(bp.x * 10.0 - uTime * 2.0, bp.y * 6.0));
            vec3 melt = vec3(1.0, 0.72, 0.30) * (0.6 + 0.8 * moltenBand);
            metalC = mix(metalC, melt, uBodyMolten * (0.35 + 0.65 * smoothstep(0.2, 0.9, moltenBand)));
            metalC += vec3(1.0, 0.60, 0.20) * smoothstep(-hl2 * 0.4, -hl2, bp.x) * 0.5;
            col = mix(col, metalC, inside * 0.95 * uBody);
        }
    }


    if (uCoreGlow > 0.001) {
        vec2 cq = suv - uCorePos;
        col += vec3(1.0, 0.72, 0.30) * uCoreGlow * 0.55 * exp(-dot(cq, cq) * 4.5);
    }
    if (uFlash > 0.001) {
        vec2 fq = suv - uFlashPos;
        col += vec3(1.0, 0.96, 0.85) * uFlash * exp(-dot(fq, fq) * 3.5);
    }
    col *= 1.0 - uVig * smoothstep(0.55, 1.35, length(suv * vec2(0.72, 1.0)));

    fragColor = vec4(col, 1.0);
}