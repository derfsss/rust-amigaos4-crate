#version 450
// An animated plasma: four sine waves over the window, coloured through a
// cosine palette. Time and the window's aspect ratio arrive as push
// constants (two 32-bit floats).
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 colour;

layout(push_constant) uniform Push {
    float time;
    float aspect;
} pc;

void main()
{
    vec2 p = (uv - 0.5) * vec2(pc.aspect, 1.0) * 6.0;
    float t = pc.time;
    float v = sin(p.x + t)
            + sin(p.y * 1.3 - t * 0.7)
            + sin((p.x + p.y) * 0.7 + t * 1.1)
            + sin(length(p) * 1.5 - t * 1.6);
    vec3 c = 0.5 + 0.5 * cos(v * 1.2 + vec3(0.0, 2.1, 4.2));
    colour = vec4(c, 1.0);
}
