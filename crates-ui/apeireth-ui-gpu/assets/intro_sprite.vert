#version 300 es
precision highp float;
layout(location=0) in vec4 aSeed;
uniform float uAspect;
uniform float uDpr;
uniform float uTime;
uniform vec2  uP0;
uniform vec2  uVel;
uniform vec2  uAcc;
uniform vec2  uDir;
uniform float uSpread;
uniform float uSpawnR;
uniform float uSpeed;
uniform float uSpeedVar;
uniform float uInherit;
uniform float uLife;
uniform float uLifeVar;
uniform float uSize0;
uniform float uSize1;
uniform float uSizeVar;
uniform vec2  uGrav;
uniform float uDrag;
uniform float uTurb;
uniform float uTurbFreq;
uniform float uTurbTime;
uniform float uCount01;
uniform float uBurst;
uniform float uBurstSpan;
uniform float uRadial01;
uniform float uRadialSpeed;
uniform float uRadialDrag;
uniform float uRadialLife;
uniform float uSoft01;
uniform float uSoftSize;
uniform float uSoftGain;
uniform float uStreak;
uniform float uTwinkle;
uniform float uGain;
uniform float uEmber01;
uniform float uAlpha;
uniform vec3  uColHot;
uniform vec3  uColMid;
uniform vec3  uColTail;
out vec3 vCol;
out float vAlpha;
out float vStreak;
out vec2 vAxis;
out float vSoft;

void main(){
    float r1 = aSeed.x;
    float r2 = aSeed.y;
    float r3 = aSeed.z;
    float r4 = aSeed.w;
    float gate = fract(r1 * 7.31 + r2 * 3.17 + r4 * 1.93);
    bool live = gate < uCount01;
    bool ember = r4 > 1.0 - uEmber01;
    bool radial = r3 < uRadial01;
    bool soft = (!ember) && (!radial) && (fract(r3 * 5.7) < uSoft01);

    float life = (radial ? uRadialLife : uLife)
               * (1.0 + (r1 - 0.5) * 2.0 * uLifeVar)
               * (ember ? 1.7 : 1.0);
    life = max(life, 0.05);
    float birth = uBurst > 0.5 ? r2 * (radial ? min(uBurstSpan, 0.18) : uBurstSpan) : 0.0;
    float ageSec = uBurst > 0.5 ? (uTime - birth) : fract(uTime / life + r2) * life;

    if (!live || ageSec < 0.0 || ageSec > life) {
        gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
        gl_PointSize = 0.0;
        vCol = vec3(0.0);
        vAlpha = 0.0;
        vStreak = 0.0;
        vAxis = vec2(1.0, 0.0);
        vSoft = 0.0;
        return;
    }
    float age01 = ageSec / life;


    vec2 org = uP0 - uVel * ageSec + 0.5 * uAcc * ageSec * ageSec;
    vec2 orgVel = uVel - uAcc * ageSec;
    float ja = r2 * 6.28318 + r4 * 2.1;
    org += vec2(cos(ja), sin(ja)) * uSpawnR * (0.3 + 0.7 * r1);

    float ang;
    float spd;
    float drag;
    if (radial) {
        ang = r2 * 6.28318 + r1 * 0.7;
        spd = uRadialSpeed * (0.75 + 0.5 * r4);
        drag = uRadialDrag;
    } else {
        ang = atan(uDir.y, uDir.x) + (r3 - 0.5) * 2.0 * uSpread + (ember ? (r2 - 0.5) * 0.9 : 0.0);
        spd = uSpeed * (1.0 + (r1 - 0.5) * 2.0 * uSpeedVar) * (ember ? 1.9 : 1.0);
        drag = uDrag;
    }
    vec2 v0 = vec2(cos(ang), sin(ang)) * spd + orgVel * uInherit;


    float e = exp(-drag * ageSec);
    vec2 disp;
    if (drag > 0.001) {
        float k = (1.0 - e) / drag;
        disp = v0 * k + uGrav * (ageSec - k) / drag;
    } else {
        disp = v0 * ageSec + 0.5 * uGrav * ageSec * ageSec;
    }


    vec2 tp = org * uTurbFreq + vec2(r4 * 17.0, r1 * 13.0);
    float tn = uTime * uTurbTime;
    vec2 turb = vec2(
        sin(tp.y * 1.7 + tn * 1.9 + sin(tp.x * 1.3 + tn) * 1.2),
        cos(tp.x * 1.9 - tn * 1.7 + sin(tp.y * 1.1 - tn * 1.3) * 1.2)
    ) * (uTurb * age01 * (0.4 + 0.6 * r2));

    vec2 pos = org + disp + turb;
    gl_Position = vec4(pos.x / uAspect, pos.y, 0.0, 1.0);

    float size = mix(uSize0, uSize1, age01) * (1.0 + (r4 - 0.5) * 2.0 * uSizeVar) * (ember ? 0.45 : 1.0);
    if (soft) size = uSoftSize * (0.65 + 0.7 * r1);
    gl_PointSize = size * uDpr;

    float env = smoothstep(0.0, 0.07, age01) * pow(1.0 - age01, 1.35);
    float tw = 1.0 - uTwinkle * (0.5 + 0.5 * sin(uTime * (18.0 + 26.0 * r3) + r4 * 40.0));
    vAlpha = env * tw * uAlpha * uGain * (soft ? uSoftGain : 1.0);

    vec3 c = mix(uColHot, uColMid, smoothstep(0.0, 0.30, age01));
    c = mix(c, uColTail, smoothstep(0.30, 1.0, age01));
    vCol = c;


    vec2 vcur = v0 * e + uGrav * ageSec + orgVel * (1.0 - uInherit);
    if (dot(vcur, vcur) < 0.0025) vcur = uVel;
    vAxis = normalize(vcur + vec2(1e-5, 0.0));
    vStreak = uStreak * (ember ? 1.3 : 1.0) * (radial ? 1.2 : 1.0);
    vSoft = soft ? 1.0 : 0.0;
}