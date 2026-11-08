#version 450
// One triangle that covers the whole window: no vertex buffer, the
// corners come from the vertex index (0, 1, 2 -> (0,0), (2,0), (0,2)).
layout(location = 0) out vec2 uv;

void main()
{
    vec2 p = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
    uv = p;
    gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}
