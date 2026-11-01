#version 300 es
precision highp float;
in vec3 vCol;
in float vAlpha;
in float vStreak;
in vec2 vAxis;
in float vSoft;
out vec4 fragColor;
void main(){
    vec2 d = gl_PointCoord * 2.0 - 1.0;
    vec2 t = vAxis;
    vec2 n = vec2(-t.y, t.x);
    vec2 p = vec2(dot(d, t), dot(d, n));
    vec2 q = vec2(p.x / (1.0 + vStreak * 7.0), p.y);
    float r2 = dot(q, q);

    float thin = 1.0 + vStreak * 3.0;
    float m = vSoft > 0.5
        ? exp(-r2 * 1.8)
        : exp(-r2 * 8.0 * thin) + exp(-r2 * 2.2) * 0.30 / thin;


    float win = (1.0 - smoothstep(0.80, 1.0, abs(p.x)))
              * (1.0 - smoothstep(0.60, 1.0, abs(p.y)));
    fragColor = vec4(vCol * (m * vAlpha * win), 1.0);
}